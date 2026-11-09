//! Behavior of the sandbox, checked with a recording host.

use super::*;
use serde_json::json;
use std::cell::RefCell;

/// Host that records requests and answers with fixed data.
#[derive(Default)]
struct Recorder {
    calls: RefCell<Vec<Request>>,
}

impl Host for Recorder {
    fn call(&self, request: Request) -> Result<Json, String> {
        self.calls.borrow_mut().push(request.clone());
        match request {
            Request::ProcessList => Ok(json!(["game.exe", "chrome.exe"])),
            Request::ProcessArgs { .. } => Ok(json!({"-h": "127.0.0.1", "-p": "7777"})),
            Request::HttpGet { .. } => Ok(json!({"status": 200, "body": "{\"name\":\"Obby\"}"})),
            Request::ServerQuery { .. } => Ok(json!({"hostname": "Roleplay"})),
            _ => Ok(Json::Null),
        }
    }
}

fn manifest(permissions: &str) -> Manifest {
    Manifest::parse(&format!(
        r#"{{"name":"Test","version":"1","client_id":"12345678901234567","permissions":{permissions}}}"#
    ))
    .unwrap()
}

fn load(permissions: &str, source: &str) -> (Plugin, Rc<Recorder>) {
    let host = Rc::new(Recorder::default());
    let loaded = Plugin::load(manifest(permissions), source, host.clone()).unwrap();
    (loaded, host)
}

fn details(plugin: &mut Plugin) -> Option<String> {
    plugin.tick(&json!({})).unwrap().unwrap().details
}

const BASIC: &str = r#"
    function read(context) { return { title: context.title, n: context.n * 2 }; }
    function activity(snapshot) {
        if (!snapshot.title) return null;
        return { details: snapshot.title, state: "n=" + snapshot.n, kind: "watching", start: 100 };
    }
"#;

#[test]
fn read_feeds_activity() {
    let (mut plugin, _) = load("{}", BASIC);
    let shown = plugin
        .tick(&json!({"title": "Episode 1", "n": 21}))
        .unwrap()
        .unwrap();
    assert_eq!(shown.details.as_deref(), Some("Episode 1"));
    assert_eq!(shown.state.as_deref(), Some("n=42"));
    assert_eq!(shown.start, Some(100));
    assert_eq!(plugin.tick(&json!({"n": 1})).unwrap(), None);
}

#[test]
fn short_texts_are_dropped_by_validation() {
    let source = "function read(){return {}} function activity(){return {details:'A', state:'Ok'}}";
    let (mut plugin, _) = load("{}", source);
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.details, None);
    assert_eq!(shown.state.as_deref(), Some("Ok"));
}

#[test]
fn missing_functions_and_bad_syntax_are_reported() {
    let host = Rc::new(Recorder::default());
    assert!(matches!(
        Plugin::load(manifest("{}"), "function (", host.clone()),
        Err(PluginError::Load(_))
    ));
    let mut incomplete = Plugin::load(manifest("{}"), "function read(){return 1}", host).unwrap();
    assert!(matches!(
        incomplete.tick(&json!({})),
        Err(PluginError::Missing("activity"))
    ));
}

