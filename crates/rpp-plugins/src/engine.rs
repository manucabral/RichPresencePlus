//! Loading a presence script and running its `read` and `activity` functions
//! inside a QuickJS context with memory, stack and time limits.

use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use rpp_core::{Activity, ActivityError};
use rquickjs::context::intrinsic;
use rquickjs::{Context, Ctx, Exception, Function, Runtime, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use crate::console::Console;
use crate::manifest::Manifest;
use crate::permissions::{Host, Permissions, Request};
use crate::setting::resolve as resolve_settings;

/// Memory a script may use.
const MEMORY_LIMIT: usize = 16 * 1024 * 1024;
/// Native stack a script may use.
const STACK_LIMIT: usize = 512 * 1024;
/// Time allowed to evaluate the script file.
const LOAD_BUDGET: Duration = Duration::from_secs(1);
/// Time allowed for one `read` plus `activity` round.
const TICK_BUDGET: Duration = Duration::from_secs(2);
/// Largest snapshot a script may produce.
pub const MAX_SNAPSHOT: usize = 64 * 1024;

/// Why a script could not produce an activity.
#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    /// The script file failed to evaluate.
    #[error("script failed to load: {0}")]
    Load(String),
    /// The script does not define a required function.
    #[error("script must define the function {0}")]
    Missing(&'static str),
    /// The script threw an error.
    #[error("script error: {0}")]
    Script(String),
    /// The script ran longer than its budget.
    #[error("script exceeded its time budget")]
    Timeout,
    /// The script returned something that is not plain data of the right shape.
    #[error("script returned invalid data: {0}")]
    Output(String),
    /// The snapshot is larger than [`MAX_SNAPSHOT`].
    #[error("snapshot exceeds {MAX_SNAPSHOT} bytes")]
    SnapshotTooLarge,
    /// The activity breaks Discord's rules.
    #[error(transparent)]
    Activity(#[from] ActivityError),
}

/// What a script says about its activity besides the activity itself.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Meta {
    /// The source is the window the user is looking at.
    pub foreground: bool,
    /// The media is paused.
    pub paused: bool,
    /// Application name, for the privacy exclusions.
    pub application: String,
    /// Extra hostnames involved, for the privacy exclusions.
    pub domains: Vec<String>,
}

/// Result of one poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tick {
    /// What to show, or nothing.
    pub activity: Option<Activity>,
    /// Extra facts used to rank and filter the activity.
    pub meta: Meta,
    /// What `read` returned, as JSON text; `None` when it returned nothing.
    pub snapshot: Option<String>,
}

/// The time budget of the call in progress.
struct Clock {
    origin: Instant,
    /// Deadline in milliseconds since `origin`; 0 while no call is running.
    deadline: AtomicU64,
    timed_out: AtomicBool,
}

impl Clock {
    fn elapsed_ms(&self) -> u64 {
        u64::try_from(self.origin.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Whether the running call is past its deadline; remembers it if so.
    fn expired(&self) -> bool {
        let deadline = self.deadline.load(Ordering::Relaxed);
        let over = deadline != 0 && self.elapsed_ms() >= deadline;
        if over {
            self.timed_out.store(true, Ordering::Relaxed);
        }
        over
    }
}

/// A loaded presence ready to be polled.
pub struct Plugin {
    manifest: Manifest,
    // The runtime must outlive the context, so it is kept alongside it.
    _runtime: Runtime,
    context: Context,
    clock: Arc<Clock>,
    console: Arc<Mutex<Console>>,
}

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugin")
            .field("name", &self.manifest.name)
            .finish()
    }
}

/// JavaScript that builds the frozen `host` object from the granted permissions
/// and removes the raw bridge from the global scope.
/// Names the prelude takes before the script runs. A script cannot declare its own
/// function or variable with one of them.
pub const RESERVED: [&str; 12] = [
    "host",
    "console",
    "t",
    "cut",
    "parseUrl",
    "bestPage",
    "parseTime",
    "progress",
    "rotate",
    "format",
    "icon",
    "playback",
];

