//! The tray icon and its menu.

use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock, PoisonError};

use rpp_core::locale::{self, Table};
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::commands::Shared;
use crate::window;

/// Id of the one tray icon.
const TRAY_ID: &str = "main";
/// Most favorites listed; the window lists every presence.
pub const MAX_PRESENCES: usize = 40;

/// What the menu shows, read from the application when it is built.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TrayModel {
    /// Language tag of the texts.
    pub language: String,
    /// Publishing is paused.
    pub paused: bool,
    /// The application starts with Windows.
    pub startup: bool,
    /// Saved custom statuses, by name.
    pub presets: Vec<String>,
    /// The custom status on display.
    pub active_preset: Option<String>,
    /// The favorite presences and whether each is on. Only favorites go in
    /// the menu, so it stays short however many presences are installed.
    pub presences: Vec<(String, bool)>,
}

/// One line of the menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// A plain item.
    Item { id: String, text: String },
    /// An item with a check mark.
    Check {
        id: String,
        text: String,
        checked: bool,
    },
    /// A line that only informs.
    Note { text: String },
    /// A submenu.
    Sub { text: String, entries: Vec<Entry> },
    /// A dividing line.
    Separator,
}

/// What a click asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Show the window.
    Open,
    /// Close the application.
    Quit,
    /// Pause publishing, or resume it.
    Pause(bool),
    /// Start with Windows, or stop doing so.
    Startup(bool),
    /// Show a custom status, or none.
    Preset(Option<String>),
    /// Turn a presence on or off.
    Presence(String, bool),
}

/// The texts of the window, which also hold the tray's (`tray.*` keys).
fn tables() -> &'static BTreeMap<String, Table> {
    static TABLES: OnceLock<BTreeMap<String, Table>> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut tables = BTreeMap::new();
        for (tag, text) in [
            ("en", include_str!("../../frontend/src/locales/en.json")),
            ("es", include_str!("../../frontend/src/locales/es.json")),
        ] {
            if let Ok(table) = serde_json::from_str::<Table>(text) {
                tables.insert(tag.to_owned(), table);
            }
        }
        tables
    })
}

fn texts(language: &str) -> Table {
    locale::resolve(tables(), language)
}

fn text(table: &Table, key: &str) -> String {
    table.get(key).cloned().unwrap_or_else(|| key.to_owned())
}

fn check(id: impl Into<String>, text: impl Into<String>, checked: bool) -> Entry {
    Entry::Check {
        id: id.into(),
        text: text.into(),
        checked,
    }
}

/// The menu for a state of the application.
pub fn entries(model: &TrayModel) -> Vec<Entry> {
    let t = texts(&model.language);
    let mut statuses = vec![check(
        "manual:none",
        text(&t, "tray.none"),
        model.active_preset.is_none(),
    )];
    statuses.extend(model.presets.iter().enumerate().map(|(index, name)| {
        check(
            format!("manual:{index}"),
            name.clone(),
            model.active_preset.as_deref() == Some(name),
        )
    }));
    let mut presences: Vec<Entry> = model
        .presences
        .iter()
        .take(MAX_PRESENCES)
        .enumerate()
        .map(|(index, (name, on))| check(format!("presence:{index}"), name.clone(), *on))
        .collect();
    if presences.is_empty() {
        presences.push(Entry::Note {
            text: text(&t, "tray.noFavorites"),
        });
    }
    vec![
        Entry::Item {
            id: "open".into(),
            text: text(&t, "tray.open"),
        },
        Entry::Separator,
        check("pause", text(&t, "tray.pause"), model.paused),
        Entry::Sub {
            text: text(&t, "tray.status"),
            entries: statuses,
        },
        Entry::Sub {
            text: text(&t, "tray.favorites"),
            entries: presences,
        },
        check("startup", text(&t, "tray.startup"), model.startup),
        Entry::Separator,
        Entry::Item {
            id: "quit".into(),
            text: text(&t, "tray.quit"),
        },
    ]
}

fn position(id: &str, prefix: &str) -> Option<usize> {
    id.strip_prefix(prefix)?.parse().ok()
}

