//! One thread per presence script.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rpp_core::{ActivityKind, Backoff, Candidate, Kind};
use rpp_plugins::{Host, Manifest, Permissions, Plugin, Tick};
use serde_json::json;
use tokio::sync::mpsc::UnboundedSender;

use crate::message::Message;
use crate::{Inputs, Presence};

/// A source that stays silent this long (or three intervals) is dropped.
const MIN_SILENCE: Duration = Duration::from_secs(30);
/// Domains taken from one reading of a script.
const MAX_DOMAINS: usize = 32;
/// Longest domain name there can be.
const MAX_DOMAIN: usize = 253;
/// Longest wait between polls of a script that keeps failing.
const MAX_FAILURE_WAIT: Duration = Duration::from_secs(60);
/// Longest wait for a stopped script to finish the read in progress.
pub const STOP_WAIT: Duration = Duration::from_secs(2);

/// How long a presence with no pages keeps its script loaded.
#[cfg(not(test))]
const IDLE_UNLOAD: Duration = Duration::from_secs(60);
#[cfg(test)]
const IDLE_UNLOAD: Duration = Duration::from_millis(150);

/// How long open pages must give nothing before the user hears about it. Pages
/// that hold nothing to read are normal for a moment, while one loads.
#[cfg(not(test))]
const BLANK_AFTER: Duration = Duration::from_secs(120);
#[cfg(test)]
const BLANK_AFTER: Duration = Duration::from_millis(100);

/// Tells whether every open page came back with every value empty.
fn pages_blank(pages: &serde_json::Value) -> bool {
    let Some(list) = pages.as_array() else {
        return false;
    };
    !list.is_empty()
        && list.iter().all(|page| {
            page.get("values")
                .and_then(serde_json::Value::as_object)
                .is_some_and(|values| {
                    !values.is_empty() && values.values().all(serde_json::Value::is_null)
                })
        })
}

/// Follows how long a presence's pages have been blank.
#[derive(Debug, Default)]
struct BlankWatch {
    since: Option<Instant>,
    reported: bool,
}

impl BlankWatch {
    /// Takes one reading and returns the new state when it should be
    /// reported: true once pages stayed blank long enough, false when they
    /// recover.
    fn observe(&mut self, blank: bool, now: Instant) -> Option<bool> {
        if !blank {
            self.since = None;
            let recovered = self.reported;
            self.reported = false;
            return recovered.then_some(false);
        }
        let since = *self.since.get_or_insert(now);
        if !self.reported && now.duration_since(since) >= BLANK_AFTER {
            self.reported = true;
            return Some(true);
        }
        None
    }
}

/// Tells whether a presence can have nothing to show without a page: it reads
/// pages and has no other way to learn anything.
fn needs_pages(manifest: &Manifest) -> bool {
    manifest.page.is_some() && manifest.permissions == Permissions::default()
}

/// Group used for ranking when the manifest does not name one.
fn infer_kind(kind: ActivityKind) -> Kind {
    match kind {
        ActivityKind::Playing | ActivityKind::Competing => Kind::Game,
        ActivityKind::Listening => Kind::Music,
        ActivityKind::Watching => Kind::Video,
    }
}

/// Turns the result of a poll into what the policy judges; `None` when the
/// script has nothing to show. The expiry counts from `now`.
pub fn candidate(manifest: &Manifest, tick: Tick, now: Instant) -> Option<Candidate> {
    let activity = tick.activity?;
    let silence = Duration::from_secs(u64::from(manifest.interval) * 3).max(MIN_SILENCE);
    let mut domains = manifest.domains.clone();
    // A script can return any list; the privacy check runs on it at every pass.
    let reported = tick.meta.domains.into_iter();
    domains.extend(
        reported
            .filter(|domain| domain.len() <= MAX_DOMAIN)
            .take(MAX_DOMAINS),
    );
    domains.sort();
    domains.dedup();
    Some(Candidate {
        source: manifest.name.clone(),
        label: manifest.name.clone(),
        client_id: manifest.client_id.clone(),
        kind: manifest.kind.unwrap_or_else(|| infer_kind(activity.kind)),
        activity,
        foreground: tick.meta.foreground,
        paused: tick.meta.paused,
        hide_when_paused: manifest.hide_when_paused,
        domains,
        application: tick.meta.application,
        expires_at: Some(now + silence),
    })
}