#[test]
fn replaying_skips_read_and_shows_the_snapshot() {
    let (mut plugin, _) = load(
        "{}",
        "function read(){ throw new Error('read must not run') }
         function activity(s){ return {details: s.title} }
         function meta(){ return {paused: true} }",
    );
    let tick = plugin.replay(&json!({"title": "Saved"})).unwrap();
    assert_eq!(tick.activity.unwrap().details.as_deref(), Some("Saved"));
    assert!(tick.meta.paused);
    assert_eq!(tick.snapshot.as_deref(), Some(r#"{"title":"Saved"}"#));
}

#[test]
fn replaying_keeps_the_limits_of_a_read() {
    let (mut plugin, _) = load(
        "{}",
        "function read(){return {}} function activity(){ while(true){} }",
    );
    assert!(matches!(
        plugin.replay(&json!({})),
        Err(PluginError::Timeout)
    ));
    let big = json!({"text": "x".repeat(MAX_SNAPSHOT)});
    assert!(matches!(
        plugin.replay(&big),
        Err(PluginError::SnapshotTooLarge)
    ));
}

#[test]
fn script_errors_carry_the_message() {
    let source = "function read(){ throw new Error('boom') } function activity(){}";
    let (mut plugin, _) = load("{}", source);
    match plugin.tick(&json!({})) {
        Err(PluginError::Script(message)) => assert!(message.contains("boom"), "{message}"),
        other => panic!("unexpected {other:?}"),
    }
}

#[test]
fn an_endless_loop_times_out_and_the_plugin_recovers() {
    let source = "function read(c){ if (c.spin) while(true){} return {}; } function activity(){ return {details:'Alive'} }";
    let (mut plugin, _) = load("{}", source);
    let started = Instant::now();
    assert!(matches!(
        plugin.tick(&json!({"spin": true})),
        Err(PluginError::Timeout)
    ));
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(details(&mut plugin).as_deref(), Some("Alive"));
}

#[test]
fn memory_bombs_fail_without_taking_the_process_down() {
    let source = "function read(){ const a=[]; for(;;) a.push(new Array(100000).fill(1)); } function activity(){}";
    let (mut plugin, _) = load("{}", source);
    assert!(plugin.tick(&json!({})).is_err());
}

#[test]
fn oversized_snapshots_and_promises_are_refused() {
    let big = "function read(){ return 'x'.repeat(70000) } function activity(){}";
    let (mut plugin, _) = load("{}", big);
    assert!(matches!(
        plugin.tick(&json!({})),
        Err(PluginError::SnapshotTooLarge)
    ));
    let promise = "function read(){ return Promise.resolve(1) } function activity(){}";
    let (mut plugin, _) = load("{}", promise);
    assert!(plugin.tick(&json!({})).is_err());
}

#[test]
fn invalid_activities_are_rejected() {
    let unknown =
        "function read(){return {}} function activity(){ return {details:'Hi', unknown: 1} }";
    let (mut plugin, _) = load("{}", unknown);
    assert!(matches!(
        plugin.tick(&json!({})),
        Err(PluginError::Output(_))
    ));
    let link = "function read(){return {}} function activity(){ return {details:'Hi', buttons:[{label:'Go', url:'javascript:1'}]} }";
    let (mut plugin, _) = load("{}", link);
    assert!(matches!(
        plugin.tick(&json!({})),
        Err(PluginError::Activity(_))
    ));
}

#[test]
fn scripts_can_return_party_links_and_the_member_list_choice() {
    let source = "function read(){return {}} function activity(){ return {
        details: 'Song', details_url: 'https://a.io/song',
        state: 'Artist', status_display: 'state', party_size: [2, 4] } }";
    let (mut plugin, _) = load("{}", source);
    let activity = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(activity.party_size, Some([2, 4]));
    assert_eq!(activity.details_url.as_deref(), Some("https://a.io/song"));
    assert_eq!(
        activity.status_display,
        Some(rpp_core::StatusDisplay::State)
    );

    let link = "function read(){return {}} function activity(){ return {details:'Hi', details_url:'javascript:1'} }";
    let (mut plugin, _) = load("{}", link);
    assert!(matches!(
        plugin.tick(&json!({})),
        Err(PluginError::Activity(_))
    ));
    let party = "function read(){return {}} function activity(){ return {details:'Hi', party_size:[5, 2]} }";
    let (mut plugin, _) = load("{}", party);
    assert!(matches!(
        plugin.tick(&json!({})),
        Err(PluginError::Activity(_))
    ));
    let display = "function read(){return {}} function activity(){ return {details:'Hi', status_display:'title'} }";
    let (mut plugin, _) = load("{}", display);
    assert!(plugin.tick(&json!({})).is_err());
}