/// What a click on `id` asks for, read against the menu that was on screen.
pub fn action(model: &TrayModel, id: &str) -> Option<Action> {
    match id {
        "open" => Some(Action::Open),
        "quit" => Some(Action::Quit),
        "pause" => Some(Action::Pause(!model.paused)),
        "startup" => Some(Action::Startup(!model.startup)),
        "manual:none" => Some(Action::Preset(None)),
        _ => {
            if let Some(index) = position(id, "manual:") {
                let name = model.presets.get(index)?;
                // Clicking the status on display takes it off.
                let shown = model.active_preset.as_deref() == Some(name.as_str());
                return Some(Action::Preset((!shown).then(|| name.clone())));
            }
            let index = position(id, "presence:")?;
            let (name, on) = model.presences.get(index)?;
            Some(Action::Presence(name.clone(), !on))
        }
    }
}

/// What the tray tooltip describes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TipState {
    /// Language tag of the texts.
    pub language: String,
    /// Publishing is paused.
    pub paused: bool,
    /// A Discord client is running.
    pub discord_running: bool,
    /// Everything is hidden because the computer is idle.
    pub idle: bool,
    /// Everything is hidden because a rule of the person holds.
    pub quiet: bool,
    /// Name of the activity on display.
    pub showing: Option<String>,
}

/// Windows cuts tray tooltips at 127 characters.
const MAX_TIP: usize = 127;

/// The tooltip of the tray icon: the application and what it is doing.
pub fn tooltip(state: &TipState) -> String {
    let t = texts(&state.language);
    let doing = if state.paused {
        text(&t, "tray.tipPaused")
    } else if !state.discord_running {
        text(&t, "tray.tipNoDiscord")
    } else if state.quiet {
        text(&t, "tray.tipQuiet")
    } else if state.idle {
        text(&t, "tray.tipIdle")
    } else if let Some(name) = &state.showing {
        locale::fill(
            &text(&t, "tray.tipShowing"),
            &BTreeMap::from([("name".to_owned(), name.clone())]),
        )
    } else {
        text(&t, "tray.tipNothing")
    };
    format!("Rich Presence Plus: {doing}")
        .chars()
        .take(MAX_TIP)
        .collect()
}

fn tip_of(app: &AppHandle) -> Option<TipState> {
    let glance = app.try_state::<Shared>()?.glance();
    Some(TipState {
        language: glance.language,
        paused: glance.paused,
        discord_running: glance.discord_running,
        idle: glance.idle,
        quiet: glance.quiet,
        showing: glance.showing,
    })
}

/// The colored dot drawn on the tray icon.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Something is on Discord.
    Showing,
    /// Publishing is paused.
    Paused,
    /// Discord is closed, or everything is hidden while away.
    Off,
}

impl Badge {
    fn color(self) -> [u8; 4] {
        match self {
            Self::Showing => [0x19, 0xC2, 0x83, 0xFF],
            Self::Paused => [0xF5, 0xB8, 0x3D, 0xFF],
            Self::Off => [0x8A, 0x8F, 0x98, 0xFF],
        }
    }
}

/// The badge for a state; none when there is simply nothing to show.
pub fn badge(state: &TipState) -> Option<Badge> {
    if state.paused {
        Some(Badge::Paused)
    } else if !state.discord_running || state.idle || state.quiet {
        Some(Badge::Off)
    } else if state.showing.is_some() {
        Some(Badge::Showing)
    } else {
        None
    }
}

/// A copy of an RGBA image, four bytes per pixel, with a dot in a dark ring in
/// the lower right corner. An image whose size does not match its bytes comes
/// back unchanged.
pub fn with_badge(rgba: &[u8], width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    let (w, h) = (width as usize, height as usize);
    if w == 0 || h == 0 || out.len() != w * h * 4 {
        return out;
    }
    let side = w.min(h) as f32;
    let radius = side * 0.24;
    let ring = (side * 0.06).max(1.0);
    let (cx, cy) = (w as f32 - radius - ring, h as f32 - radius - ring);
    for y in 0..h {
        for x in 0..w {
            let distance = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let paint = if distance <= radius {
                color
            } else if distance <= radius + ring {
                [0x08, 0x09, 0x0A, 0xFF]
            } else {
                continue;
            };
            let at = (y * w + x) * 4;
            out[at..at + 4].copy_from_slice(&paint);
        }
    }
    out
}