/// A running plugin thread.
#[derive(Debug)]
pub struct Runner {
    name: String,
    stop: Option<Sender<()>>,
    /// Disconnects when the thread is done, so a stop can wait without polling.
    done: Option<Receiver<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Runner {
    /// Loads a script on a new thread and polls it every `manifest.interval`
    /// seconds.
    ///
    /// The handle stops the thread when dropped.
    ///
    /// Args:
    ///     presence: Its `options` are ignored: the script reads them through
    ///         `inputs`.
    ///     inputs: Where `context.pages` and `context.options` come from.
    pub fn spawn<H>(
        presence: Presence,
        host: H,
        inputs: Inputs,
        updates: UnboundedSender<Message>,
    ) -> Self
    where
        H: Host + Send + 'static,
    {
        let (stop, signal) = mpsc::channel();
        let (finished, done) = mpsc::channel::<()>();
        let name = presence.manifest.name.clone();
        let thread = thread::Builder::new()
            .name(format!("presence-{name}"))
            .spawn(move || {
                let _finished = finished;
                let source = presence.manifest.name.clone();
                // A bug in a host source or in the engine must not end the
                // presence in silence: the user sees it failed and can retry.
                let ran = catch_unwind(AssertUnwindSafe(|| {
                    run(presence, host, &inputs, &updates, &signal);
                }));
                let stopped = matches!(signal.try_recv(), Err(TryRecvError::Disconnected));
                if ran.is_err() && !stopped {
                    let _ = updates.send(Message::Failed {
                        source,
                        error:
                            "The presence stopped unexpectedly. Turn it off and on to try again."
                                .into(),
                    });
                }
            })
            .ok();
        Self {
            name,
            stop: Some(stop),
            done: Some(done),
            thread,
        }
    }

    /// Tells whether the script's thread is still running.
    pub fn is_alive(&self) -> bool {
        self.thread
            .as_ref()
            .is_some_and(|thread| !thread.is_finished())
    }

    /// Asks the script to read again now instead of at its next interval,
    /// for example because its options changed.
    pub fn wake(&self) {
        if let Some(stop) = &self.stop {
            // A message means "read now"; the channel closing means "stop".
            let _ = stop.send(());
        }
    }

    /// Asks the script to stop after the read in progress, without waiting.
    pub fn request_stop(&mut self) {
        self.stop.take();
    }

    /// Stops the script and waits up to [`STOP_WAIT`] for its thread to end.
    pub fn stop(&mut self) {
        self.stop_by(Instant::now() + STOP_WAIT);
    }

    /// Stops the script and waits for its thread until `deadline`.
    pub fn stop_by(&mut self, deadline: Instant) {
        self.stop.take();
        let Some(thread) = self.thread.take() else {
            return;
        };
        let wait = deadline.saturating_duration_since(Instant::now());
        let ended = self.done.take().is_none_or(|done| {
            matches!(done.recv_timeout(wait), Err(RecvTimeoutError::Disconnected))
        });
        if ended {
            let _ = thread.join();
        } else {
            tracing::warn!(presence = %self.name, "a script is slow to stop; it ends when its call returns");
        }
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.stop();
    }
}

fn run<H: Host + 'static>(
    presence: Presence,
    host: H,
    inputs: &Inputs,
    updates: &UnboundedSender<Message>,
    signal: &mpsc::Receiver<()>,
) {
    let Presence {
        manifest,
        source,
        strings,
        ..
    } = presence;
    let name = manifest.name.clone();
    let interval = Duration::from_secs(manifest.interval.into());
    let host: Rc<dyn Host> = Rc::new(host);
    let load = || {
        Plugin::load(manifest.clone(), &source, Rc::clone(&host)).and_then(|mut plugin| {
            plugin.set_strings(&strings)?;
            Ok(plugin)
        })
    };
    // Loaded at the start even if no page is open, so a broken script says so
    // when it is turned on and not when the user reaches its site.
    let mut plugin = match load() {
        Ok(plugin) => Some(plugin),
        Err(error) => {
            let _ = updates.send(Message::Failed {
                source: name,
                error: error.to_string(),
            });
            return;
        }
    };
    let waits_for_pages = needs_pages(&manifest);
    let mut failures = Backoff::new(interval, MAX_FAILURE_WAIT.max(interval));
    let mut offering = true;
    let mut blank = BlankWatch::default();
    loop {
        let pages = (inputs.pages)(&name);
        if let Some(state) = blank.observe(pages_blank(&pages), Instant::now()) {
            let health = Message::Health {
                source: name.clone(),
                blank: state,
            };
            if updates.send(health).is_err() {
                return;
            }
        }
        if waits_for_pages && pages.as_array().is_some_and(Vec::is_empty) {
            if offering {
                offering = false;
                let clear = Message::Clear {
                    source: name.clone(),
                };
                if updates.send(clear).is_err() {
                    return;
                }
            }
            if !doze(signal, &mut plugin) {
                return;
            }
            continue;
        }
        offering = true;
        if plugin.is_none() {
            match load() {
                Ok(loaded) => plugin = Some(loaded),
                Err(error) => {
                    let _ = updates.send(Message::Failed {
                        source: name,
                        error: error.to_string(),
                    });
                    return;
                }
            }
        }
        let Some(active) = plugin.as_mut() else {
            return;
        };
        let unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let context = json!({"now": unix, "options": (inputs.settings)(&name), "pages": pages});
        let (message, wait) = match active.poll(&context) {
            Ok(tick) => {
                failures.success();
                let message = match candidate(&manifest, tick, Instant::now()) {
                    Some(candidate) => Message::Candidate(Box::new(candidate)),
                    None => Message::Clear {
                        source: name.clone(),
                    },
                };
                (message, interval)
            }
            Err(error) => {
                let now = Instant::now();
                failures.failure(now);
                let message = Message::Failed {
                    source: name.clone(),
                    error: error.to_string(),
                };
                (message, failures.remaining(now).max(interval))
            }
        };
        // The stop may have come while the read ran, and the service has
        // withdrawn this presence already: nothing may reach it now. A wake
        // that came meanwhile may carry options this read did not see.
        let woken = match drain(signal) {
            Signal::Stop => return,
            Signal::Woken => true,
            Signal::Quiet => false,
        };
        if updates.send(message).is_err() {
            return;
        }
        if woken {
            continue;
        }
        match signal.recv_timeout(wait) {
            Ok(()) => {
                // Several wakes in a row need one read, not one each.
                if matches!(drain(signal), Signal::Stop) {
                    return;
                }
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
}

/// Sleeps until a wake, with no timers, freeing the script on the way.
fn doze(signal: &mpsc::Receiver<()>, plugin: &mut Option<Plugin>) -> bool {
    if plugin.is_some() {
        match signal.recv_timeout(IDLE_UNLOAD) {
            Ok(()) => return !matches!(drain(signal), Signal::Stop),
            Err(RecvTimeoutError::Disconnected) => return false,
            Err(RecvTimeoutError::Timeout) => *plugin = None,
        }
    }
    signal.recv().is_ok() && !matches!(drain(signal), Signal::Stop)
}

/// What came on the signal channel since it was last read.
enum Signal {
    /// The channel closed: stop.
    Stop,
    /// At least one request to read now.
    Woken,
    /// Nothing.
    Quiet,
}

/// Reads every pending signal without waiting.
fn drain(signal: &mpsc::Receiver<()>) -> Signal {
    let mut woken = false;
    loop {
        match signal.try_recv() {
            Ok(()) => woken = true,
            Err(TryRecvError::Empty) => return if woken { Signal::Woken } else { Signal::Quiet },
            Err(TryRecvError::Disconnected) => return Signal::Stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PageSource, SettingsSource};
    use rpp_core::Activity;
    use rpp_plugins::{Meta, Request};
    use serde_json::Value;
    use std::sync::Arc;
    use tokio::sync::mpsc::unbounded_channel;

    struct Silent;

    impl Host for Silent {
        fn call(&self, _: Request) -> Result<Value, String> {
            Err("no host".into())
        }
    }

    fn manifest(interval: u32) -> Manifest {
        Manifest::parse(&format!(
            r#"{{"name":"Demo","version":"1","client_id":"12345678901234567","interval":{interval},"domains":["demo.example.com"]}}"#
        ))
        .unwrap()
    }

    fn presence(source: &str) -> Presence {
        Presence {
            manifest: manifest(1),
            source: source.into(),
            strings: Default::default(),
            settings: Default::default(),
        }
    }

    fn tick(kind: ActivityKind, meta: Meta) -> Tick {
        Tick {
            activity: Some(Activity {
                details: Some("Hello".into()),
                kind,
                ..Activity::default()
            }),
            meta,
            snapshot: None,
        }
    }

    #[test]
    fn kind_comes_from_the_manifest_or_the_activity_type() {
        let now = Instant::now();
        let from_type = candidate(
            &manifest(5),
            tick(ActivityKind::Watching, Meta::default()),
            now,
        );
        assert_eq!(from_type.unwrap().kind, Kind::Video);
        let mut named = manifest(5);
        named.kind = Some(Kind::Music);
        let from_manifest = candidate(&named, tick(ActivityKind::Watching, Meta::default()), now);
        assert_eq!(from_manifest.unwrap().kind, Kind::Music);
    }

    #[test]
    fn a_script_cannot_hand_over_an_endless_list_of_domains() {
        let mut domains: Vec<String> = (0..500).map(|n| format!("site{n}.test")).collect();
        domains.insert(0, "x".repeat(MAX_DOMAIN + 1));
        let meta = Meta {
            domains,
            ..Meta::default()
        };
        let result = candidate(
            &manifest(5),
            tick(ActivityKind::Watching, meta),
            Instant::now(),
        );
        let declared = manifest(5).domains.len();
        assert_eq!(result.unwrap().domains.len(), MAX_DOMAINS + declared);
    }

    #[test]
    fn candidates_carry_meta_domains_and_an_expiry() {
        let now = Instant::now();
        let meta = Meta {
            foreground: true,
            paused: true,
            application: "demo.exe".into(),
            // A duplicate of the manifest's domain, reported again by the script.
            domains: vec!["cdn.example.com".into(), "demo.example.com".into()],
        };
        let result = candidate(&manifest(20), tick(ActivityKind::Playing, meta), now).unwrap();
        assert!(result.foreground && result.paused);
        assert_eq!(result.application, "demo.exe");
        assert_eq!(result.domains, ["cdn.example.com", "demo.example.com"]);
        assert_eq!(result.expires_at, Some(now + Duration::from_secs(60)));
        let quick = candidate(
            &manifest(1),
            tick(ActivityKind::Playing, Meta::default()),
            now,
        );
        assert_eq!(quick.unwrap().expires_at, Some(now + MIN_SILENCE));
    }

    #[test]
    fn an_empty_tick_offers_nothing() {
        let empty = Tick {
            activity: None,
            meta: Meta::default(),
            snapshot: None,
        };
        assert!(candidate(&manifest(5), empty, Instant::now()).is_none());
    }

    #[test]
    fn a_runner_reports_results_and_stops_on_request() {
        let (updates, mut received) = unbounded_channel();
        let source = "function read(){return {}} function activity(){return {details:'Hello'}}";
        let mut runner = Runner::spawn(presence(source), Silent, Inputs::none(), updates);
        let first = received.blocking_recv().unwrap();
        assert!(matches!(first, Message::Candidate(c) if c.source == "Demo"));
        runner.stop();
        // After stopping, the channel drains and closes.
        while received.try_recv().is_ok() {}
        assert!(received.blocking_recv().is_none());
    }

    /// Answers the process list only after a long pause, and says when it was asked.
    struct Stuck {
        asked: std::sync::mpsc::Sender<()>,
        pause: Duration,
    }

    impl Host for Stuck {
        fn call(&self, _: Request) -> Result<Value, String> {
            let _ = self.asked.send(());
            thread::sleep(self.pause);
            Ok(serde_json::json!(["late.exe"]))
        }
    }

    #[test]
    fn a_script_stuck_in_a_host_call_does_not_hold_up_its_stop_or_speak_after_it() {
        let (updates, mut received) = unbounded_channel();
        let (asked, called) = mpsc::channel();
        let mut slow = presence(
            "function read(){ return {n: host.process.list().length} }              function activity(s){ return {details: 'late ' + s.n} }",
        );
        slow.manifest.permissions.process = true;
        let host = Stuck {
            asked,
            pause: STOP_WAIT + Duration::from_millis(1500),
        };
        let mut runner = Runner::spawn(slow, host, Inputs::none(), updates);
        called.recv_timeout(Duration::from_secs(5)).unwrap();

        let started = Instant::now();
        runner.stop();
        let waited = started.elapsed();
        assert!(
            waited >= STOP_WAIT - Duration::from_millis(100),
            "{waited:?}"
        );
        assert!(
            waited < STOP_WAIT + Duration::from_millis(700),
            "{waited:?}"
        );

        // The call returns after the stop; its result must not be published.
        thread::sleep(Duration::from_millis(2500));
        let mut late = Vec::new();
        while let Ok(message) = received.try_recv() {
            late.push(message);
        }
        assert!(
            !late.iter().any(|m| matches!(m, Message::Candidate(_))),
            "{late:?}"
        );
    }

    const COUNTING: &str = "let reads = 0;
        function read(c){ reads += 1; return {n: c.pages.length, reads} }
        function activity(s){ return s.n > 0 ? {details: 'read ' + s.reads} : null }";

    /// A presence that reads pages, with a list of pages the test controls.
    fn web_presence() -> (Presence, Inputs, Arc<std::sync::Mutex<Value>>) {
        let mut web = presence(COUNTING);
        web.manifest = Manifest::parse(
            r#"{"name":"Demo","version":"1","client_id":"12345678901234567","interval":300,
                "page":{"matches":["*://a.io/*"],"extract":{"t":{"from":"text","selector":"h1"}}}}"#,
        )
        .unwrap();
        let pages = Arc::new(std::sync::Mutex::new(json!([])));
        let list = Arc::clone(&pages);
        let inputs = Inputs {
            pages: Arc::new(move |_: &str| list.lock().unwrap().clone()),
            settings: Inputs::none().settings,
        };
        (web, inputs, pages)
    }

    fn next(received: &mut tokio::sync::mpsc::UnboundedReceiver<Message>) -> Option<Message> {
        for _ in 0..200 {
            if let Ok(message) = received.try_recv() {
                return Some(message);
            }
            thread::sleep(Duration::from_millis(5));
        }
        None
    }

    fn shown_text(message: Option<Message>) -> String {
        match message {
            Some(Message::Candidate(candidate)) => candidate.activity.details.unwrap_or_default(),
            other => panic!("expected an activity, got {other:?}"),
        }
    }

    #[test]
    fn blank_pages_are_those_with_every_value_empty() {
        assert!(pages_blank(
            &json!([{"values": {"title": null, "player": null}}])
        ));
        assert!(!pages_blank(
            &json!([{"values": {"title": "A", "player": null}}])
        ));
        assert!(!pages_blank(&json!([
            {"values": {"title": null}},
            {"values": {"title": "A"}}
        ])));
        // No pages, or a presence that reads no values, is not a broken site.
        assert!(!pages_blank(&json!([])));
        assert!(!pages_blank(&json!([{"values": {}}])));
    }

    #[test]
    fn blank_pages_are_reported_once_after_a_while_and_again_when_they_recover() {
        let start = Instant::now();
        let later = start + BLANK_AFTER;
        let mut watch = BlankWatch::default();
        assert_eq!(watch.observe(true, start), None);
        assert_eq!(watch.observe(true, later), Some(true));
        assert_eq!(watch.observe(true, later + BLANK_AFTER), None);
        assert_eq!(watch.observe(false, later), Some(false));
        assert_eq!(watch.observe(false, later), None);
    }

    #[test]
    fn a_short_blank_says_nothing() {
        let start = Instant::now();
        let mut watch = BlankWatch::default();
        assert_eq!(watch.observe(true, start), None);
        assert_eq!(watch.observe(false, start), None);
        assert_eq!(watch.observe(true, start + BLANK_AFTER), None);
    }

    #[test]
    fn only_a_presence_with_no_other_permission_waits_for_pages() {
        let (web, _, _) = web_presence();
        assert!(needs_pages(&web.manifest));
        let mut with_process = web.manifest.clone();
        with_process.permissions.process = true;
        assert!(!needs_pages(&with_process));
        assert!(!needs_pages(&manifest(5)));
    }

    #[test]
    fn a_page_presence_sleeps_until_woken_and_clears_at_once() {
        let (web, inputs, pages) = web_presence();
        let (updates, mut received) = unbounded_channel();
        let mut runner = Runner::spawn(web, Silent, inputs, updates);
        // One clear and no read; the long interval plays no part.
        assert!(matches!(next(&mut received), Some(Message::Clear { .. })));
        thread::sleep(Duration::from_millis(100));
        assert!(received.try_recv().is_err(), "it read with no pages");

        *pages.lock().unwrap() = json!([{"url": "https://a.io/"}]);
        runner.wake();
        assert!(shown_text(next(&mut received)).starts_with("read"));

        *pages.lock().unwrap() = json!([]);
        runner.wake();
        assert!(matches!(next(&mut received), Some(Message::Clear { .. })));
        // More wakes with no pages stay silent: the clear was already sent.
        runner.wake();
        thread::sleep(Duration::from_millis(50));
        assert!(received.try_recv().is_err());

        let started = Instant::now();
        runner.stop();
        assert!(started.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn a_short_gap_keeps_the_script_and_a_long_one_frees_it() {
        let (web, inputs, pages) = web_presence();
        let (updates, mut received) = unbounded_channel();
        let runner = Runner::spawn(web, Silent, inputs, updates);
        next(&mut received);

        let open = json!([{"url": "https://a.io/"}]);
        *pages.lock().unwrap() = open.clone();
        runner.wake();
        assert_eq!(shown_text(next(&mut received)), "read 1");

        // A reload: gone and back before the script is freed.
        *pages.lock().unwrap() = json!([]);
        runner.wake();
        next(&mut received);
        *pages.lock().unwrap() = open.clone();
        runner.wake();
        assert_eq!(shown_text(next(&mut received)), "read 2");

        // Gone for longer: the script loads again from the start.
        *pages.lock().unwrap() = json!([]);
        runner.wake();
        next(&mut received);
        thread::sleep(IDLE_UNLOAD * 2);
        *pages.lock().unwrap() = open;
        runner.wake();
        assert_eq!(shown_text(next(&mut received)), "read 1");
    }

    #[test]
    fn a_broken_page_script_reports_at_the_start_without_a_page() {
        let (mut web, inputs, _) = web_presence();
        web.source = "this is not javascript".into();
        let (updates, mut received) = unbounded_channel();
        let _runner = Runner::spawn(web, Silent, inputs, updates);
        assert!(matches!(next(&mut received), Some(Message::Failed { .. })));
    }

    #[test]
    fn a_stop_waits_no_longer_than_its_deadline_and_a_quick_one_is_immediate() {
        let (updates, mut received) = unbounded_channel();
        let source = "function read(){return {}} function activity(){return {details:'Hello'}}";
        let mut runner = Runner::spawn(presence(source), Silent, Inputs::none(), updates);
        received.blocking_recv().unwrap();
        let started = Instant::now();
        runner.stop_by(Instant::now() + Duration::from_secs(10));
        assert!(started.elapsed() < Duration::from_millis(500));
        runner.stop();
    }

    #[test]
    fn a_panic_ends_the_runner_and_is_reported_as_a_failure() {
        let (updates, mut received) = unbounded_channel();
        let source = "function read(){return {}} function activity(){return {details:'x'}}";
        let inputs = Inputs {
            pages: Arc::new(|_| panic!("a broken page source")),
            settings: crate::no_settings(),
        };
        let runner = Runner::spawn(presence(source), Silent, inputs, updates);
        let message = received.blocking_recv().unwrap();
        assert!(
            matches!(&message, Message::Failed { source, error } if source == "Demo" && error.contains("unexpectedly")),
            "{message:?}"
        );
        let started = Instant::now();
        while runner.is_alive() && started.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!runner.is_alive());
    }

    #[test]
    fn a_script_that_does_not_load_leaves_no_live_runner() {
        let (updates, mut received) = unbounded_channel();
        let runner = Runner::spawn(presence("function ("), Silent, Inputs::none(), updates);
        assert!(matches!(
            received.blocking_recv(),
            Some(Message::Failed { .. })
        ));
        let started = Instant::now();
        while runner.is_alive() && started.elapsed() < Duration::from_secs(5) {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(!runner.is_alive());
    }

    #[test]
    fn scripts_receive_the_pages_of_their_presence() {
        let (updates, mut received) = unbounded_channel();
        let source = "function read(c){return {title: c.pages[0].title}} function activity(s){return {details: s.title}}";
        let pages: PageSource =
            Arc::new(|presence| json!([{ "title": format!("page for {presence}") }]));
        let inputs = Inputs {
            pages,
            ..Inputs::none()
        };
        let _runner = Runner::spawn(presence(source), Silent, inputs, updates);
        let message = received.blocking_recv().unwrap();
        let Message::Candidate(candidate) = message else {
            panic!("unexpected {message:?}");
        };
        assert_eq!(candidate.activity.details.as_deref(), Some("page for Demo"));
    }

    #[test]
    fn scripts_read_the_current_settings_of_their_presence() {
        let (updates, mut received) = unbounded_channel();
        let source = "function read(c){return {who: c.options.who}} function activity(s){return {details: 'for ' + s.who}}";
        let current = Arc::new(std::sync::Mutex::new(json!({"who": "Ana"})));
        let shared = Arc::clone(&current);
        let settings: SettingsSource = Arc::new(move |_| shared.lock().unwrap().clone());
        // Only options the manifest declares reach the script.
        let mut declared = presence(source);
        declared.manifest = Manifest::parse(
            r#"{"name":"Demo","version":"1","client_id":"12345678901234567","interval":1,
                "options":[{"type":"text","key":"who","default":"nobody"}]}"#,
        )
        .unwrap();
        let _runner = Runner::spawn(
            declared,
            Silent,
            Inputs {
                settings,
                ..Inputs::none()
            },
            updates,
        );
        let details = |message: Message| match message {
            Message::Candidate(candidate) => candidate.activity.details,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(
            details(received.blocking_recv().unwrap()).as_deref(),
            Some("for Ana")
        );
        // A change is read at the next poll, without restarting the script.
        *current.lock().unwrap() = json!({"who": "Bo"});
        assert_eq!(
            details(received.blocking_recv().unwrap()).as_deref(),
            Some("for Bo")
        );
    }

    #[test]
    fn a_wake_reads_again_at_once_with_the_new_options() {
        let (updates, mut received) = unbounded_channel();
        let source = "function read(c){return {who: c.options.who}} function activity(s){return {details: 'for ' + s.who}}";
        let current = Arc::new(std::sync::Mutex::new(json!({"who": "Ana"})));
        let shared = Arc::clone(&current);
        let settings: SettingsSource = Arc::new(move |_| shared.lock().unwrap().clone());
        let mut slow = presence(source);
        // An hour between reads: only a wake can bring the change in time.
        slow.manifest = Manifest::parse(
            r#"{"name":"Demo","version":"1","client_id":"12345678901234567","interval":300,
                "options":[{"type":"text","key":"who","default":"nobody"}]}"#,
        )
        .unwrap();
        let runner = Runner::spawn(
            slow,
            Silent,
            Inputs {
                settings,
                ..Inputs::none()
            },
            updates,
        );
        let details = |message: Message| match message {
            Message::Candidate(candidate) => candidate.activity.details,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(
            details(received.blocking_recv().unwrap()).as_deref(),
            Some("for Ana")
        );
        *current.lock().unwrap() = json!({"who": "Bo"});
        let asked = Instant::now();
        // Several wakes in a row make one read.
        runner.wake();
        runner.wake();
        runner.wake();
        assert_eq!(
            details(received.blocking_recv().unwrap()).as_deref(),
            Some("for Bo")
        );
        assert!(asked.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(300));
        assert!(
            received.try_recv().is_err(),
            "the wakes were merged into one read"
        );
        drop(runner);
    }

    #[test]
    fn load_and_script_failures_are_reported() {
        let (updates, mut received) = unbounded_channel();
        let _broken = Runner::spawn(presence("function ("), Silent, Inputs::none(), updates);
        let message = received.blocking_recv().unwrap();
        assert!(matches!(message, Message::Failed { source, .. } if source == "Demo"));

        let (updates, mut received) = unbounded_channel();
        let source = "function read(){ throw new Error('nope') } function activity(){}";
        let _throwing = Runner::spawn(presence(source), Silent, Inputs::none(), updates);
        let message = received.blocking_recv().unwrap();
        assert!(matches!(message, Message::Failed { error, .. } if error.contains("nope")));
    }
}