/// The engine says only "cannot define variable", which reads like a bug of the
/// script; this names the helper and what to do.
fn explain_load(message: String) -> String {
    RESERVED
        .iter()
        .find(|name| message.contains(&format!("cannot define variable '{name}'")))
        .map_or(message, |name| {
            format!(
                "{name} is a helper of the application; give your function or variable another name"
            )
        })
}

fn prelude(permissions: &Permissions) -> String {
    let mut members: Vec<String> = Vec::new();
    if !permissions.http.is_empty() {
        members.push(r#"http: Object.freeze({ get: (url) => call("http.get", { url }) })"#.into());
    }
    if permissions.process || !permissions.args.is_empty() {
        let mut process: Vec<&str> = Vec::new();
        if permissions.process {
            process.push(r#"list: () => call("process.list")"#);
        }
        if !permissions.args.is_empty() {
            process.push(r#"args: (name) => call("process.args", { name })"#);
        }
        members.push(format!(
            "process: Object.freeze({{ {} }})",
            process.join(", ")
        ));
    }
    if permissions.window {
        members.push(
            r#"window: Object.freeze({ foreground: () => call("window.foreground") })"#.into(),
        );
    }
    if permissions.media {
        members.push(r#"media: Object.freeze({ current: () => call("media.current") })"#.into());
    }
    if !permissions.files.is_empty() {
        // `roots` lets a script find the folders it was granted after expansion.
        let roots = serde_json::to_string(&permissions.files).unwrap_or_else(|_| "[]".into());
        members.push(format!(
            r#"files: Object.freeze({{ roots: Object.freeze({roots}), tail: (path, lines) => call("files.tail", {{ path, lines }}), latest: (dir, suffix) => call("files.latest", {{ dir, suffix }}) }})"#
        ));
    }
    if !permissions.query.is_empty() {
        members.push(
            r#"query: (protocol, address, port) => call("query", { protocol, host: address, port })"#
                .into(),
        );
    }
    include_str!("prelude.js").replace("/*MEMBERS*/", &members.join(", "))
}

/// Turn the wire form used by the bridge function into a typed request.
fn decode(kind: &str, args: &Json) -> Result<Request, String> {
    let text = |key: &str| {
        args.get(key)
            .and_then(Json::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("missing {key}"))
    };
    Ok(match kind {
        "http.get" => Request::HttpGet { url: text("url")? },
        "process.list" => Request::ProcessList,
        // The options come from the manifest; see `Permissions::complete`.
        "process.args" => Request::ProcessArgs {
            name: text("name")?,
            flags: Vec::new(),
        },
        "window.foreground" => Request::ForegroundWindow,
        "media.current" => Request::MediaSession,
        "files.latest" => Request::LatestFile {
            dir: text("dir")?,
            suffix: text("suffix")?,
        },
        "files.tail" => Request::TailFile {
            path: text("path")?,
            lines: args
                .get("lines")
                .and_then(Json::as_u64)
                .and_then(|n| usize::try_from(n).ok())
                .ok_or("missing lines")?,
        },
        "query" => Request::ServerQuery {
            protocol: text("protocol")?,
            host: text("host")?,
            port: args
                .get("port")
                .and_then(Json::as_u64)
                .and_then(|n| u16::try_from(n).ok())
                .ok_or("port must be 1 to 65535")?,
        },
        other => return Err(format!("unknown host call {other}")),
    })
}

/// Message carried by the exception pending in `ctx`.
fn describe(ctx: &Ctx<'_>, error: &rquickjs::Error) -> String {
    if error.is_exception() {
        let caught = ctx.catch();
        if let Some(message) = caught.as_exception().and_then(|e| e.message()) {
            return message;
        }
        return "unknown exception".into();
    }
    error.to_string()
}

impl Plugin {
    /// Evaluates a script with the limits applied and the permissions granted.
    ///
    /// Args:
    ///     host: What answers the capabilities the script may request.
    ///
    /// Raises:
    ///     PluginError: The script fails to evaluate or exceeds its time budget.
    pub fn load(manifest: Manifest, source: &str, host: Rc<dyn Host>) -> Result<Self, PluginError> {
        let runtime = Runtime::new().map_err(|e| PluginError::Load(e.to_string()))?;
        runtime.set_memory_limit(MEMORY_LIMIT);
        runtime.set_max_stack_size(STACK_LIMIT);

        let clock = Arc::new(Clock {
            origin: Instant::now(),
            deadline: AtomicU64::new(0),
            timed_out: AtomicBool::new(false),
        });
        {
            let clock = Arc::clone(&clock);
            runtime.set_interrupt_handler(Some(Box::new(move || clock.expired())));
        }

        // `Eval` is needed to load the file; the prelude then disables dynamic
        // code generation before any plugin code runs.
        let context = Context::custom::<(
            intrinsic::Eval,
            intrinsic::Date,
            intrinsic::Json,
            intrinsic::RegExp,
            intrinsic::RegExpCompiler,
            intrinsic::MapSet,
            intrinsic::TypedArrays,
            intrinsic::Proxy,
            intrinsic::Promise,
        )>(&runtime)
        .map_err(|e| PluginError::Load(e.to_string()))?;

        let permissions = manifest.permissions.clone();
        let console = Arc::new(Mutex::new(Console::default()));
        let plugin = Self {
            manifest,
            _runtime: runtime,
            context,
            clock,
            console: Arc::clone(&console),
        };
        plugin.arm(LOAD_BUDGET);
        let loaded = plugin.context.with(|ctx| {
            let bridge = Function::new(ctx.clone(), {
                let permissions = permissions.clone();
                move |ctx: Ctx<'_>, kind: String, args: String| -> rquickjs::Result<String> {
                    let fail = |message: String| Exception::throw_message(&ctx, &message);
                    let parsed: Json =
                        serde_json::from_str(&args).map_err(|e| fail(e.to_string()))?;
                    // Printing needs no permission: it only reaches the developer tools.
                    if kind == "console" {
                        let level = parsed.get("level").and_then(Json::as_str).unwrap_or("log");
                        let text = parsed
                            .get("text")
                            .and_then(Json::as_str)
                            .unwrap_or_default();
                        console
                            .lock()
                            .unwrap_or_else(PoisonError::into_inner)
                            .push(level, text);
                        return Ok("null".into());
                    }
                    let request = permissions.complete(decode(&kind, &parsed).map_err(fail)?);
                    permissions
                        .check(&request)
                        .map_err(|d| fail(d.to_string()))?;
                    let answer = host.call(request).map_err(fail)?;
                    Ok(answer.to_string())
                }
            })
            .map_err(|e| describe(&ctx, &e))?;
            ctx.globals()
                .set("__host", bridge)
                .map_err(|e| describe(&ctx, &e))?;
            ctx.eval::<(), _>(prelude(&permissions))
                .map_err(|e| describe(&ctx, &e))?;
            ctx.eval::<(), _>(source).map_err(|e| describe(&ctx, &e))
        });
        let timed_out = plugin.disarm();
        match loaded {
            _ if timed_out => Err(PluginError::Timeout),
            Err(message) => Err(PluginError::Load(explain_load(message))),
            Ok(()) => Ok(plugin),
        }
    }

    /// Sets the texts that `t(key, values)` returns, in the user's language.
    ///
    /// Raises:
    ///     PluginError: The table cannot be handed to the script.
    pub fn set_strings(&mut self, strings: &BTreeMap<String, String>) -> Result<(), PluginError> {
        let json = serde_json::to_string(strings).map_err(|e| PluginError::Load(e.to_string()))?;
        self.context
            .with(|ctx| {
                let table = ctx.json_parse(json).map_err(|e| describe(&ctx, &e))?;
                ctx.globals()
                    .set("__strings", table)
                    .map_err(|e| describe(&ctx, &e))
            })
            .map_err(PluginError::Load)
    }

    /// Returns the manifest this plugin was loaded with.
    pub fn manifest(&self) -> &Manifest {
        &self.manifest
    }

    fn arm(&self, budget: Duration) {
        let clock = &self.clock;
        clock.timed_out.store(false, Ordering::Relaxed);
        let budget = u64::try_from(budget.as_millis()).unwrap_or(u64::MAX);
        // At least 1, because 0 means "no deadline".
        let deadline = clock.elapsed_ms().saturating_add(budget).max(1);
        clock.deadline.store(deadline, Ordering::Relaxed);
    }

    /// Stop the clock and report whether the budget was exceeded.
    fn disarm(&self) -> bool {
        self.clock.deadline.store(0, Ordering::Relaxed);
        self.clock.timed_out.swap(false, Ordering::Relaxed)
    }

    /// Runs `read`, then `activity` and, when defined, `meta`.
    ///
    /// Args:
    ///     context: Plain data prepared by the host, such as the current time and
    ///         the user's settings. It is the argument of `read`.
    ///
    /// Returns:
    ///     The validated activity (empty when the script has nothing to show) and
    ///     the extra facts used to rank and filter it.
    ///
    /// Raises:
    ///     PluginError: A function is missing or throws, the time budget is
    ///         exceeded, the output has the wrong shape, or the activity breaks
    ///         Discord's rules.
    pub fn poll(&mut self, context: &Json) -> Result<Tick, PluginError> {
        let input = self.with_options(context).to_string();
        self.call(|ctx| run(ctx, &input))
    }

    /// Shows what `read` returned earlier, as [`Plugin::poll`] would, without
    /// calling `read`.
    ///
    /// Raises:
    ///     PluginError: The snapshot is too large, or as `poll`.
    pub fn replay(&mut self, snapshot: &Json) -> Result<Tick, PluginError> {
        let input = snapshot.to_string();
        if input.len() > MAX_SNAPSHOT {
            return Err(PluginError::SnapshotTooLarge);
        }
        self.call(|ctx| replay(ctx, &input))
    }

    /// Runs a call into the script within the time budget and reads its result.
    fn call(
        &mut self,
        work: impl FnOnce(&Ctx<'_>) -> Result<Output, PluginError>,
    ) -> Result<Tick, PluginError> {
        self.lock_console().begin_poll();
        self.arm(TICK_BUDGET);
        let result = self.context.with(|ctx| work(&ctx));
        if self.disarm() {
            return Err(PluginError::Timeout);
        }
        let output = result?;
        let activity = match output.activity {
            None => None,
            Some(text) => {
                let parsed: Activity =
                    serde_json::from_str(&text).map_err(|e| PluginError::Output(e.to_string()))?;
                Some(parsed.validated()?)
            }
        };
        let meta = match output.meta {
            None => Meta::default(),
            Some(text) => {
                serde_json::from_str(&text).map_err(|e| PluginError::Output(e.to_string()))?
            }
        };
        Ok(Tick {
            activity,
            meta,
            snapshot: output.snapshot,
        })
    }

    /// Hands over what the script printed since the last call.
    pub fn take_console(&mut self) -> Vec<String> {
        self.lock_console().take()
    }

    fn lock_console(&self) -> std::sync::MutexGuard<'_, Console> {
        self.console.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The context with `options` holding every declared option.
    fn with_options(&self, context: &Json) -> Json {
        let given: BTreeMap<String, Json> = context
            .get("options")
            .and_then(Json::as_object)
            .map(|map| map.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default();
        let options = Json::Object(resolve_settings(&self.manifest.settings, &given));
        let mut context = match context {
            Json::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        context.insert("options".into(), options);
        Json::Object(context)
    }

    /// Like `poll`, returning only the activity; `None` when there is nothing
    /// to show.
    pub fn tick(&mut self, context: &Json) -> Result<Option<Activity>, PluginError> {
        Ok(self.poll(context)?.activity)
    }
}

/// What the three script functions returned, as JSON text.
#[derive(Default)]
struct Output {
    snapshot: Option<String>,
    activity: Option<String>,
    meta: Option<String>,
}

/// Call the script functions and return their results as JSON text.
fn run(ctx: &Ctx<'_>, input: &str) -> Result<Output, PluginError> {
    let script = |e: rquickjs::Error| PluginError::Script(describe(ctx, &e));
    let globals = ctx.globals();
    let read: Function = globals
        .get("read")
        .map_err(|_| PluginError::Missing("read"))?;
    // Checked before `read` runs, whose work is wasted without it.
    globals
        .get::<_, Function>("activity")
        .map_err(|_| PluginError::Missing("activity"))?;

    let argument = ctx.json_parse(input).map_err(script)?;
    let snapshot: Value = read.call((argument,)).map_err(script)?;
    reject_promise(&snapshot, "read")?;
    // A script that has nothing to read says so by returning nothing.
    if snapshot.is_null() || snapshot.is_undefined() {
        return Ok(Output::default());
    }
    let encoded = stringify(ctx, &snapshot)?.unwrap_or_else(|| "null".into());
    if encoded.len() > MAX_SNAPSHOT {
        return Err(PluginError::SnapshotTooLarge);
    }
    show(ctx, snapshot, encoded)
}

/// Replays a snapshot given as JSON text.
fn replay(ctx: &Ctx<'_>, input: &str) -> Result<Output, PluginError> {
    let snapshot = ctx
        .json_parse(input)
        .map_err(|e| PluginError::Script(describe(ctx, &e)))?;
    show(ctx, snapshot, input.to_owned())
}

/// Calls `activity` and `meta` with a snapshot.
fn show<'js>(ctx: &Ctx<'js>, snapshot: Value<'js>, encoded: String) -> Result<Output, PluginError> {
    let script = |e: rquickjs::Error| PluginError::Script(describe(ctx, &e));
    let globals = ctx.globals();
    let activity: Function = globals
        .get("activity")
        .map_err(|_| PluginError::Missing("activity"))?;
    let shown: Value = activity.call((snapshot.clone(),)).map_err(script)?;
    reject_promise(&shown, "activity")?;
    let shown = if shown.is_null() || shown.is_undefined() {
        None
    } else {
        stringify(ctx, &shown)?.filter(|t| t != "null")
    };

    let mut meta = None;
    if let Ok(function) = globals.get::<_, Function>("meta") {
        let value: Value = function.call((snapshot,)).map_err(script)?;
        reject_promise(&value, "meta")?;
        meta = stringify(ctx, &value)?.filter(|t| t != "null");
    }
    Ok(Output {
        snapshot: Some(encoded),
        activity: shown,
        meta,
    })
}

fn reject_promise(value: &Value<'_>, function: &str) -> Result<(), PluginError> {
    if value.is_promise() {
        return Err(PluginError::Output(format!(
            "{function} must not return a promise"
        )));
    }
    Ok(())
}

fn stringify<'js>(ctx: &Ctx<'js>, value: &Value<'js>) -> Result<Option<String>, PluginError> {
    let text = ctx
        .json_stringify(value.clone())
        .map_err(|e| PluginError::Output(describe(ctx, &e)))?;
    text.map(|t| {
        t.to_string()
            .map_err(|e| PluginError::Output(e.to_string()))
    })
    .transpose()
}

#[cfg(test)]
#[path = "engine_tests.rs"]
mod tests;