/// How often the tooltip is brought up to date.
const TIP_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// Keeps the tooltip current; it is set only when its text changes.
fn watch_tooltip(app: &AppHandle) {
    let app = app.clone();
    // The plain icon, kept to draw each badge on a clean copy.
    let base = app
        .default_window_icon()
        .map(|icon| (icon.rgba().to_vec(), icon.width(), icon.height()));
    let started = std::thread::Builder::new()
        .name("tray-tip".into())
        .spawn(move || {
            let mut shown = String::new();
            // `Some(None)` is the plain icon; `None` means nothing was set yet.
            let mut drawn: Option<Option<Badge>> = None;
            loop {
                std::thread::sleep(TIP_EVERY);
                let Some(state) = tip_of(&app) else { continue };
                let Some(tray) = app.tray_by_id(TRAY_ID) else {
                    continue;
                };
                let tip = tooltip(&state);
                if tip != shown && tray.set_tooltip(Some(&tip)).is_ok() {
                    shown = tip;
                }
                let wanted = badge(&state);
                if drawn != Some(wanted)
                    && let Some((rgba, width, height)) = &base
                {
                    let pixels = match wanted {
                        Some(badge) => with_badge(rgba, *width, *height, badge.color()),
                        None => rgba.clone(),
                    };
                    let image = tauri::image::Image::new_owned(pixels, *width, *height);
                    if tray.set_icon(Some(image)).is_ok() {
                        drawn = Some(wanted);
                    }
                }
            }
        });
    if let Err(error) = started {
        tracing::warn!("the tray tooltip will not follow the state: {error}");
    }
}

/// The menu on screen, so clicks resolve against what the user saw.
#[derive(Default)]
pub struct TrayState(Mutex<TrayModel>);

fn model_of(app: &AppHandle) -> TrayModel {
    let Some(controller) = app.try_state::<Shared>() else {
        return TrayModel::default();
    };
    let language = controller.language();
    let manual = controller.manual();
    TrayModel {
        language: if language.setting == "auto" {
            language.system
        } else {
            language.setting
        },
        paused: controller.preferences().global.paused,
        startup: crate::startup::state().enabled,
        presets: manual
            .presets
            .into_iter()
            .map(|preset| preset.name)
            .collect(),
        active_preset: manual.active,
        presences: favorites_of(
            controller
                .presences()
                .into_iter()
                .map(|presence| (presence.name, presence.enabled)),
            &controller.favorites(),
        ),
    }
}

/// The favorites among `presences` (name and switch), in the presences' order.
/// A favorite that is no longer installed is left out.
fn favorites_of(
    presences: impl IntoIterator<Item = (String, bool)>,
    favorites: &[String],
) -> Vec<(String, bool)> {
    presences
        .into_iter()
        .filter(|(name, _)| favorites.contains(name))
        .collect()
}

fn menu(app: &AppHandle, entries: &[Entry]) -> tauri::Result<Menu<Wry>> {
    let menu = Menu::new(app)?;
    for entry in entries {
        append(app, entry, &|item| menu.append(item))?;
    }
    Ok(menu)
}

fn append(
    app: &AppHandle,
    entry: &Entry,
    add: &dyn Fn(&dyn tauri::menu::IsMenuItem<Wry>) -> tauri::Result<()>,
) -> tauri::Result<()> {
    match entry {
        Entry::Item { id, text } => add(&MenuItem::with_id(app, id, text, true, None::<&str>)?),
        Entry::Check { id, text, checked } => add(&CheckMenuItem::with_id(
            app,
            id,
            text,
            true,
            *checked,
            None::<&str>,
        )?),
        Entry::Note { text } => add(&MenuItem::new(app, text, false, None::<&str>)?),
        Entry::Separator => add(&PredefinedMenuItem::separator(app)?),
        Entry::Sub { text, entries } => {
            let submenu = Submenu::new(app, text, true)?;
            for inner in entries {
                append(app, inner, &|item| submenu.append(item))?;
            }
            add(&submenu)
        }
    }
}

/// Rebuilds the menu from the current state of the application.
pub fn refresh(app: &AppHandle) {
    let model = model_of(app);
    let Ok(built) = menu(app, &entries(&model)) else {
        return;
    };
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_menu(Some(built));
    }
    if let Some(state) = app.try_state::<TrayState>() {
        *state.0.lock().unwrap_or_else(PoisonError::into_inner) = model;
    }
}