#[test]
fn the_sandbox_has_no_host_and_no_dynamic_code() {
    let source = r#"
        function attempt(f) { try { return String(f()); } catch (e) { return "blocked"; } }
        function read() {
            return {
                keys: Object.keys(host).length,
                raw: typeof __host,
                eval: typeof eval,
                ctor: attempt(() => (() => 1).constructor("return 1")()),
                global: attempt(() => Function("return 1")()),
                async: attempt(() => (async () => 1).constructor("return 1")),
            };
        }
        function activity(s) { return { details: JSON.stringify(s) }; }
    "#;
    let (mut plugin, _) = load("{}", source);
    assert_eq!(
        details(&mut plugin).unwrap(),
        r#"{"keys":0,"raw":"undefined","eval":"undefined","ctor":"blocked","global":"blocked","async":"blocked"}"#
    );
}

#[test]
fn granted_capabilities_reach_the_host() {
    let source = "function read(){ return { procs: host.process.list() } } function activity(s){ return { details: s.procs.join(',') } }";
    let (mut plugin, host) = load(r#"{"process":true}"#, source);
    assert_eq!(details(&mut plugin).as_deref(), Some("game.exe,chrome.exe"));
    assert_eq!(host.calls.borrow().as_slice(), [Request::ProcessList]);

    let args_src = "function read(){ return host.process.args('GTA_SA.exe') } function activity(a){ return { details: a['-h'] + ':' + a['-p'] } }";
    let (mut plugin, host) = load(r#"{"args":{"gta_sa.exe":["-h","-p"]}}"#, args_src);
    assert_eq!(details(&mut plugin).as_deref(), Some("127.0.0.1:7777"));
    // The host is asked for the manifest's options, whatever the script says.
    assert_eq!(
        host.calls.borrow().as_slice(),
        [Request::ProcessArgs {
            name: "GTA_SA.exe".into(),
            flags: vec!["-h".into(), "-p".into()],
        }]
    );

    let query_src = "function read(){ return host.query('samp', '127.0.0.1', 7777) } function activity(s){ return { details: s.hostname } }";
    let (mut plugin, host) = load(r#"{"query":["samp"]}"#, query_src);
    assert_eq!(details(&mut plugin).as_deref(), Some("Roleplay"));
    assert_eq!(
        host.calls.borrow().as_slice(),
        [Request::ServerQuery {
            protocol: "samp".into(),
            host: "127.0.0.1".into(),
            port: 7777,
        }]
    );
}

#[test]
fn undeclared_programs_and_protocols_never_reach_the_host() {
    let other = "function read(){ return host.process.args('steam.exe') } function activity(){ return { details: 'x' } }";
    let (mut plugin, host) = load(r#"{"args":{"gta_sa.exe":["-h"]}}"#, other);
    assert!(plugin.tick(&json!({})).is_err());
    let protocol = "function read(){ return host.query('source', '1.2.3.4', 27015) } function activity(){ return { details: 'x' } }";
    let (mut second, second_host) = load(r#"{"query":["samp"]}"#, protocol);
    assert!(second.tick(&json!({})).is_err());
    let port = "function read(){ return host.query('samp', '1.2.3.4', 70000) } function activity(){ return { details: 'x' } }";
    let (mut third, third_host) = load(r#"{"query":["samp"]}"#, port);
    assert!(third.tick(&json!({})).is_err());
    assert!(host.calls.borrow().is_empty());
    assert!(second_host.calls.borrow().is_empty());
    assert!(third_host.calls.borrow().is_empty());
}

#[test]
fn ungranted_capabilities_do_not_exist() {
    let source = "function read(){ return host.window.foreground() } function activity(){}";
    let (mut plugin, host) = load(r#"{"process":true}"#, source);
    assert!(matches!(
        plugin.tick(&json!({})),
        Err(PluginError::Script(_))
    ));
    assert!(host.calls.borrow().is_empty());
}

#[test]
fn denied_requests_never_reach_the_host() {
    let source = "function read(){ return host.http.get('https://evil.example.org/') } function activity(){}";
    let (mut plugin, host) = load(r#"{"http":["api.example.com"]}"#, source);
    match plugin.tick(&json!({})) {
        Err(PluginError::Script(message)) => {
            assert!(message.contains("permission denied"), "{message}");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert!(host.calls.borrow().is_empty());
}

#[test]
fn allowed_http_returns_parsed_data() {
    let source = "function read(){ const r = host.http.get('https://api.example.com/v1'); return JSON.parse(r.body) } function activity(s){ return { details: s.name } }";
    let (mut plugin, host) = load(r#"{"http":["api.example.com"]}"#, source);
    assert_eq!(details(&mut plugin).as_deref(), Some("Obby"));
    assert_eq!(host.calls.borrow().len(), 1);
}

#[test]
fn the_host_object_cannot_be_replaced() {
    let source = "function read(){ try { globalThis.host = {}; } catch (e) {} return { same: Object.isFrozen(host) && typeof host.process.list } } function activity(s){ return { details: String(s.same) } }";
    let (mut plugin, _) = load(r#"{"process":true}"#, source);
    assert_eq!(details(&mut plugin).as_deref(), Some("function"));
}

#[test]
fn granted_folders_are_listed_and_searchable() {
    let source = "function read(){ return { root: host.files.roots[0], latest: host.files.latest(host.files.roots[0], '.log') } } function activity(s){ return { details: s.root + '|' + s.latest } }";
    let (mut plugin, host) = load(r#"{"files":["C:/Logs"]}"#, source);
    assert_eq!(details(&mut plugin).as_deref(), Some("C:/Logs|null"));
    assert_eq!(
        host.calls.borrow().as_slice(),
        [Request::LatestFile {
            dir: "C:/Logs".into(),
            suffix: ".log".into()
        }]
    );
}

#[test]
fn meta_is_optional_and_validated() {
    let plain = "function read(){return {}} function activity(){return {details:'Hi'}}";
    let (mut plugin, _) = load("{}", plain);
    assert_eq!(plugin.poll(&json!({})).unwrap().meta, Meta::default());

    let with = "function read(){return {fg:true}} function activity(){return {details:'Hi'}} function meta(s){ return { foreground: s.fg, application: 'game.exe', domains: ['a.io'] } }";
    let (mut plugin, _) = load("{}", with);
    let meta = plugin.poll(&json!({})).unwrap().meta;
    assert!(meta.foreground && !meta.paused);
    assert_eq!(meta.application, "game.exe");
    assert_eq!(meta.domains, ["a.io"]);

    let bad = "function read(){return {}} function activity(){return {details:'Hi'}} function meta(){ return { surprise: 1 } }";
    let (mut plugin, _) = load("{}", bad);
    assert!(matches!(
        plugin.poll(&json!({})),
        Err(PluginError::Output(_))
    ));
}

const GREETING: &str = r#"
    function read() { return {}; }
    function activity() { return { details: t("hello", { who: "Ana", n: 2 }), state: t("absent") }; }
"#;

#[test]
fn t_returns_the_chosen_text_and_fills_placeholders() {
    let (mut plugin, _) = load("{}", GREETING);
    plugin
        .set_strings(&std::collections::BTreeMap::from([(
            "hello".to_owned(),
            "Hola {who}, {n} y {other}".to_owned(),
        )]))
        .unwrap();
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.details.as_deref(), Some("Hola Ana, 2 y {other}"));
    // A key with no text comes back as written.
    assert_eq!(shown.state.as_deref(), Some("absent"));
}

#[test]
fn t_works_before_any_strings_are_set() {
    let (mut plugin, _) = load("{}", GREETING);
    let shown = plugin.tick(&json!({})).unwrap().unwrap();
    assert_eq!(shown.details.as_deref(), Some("hello"));
}

fn shown_by(source: &str, context: serde_json::Value) -> serde_json::Value {
    let (mut plugin, _) = load("{}", source);
    let activity = plugin.tick(&context).unwrap().unwrap();
    serde_json::to_value(activity).unwrap()
}

#[test]
fn cut_shortens_long_texts_and_passes_nothing_through() {
    let source = r#"
        function read() { return {}; }
        function activity() {
            return { details: cut("x".repeat(300)), state: cut("abcdef", 4), large_text: String(cut(null)) };
        }
    "#;
    let shown = shown_by(source, json!({}));
    assert_eq!(shown["details"].as_str().unwrap().chars().count(), 128);
    assert!(shown["details"].as_str().unwrap().ends_with('…'));
    assert_eq!(shown["state"], "abc…");
    assert_eq!(shown["large_text"], "null");
}

#[test]
fn rotate_takes_the_next_item_every_few_seconds_and_skips_empty_ones() {
    let source = r#"
        function read() { return {}; }
        function activity() {
            const items = ["a", "b", "c"];
            return {
                details: [
                    rotate(items, 10, 0), rotate(items, 10, 15), rotate(items, 10, 25), rotate(items, 10, 35),
                    rotate(["a", null, "", undefined, "b"], 10, 10),
                    String(rotate([], 10, 0)), String(rotate(null, 10, 0)), String(rotate([null, ""], 10, 0)),
                    rotate(["a", "b"], 0, 30),
                ].join("|"),
            };
        }
    "#;
    let shown = shown_by(source, json!({}));
    assert_eq!(shown["details"], "a|b|c|a|b|null|null|null|b");
}

#[test]
fn format_fills_a_text_the_person_wrote_and_drops_what_is_missing() {
    let source = r##"
        function read() { return {}; }
        function activity() {
            return {
                details: [
                    format("{title} - {channel}", { title: "T", channel: "C" }),
                    format("{title} - {channel}", { title: "T", channel: null }),
                    format("{title} - {channel} - {year}", { title: "T", channel: "", year: 2020 }),
                    format("{channel} | {title}", { title: "T" , channel: undefined }),
                    format("{tittle}", { title: "T" }),
                    String(format("{a}", { a: null })),
                    String(format(5, {})),
                ].join("#"),
                state: format("{url} - {name}", { url: "https://a.io/x", name: "" }),
            };
        }
    "##;
    let shown = shown_by(source, json!({}));
    assert_eq!(shown["details"], "T - C#T#T - 2020#T#{tittle}#null#null");
    assert_eq!(shown["state"], "https://a.io/x");
}

#[test]
fn icon_gives_the_address_of_a_shared_picture() {
    let source = r#"
        function read() { return {}; }
        function activity() {
            return { large_image: icon("play"), small_image: icon("repeat-one"), details: String(icon("nope")) };
        }
    "#;
    let shown = shown_by(source, json!({}));
    assert_eq!(
        shown["large_image"],
        "https://manucabral.github.io/RichPresencePlus/icons/play.png"
    );
    assert_eq!(
        shown["small_image"],
        "https://manucabral.github.io/RichPresencePlus/icons/repeat-one.png"
    );
    assert_eq!(shown["details"], "null");
}

#[test]
fn a_script_that_takes_the_name_of_a_helper_is_told_which_one() {
    for name in ["format", "icon", "cut"] {
        let source = format!(
            "function {name}(x) {{ return x; }} function read() {{ return {{}}; }} function activity() {{ return null; }}"
        );
        let error = Plugin::load(manifest("{}"), &source, Rc::new(Recorder::default()))
            .err()
            .unwrap()
            .to_string();
        assert!(
            error.contains(&format!("{name} is a helper of the application")),
            "{error}"
        );
    }
}

/// Every name the prelude defines is in [`RESERVED`], so the message above covers it.
#[test]
fn reserved_lists_every_helper_of_the_prelude() {
    let prelude = include_str!("prelude.js");
    let mut defined: Vec<&str> = prelude
        .split("define(\"")
        .skip(1)
        .map(|rest| &rest[..rest.find('"').unwrap()])
        .collect();
    defined.extend(["host", "console", "t"]);
    defined.sort_unstable();
    let mut reserved = RESERVED.to_vec();
    reserved.sort_unstable();
    assert_eq!(defined, reserved);
}

#[test]
fn parse_url_splits_host_path_and_decoded_query() {
    let source = r#"
        function read() { return {}; }
        function activity() {
            const a = parseUrl("https://www.example.com/watch/42?v=a%20b&t=1+2&flag&bad=%E0%A4#frag");
            const none = parseUrl("javascript:alert(1)");
            return {
                details: [a.host, a.path, a.query.v, a.query.t, "flag" in a.query, a.query.bad].join("|"),
                state: String(none) + "|" + parseUrl("https://a.io").path,
            };
        }
    "#;
    let shown = shown_by(source, json!({}));
    assert_eq!(
        shown["details"],
        "www.example.com|/watch/42|a b|1 2|true|%E0%A4"
    );
    assert_eq!(shown["state"], "null|/");
}

#[test]
fn best_page_prefers_a_running_player() {
    let source = r#"
        function read(context) { const page = bestPage(context.pages); return page && { tab: page.tab }; }
        function activity(snapshot) { return { details: "tab " + snapshot.tab }; }
    "#;
    let page = |tab: u32, paused: Option<bool>| match paused {
        Some(paused) => json!({"tab": tab, "values": {"player": {"paused": paused}}}),
        None => json!({"tab": tab, "values": {}}),
    };
    let pick =
        |pages: serde_json::Value| shown_by(source, json!({"pages": pages}))["details"].clone();
    assert_eq!(
        pick(json!([
            page(1, None),
            page(2, Some(true)),
            page(3, Some(false))
        ])),
        "tab 3"
    );
    assert_eq!(pick(json!([page(1, None), page(2, Some(true))])), "tab 2");
    assert_eq!(pick(json!([page(1, None)])), "tab 1");

    // With no pages, `bestPage` gives nothing and the script shows nothing.
    let (mut plugin, _) = load("{}", source);
    assert!(plugin.tick(&json!({"pages": []})).unwrap().is_none());
    assert!(plugin.tick(&json!({})).unwrap().is_none());
}

#[test]
fn playback_gives_a_progress_bar_only_while_playing() {
    let source = r#"
        function read(context) { return context; }
        function activity(snapshot) {
            const p = playback(snapshot.player, snapshot.at);
            return { details: [p.playing, p.start, p.end].join("|") };
        }
    "#;
    let details = |player: serde_json::Value| {
        shown_by(
            source,
            json!({"player": player, "at": 1_800_000_000_000_u64}),
        )["details"]
            .clone()
    };
    let running = |current: f64, duration: serde_json::Value, rate: f64| json!({"paused": false, "current": current, "duration": duration, "rate": rate});
    assert_eq!(
        details(running(60.0, json!(212.5), 1.0)),
        "true|1799999940|1800000152"
    );
    // At double speed the video started later than the position suggests,
    // and its 100 seconds take 50 of real time.
    assert_eq!(
        details(running(60.0, json!(100.0), 2.0)),
        "true|1799999970|1800000020"
    );
    // A live stream has no length; a missing rate counts as normal speed.
    assert_eq!(details(running(10.0, json!(null), 0.0)), "true|1799999990|");
    assert_eq!(details(json!({"paused": true, "current": 5.0})), "false||");
    assert_eq!(details(json!(null)), "false||");
}

#[test]
fn a_script_that_reads_nothing_shows_nothing_and_skips_activity() {
    let source = r#"
        function read() { return null; }
        function activity() { throw new Error("must not run"); }
    "#;
    let (mut plugin, _) = load("{}", source);
    let tick = plugin.poll(&json!({})).unwrap();
    assert!(tick.activity.is_none());
}

#[test]
fn scripts_always_see_every_declared_option_and_nothing_else() {
    let manifest = Manifest::parse(
        r#"{"name":"Test","version":"1","client_id":"12345678901234567",
            "options":[{"type":"number","key":"size","default":3,"max":10},
                        {"type":"boolean","key":"on","default":true}]}"#,
    )
    .unwrap();
    let source = "function read(c){return {s: JSON.stringify(c.options)}} function activity(x){return {details: x.s}}";
    let mut plugin = Plugin::load(manifest, source, Rc::new(Recorder::default())).unwrap();
    // Compared as values: whether keys keep their order depends on which
    // crates of the workspace turn on serde_json's `preserve_order`.
    let shown = |plugin: &mut Plugin, context: serde_json::Value| {
        let text = plugin.tick(&context).unwrap().unwrap().details.unwrap();
        serde_json::from_str::<serde_json::Value>(&text).unwrap()
    };
    // Nothing given: the defaults.
    assert_eq!(
        shown(&mut plugin, json!({})),
        json!({"on": true, "size": 3})
    );
    // A value out of range falls back, an undeclared key is dropped.
    assert_eq!(
        shown(
            &mut plugin,
            json!({"options": {"size": 99, "on": false, "extra": 1}})
        ),
        json!({"on": false, "size": 3})
    );
}

#[test]
fn a_poll_returns_what_read_returned_as_text() {
    let (mut plugin, _) = load("{}", BASIC);
    let tick = plugin.poll(&json!({"title": "A", "n": 3})).unwrap();
    let snapshot: serde_json::Value = serde_json::from_str(&tick.snapshot.unwrap()).unwrap();
    assert_eq!(snapshot, json!({"title": "A", "n": 6}));

    // A script that reads nothing has no snapshot either.
    let (mut idle, _) = load(
        "{}",
        "function read(){return null} function activity(){return null}",
    );
    assert_eq!(idle.poll(&json!({})).unwrap().snapshot, None);
}

#[test]
fn console_lines_reach_the_developer_tools_and_need_no_permission() {
    let source = r#"
        function read() {
            console.log("hello", { a: 1 }, [2], null, undefined);
            console.warn("careful");
            console.error("broken");
            return {};
        }
        function activity() { return null; }
    "#;
    let (mut plugin, host) = load("{}", source);
    plugin.poll(&json!({})).unwrap();
    assert_eq!(
        plugin.take_console(),
        [
            "hello {\"a\":1} [2] null undefined",
            "[warn] careful",
            "[error] broken"
        ]
    );
    assert!(plugin.take_console().is_empty());
    assert!(
        host.calls.borrow().is_empty(),
        "printing is not a host call"
    );
}

#[test]
fn a_script_cannot_replace_the_console_or_flood_it() {
    let source = r#"
        function read() {
            for (let i = 0; i < 500; i++) console.log("x".repeat(2000));
            return {};
        }
        function activity() { return null; }
    "#;
    let (mut plugin, _) = load("{}", source);
    plugin.poll(&json!({})).unwrap();
    let lines = plugin.take_console();
    assert_eq!(lines.len(), 50);
    assert!(lines.iter().all(|line| line.chars().count() <= 500));

    // The console is frozen like `host`: replacing it stops the script at load.
    let hostile = Plugin::load(
        manifest("{}"),
        "console = null; function read(){ return {} } function activity(){ return null }",
        Rc::new(Recorder::default()),
    );
    assert!(matches!(hostile, Err(PluginError::Load(message)) if message.contains("read-only")));
}