fn perform(app: &AppHandle, action: Action) {
    let Some(controller) = app.try_state::<Shared>().map(|c| Shared::clone(&c)) else {
        return;
    };
    let result = match action {
        Action::Open | Action::Quit => Ok(()),
        Action::Pause(paused) => {
            let mut preferences = controller.preferences();
            preferences.global.paused = paused;
            controller.set_preferences(preferences).map(|_| ())
        }
        Action::Startup(enabled) => crate::startup::set(enabled, true)
            .and_then(|()| controller.save_window_choices(controller.minimize_to_tray())),
        Action::Preset(name) => controller.show_manual(name.as_deref()),
        Action::Presence(name, on) => controller.set_enabled(&name, on),
    };
    if let Err(error) = result {
        tracing::warn!("a tray action failed: {error}");
    }
    refresh(app);
    // Pages that are open read the change.
    let _ = app.emit("app-changed", ());
}

fn on_click(app: &AppHandle, id: &str) {
    let model = app
        .try_state::<TrayState>()
        .map(|state| {
            state
                .0
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        })
        .unwrap_or_default();
    match action(&model, id) {
        Some(Action::Open) => window::show(app),
        Some(Action::Quit) => app.exit(0),
        // Turning a presence off can wait for its thread; not on the main thread.
        Some(other) => {
            let app = app.clone();
            tauri::async_runtime::spawn_blocking(move || perform(&app, other));
        }
        None => {}
    }
}

/// Creates the tray icon with its menu.
pub fn build(app: &AppHandle) -> tauri::Result<()> {
    app.manage(TrayState::default());
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("Rich Presence Plus")
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| on_click(app, event.id.as_ref()))
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                window::show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    refresh(app);
    watch_tooltip(app);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model() -> TrayModel {
        TrayModel {
            language: "en".into(),
            paused: false,
            startup: true,
            presets: vec!["Studying".into(), "Coding".into()],
            active_preset: Some("Coding".into()),
            presences: vec![("YouTube".into(), true), ("Roblox".into(), false)],
        }
    }

    fn ids(entries: &[Entry]) -> Vec<String> {
        let mut found = Vec::new();
        for entry in entries {
            match entry {
                Entry::Item { id, .. } | Entry::Check { id, .. } => found.push(id.clone()),
                Entry::Sub { entries, .. } => found.extend(ids(entries)),
                Entry::Note { .. } | Entry::Separator => {}
            }
        }
        found
    }

    #[test]
    fn the_menu_lists_every_option_with_positions_as_ids() {
        let shown = entries(&model());
        assert_eq!(
            ids(&shown),
            [
                "open",
                "pause",
                "manual:none",
                "manual:0",
                "manual:1",
                "presence:0",
                "presence:1",
                "startup",
                "quit"
            ]
        );
        assert!(shown.contains(&check("pause", "Pause publishing", false)));
        assert!(shown.contains(&check("startup", "Start with Windows", true)));
        let Entry::Sub {
            entries: statuses, ..
        } = &shown[3]
        else {
            panic!("{:?}", shown[3]);
        };
        assert_eq!(statuses[2], check("manual:1", "Coding", true));
        assert_eq!(statuses[0], check("manual:none", "None", false));
    }

    fn tip(language: &str) -> TipState {
        TipState {
            language: language.into(),
            discord_running: true,
            ..TipState::default()
        }
    }

    #[test]
    fn the_tooltip_says_what_the_application_is_doing() {
        assert_eq!(tooltip(&tip("en")), "Rich Presence Plus: nothing to show");
        let showing = TipState {
            showing: Some("YouTube".into()),
            ..tip("en")
        };
        assert_eq!(tooltip(&showing), "Rich Presence Plus: showing YouTube");
        // Pause, then a missing Discord, then idle: the first that applies wins.
        let all = TipState {
            paused: true,
            discord_running: false,
            idle: true,
            quiet: false,
            ..showing.clone()
        };
        assert_eq!(tooltip(&all), "Rich Presence Plus: paused");
        let away = TipState {
            idle: true,
            quiet: false,
            ..showing.clone()
        };
        assert_eq!(
            tooltip(&away),
            "Rich Presence Plus: hidden while you are away"
        );
        let closed = TipState {
            discord_running: false,
            ..showing
        };
        assert_eq!(tooltip(&closed), "Rich Presence Plus: Discord is not open");
        assert!(tooltip(&tip("es")).starts_with("Rich Presence Plus: nada"));
    }

    #[test]
    fn the_badge_follows_the_state() {
        assert_eq!(badge(&tip("en")), None);
        let showing = TipState {
            showing: Some("YouTube".into()),
            ..tip("en")
        };
        assert_eq!(badge(&showing), Some(Badge::Showing));
        let paused = TipState {
            paused: true,
            ..showing.clone()
        };
        assert_eq!(badge(&paused), Some(Badge::Paused));
        let closed = TipState {
            discord_running: false,
            ..showing.clone()
        };
        assert_eq!(badge(&closed), Some(Badge::Off));
        let away = TipState {
            idle: true,
            quiet: false,
            ..showing
        };
        assert_eq!(badge(&away), Some(Badge::Off));
    }

    #[test]
    fn the_dot_is_drawn_in_the_corner_and_nowhere_else() {
        let (w, h) = (32_u32, 32_u32);
        let plain = vec![10_u8; (w * h * 4) as usize];
        let green = [0x19, 0xC2, 0x83, 0xFF];
        let drawn = with_badge(&plain, w, h, green);
        let pixel =
            |x: u32, y: u32| &drawn[((y * w + x) * 4) as usize..((y * w + x) * 4 + 4) as usize];
        assert_eq!(pixel(24, 24), green);
        assert_eq!(pixel(2, 2), [10, 10, 10, 10]);
        assert_eq!(pixel(12, 12), [10, 10, 10, 10]);
        assert_eq!(pixel(30, 2), [10, 10, 10, 10]);
        // Bytes that do not match the size are left alone instead of panicking.
        assert_eq!(with_badge(&[1, 2, 3], 32, 32, green), [1, 2, 3]);
        assert!(with_badge(&[], 0, 0, green).is_empty());
    }

    #[test]
    fn a_long_name_is_cut_to_what_windows_shows() {
        let long = TipState {
            showing: Some("é".repeat(300)),
            ..tip("en")
        };
        assert_eq!(tooltip(&long).chars().count(), MAX_TIP);
    }

    #[test]
    fn clicks_resolve_against_the_menu_that_was_shown() {
        let m = model();
        assert_eq!(action(&m, "pause"), Some(Action::Pause(true)));
        assert_eq!(action(&m, "startup"), Some(Action::Startup(false)));
        assert_eq!(
            action(&m, "manual:0"),
            Some(Action::Preset(Some("Studying".into())))
        );
        // The status on display is taken off by clicking it.
        assert_eq!(action(&m, "manual:1"), Some(Action::Preset(None)));
        assert_eq!(action(&m, "manual:none"), Some(Action::Preset(None)));
        assert_eq!(
            action(&m, "presence:1"),
            Some(Action::Presence("Roblox".into(), true))
        );
        assert_eq!(
            action(&m, "presence:0"),
            Some(Action::Presence("YouTube".into(), false))
        );
        for unknown in [
            "presence:9",
            "manual:7",
            "presence:-1",
            "presence:x",
            "nope",
            "",
        ] {
            assert_eq!(action(&m, unknown), None, "{unknown}");
        }
    }

    #[test]
    fn only_installed_favorites_reach_the_menu() {
        let installed = vec![
            ("Netflix".to_owned(), false),
            ("Roblox".to_owned(), true),
            ("YouTube".to_owned(), true),
        ];
        let favorites = vec![
            "YouTube".to_owned(),
            "Gone".to_owned(),
            "Netflix".to_owned(),
        ];
        assert_eq!(
            favorites_of(installed, &favorites),
            vec![("Netflix".to_owned(), false), ("YouTube".to_owned(), true)]
        );
        assert!(favorites_of(vec![("Roblox".to_owned(), true)], &[]).is_empty());
    }

    #[test]
    fn long_lists_are_cut_and_an_empty_one_says_so() {
        let mut many = model();
        many.presences = (0..100).map(|n| (format!("P{n}"), true)).collect();
        let listed = ids(&entries(&many))
            .iter()
            .filter(|id| id.starts_with("presence:"))
            .count();
        assert_eq!(listed, MAX_PRESENCES);

        let mut none = model();
        none.presences.clear();
        let Entry::Sub { entries: inner, .. } = &entries(&none)[4] else {
            panic!("no presences submenu");
        };
        assert!(matches!(&inner[0], Entry::Note { text } if text.starts_with("Star a presence")));
    }

    #[test]
    fn texts_follow_the_language_and_fall_back_to_english() {
        let mut spanish = model();
        spanish.language = "es-AR".into();
        let shown = entries(&spanish);
        assert_eq!(
            shown[0],
            Entry::Item {
                id: "open".into(),
                text: "Abrir".into()
            }
        );
        let mut other = model();
        other.language = "fr".into();
        assert_eq!(
            entries(&other)[0],
            Entry::Item {
                id: "open".into(),
                text: "Open".into()
            }
        );
    }
}
