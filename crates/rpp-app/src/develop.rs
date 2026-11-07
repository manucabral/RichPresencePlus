//! Tools for people who write presences: create, reload, inspect, hot reload.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Weak};
use std::time::{SystemTime, UNIX_EPOCH};

use rpp_config::{Origin, load_presence};
use rpp_plugins::{Host, Plugin, Request, identity};
use serde_json::json;

use crate::controller::{Controller, lock};
use crate::editor;
use crate::library::effective_language;
use crate::manifest_edit::{self, ManifestChanges};
use crate::program::{self, ProgramPresence};
use crate::site;
use crate::templates::{self, Template};
use crate::views::{DevStatus, Inspection};
use crate::watcher::Watcher;

/// The folder being watched, kept alive by the watcher thread it owns.
pub(crate) struct Watched {
    folder: PathBuf,
    _watcher: Watcher,
}

/// What the last hot reload did.
#[derive(Debug, Default)]
pub(crate) struct Reloads {
    at: Option<u64>,
    error: Option<String>,
}

const NOT_LOCAL: &str = "Only presences you wrote can be modified. \
                         Copy this one to make your own.";

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Creates a presence in the user's folder from a template and returns the
    /// new folder.
    ///
    /// Raises:
    ///     String: The name cannot be used or the folder exists.
    pub fn create_presence(&self, name: &str, template: Template) -> Result<String, String> {
        self.ensure_free(name)?;
        let author = self.default_author();
        let folder = templates::create(&self.roots[0], name, template, author.as_deref())?;
        self.reload_presences();
        self.prepare_editors();
        Ok(folder.to_string_lossy().into_owned())
    }

    /// Creates a presence for a program from what the user chose and returns
    /// the new folder.
    ///
    /// Raises:
    ///     String: The choice cannot be used or the folder exists.
    pub fn create_program_presence(&self, choice: &ProgramPresence) -> Result<String, String> {
        self.ensure_free(&choice.name)?;
        let author = self.default_author();
        let folder = program::create(&self.roots[0], choice, author.as_deref())?;
        self.reload_presences();
        self.prepare_editors();
        Ok(folder.to_string_lossy().into_owned())
    }

    /// Makes the presences someone asked for from a page.
    pub fn make_requested_sites(&self) {
        for request in self.bridge.take_site_requests() {
            let made = self.ensure_free(&request.name).and_then(|()| {
                let author = self.default_author();
                site::create(&self.roots[0], &request, author.as_deref())
            });
            match made {
                Ok(_) => {
                    tracing::info!(presence = %request.name, "made a presence from a page");
                    self.reload_presences();
                }
                Err(error) => tracing::warn!("cannot make a presence from a page: {error}"),
            }
        }
    }

    /// The running programs a presence could be made for.
    pub fn running_programs(&self) -> Result<Vec<String>, String> {
        let names = self.host.call(Request::ProcessList)?;
        let names: Vec<String> = serde_json::from_value(names).map_err(|e| e.to_string())?;
        Ok(program::useful_programs(&names))
    }

    /// Copies a presence into the user's folder under a new name and returns
    /// the new folder.
    pub fn copy_presence(&self, name: &str, new_name: &str) -> Result<String, String> {
        self.ensure_free(new_name)?;
        let source = lock(&self.library)
            .folder_of(name)
            .map(Path::to_owned)
            .ok_or_else(|| format!("Unknown presence: {name}"))?;
        let author = self.default_author();
        let folder = templates::copy(&self.roots[0], &source, new_name, author.as_deref())?;
        self.reload_presences();
        self.prepare_editors();
        Ok(folder.to_string_lossy().into_owned())
    }

    /// Removes a presence the user wrote, along with what is saved about it.
    ///
    /// Args:
    ///     remove: Takes the folder away; it decides where it goes.
    ///
    /// Raises:
    ///     String: The presence is not the user's or the folder cannot be
    ///         removed. Nothing changes in that case.
    pub fn delete_presence(
        &self,
        name: &str,
        remove: impl FnOnce(&Path) -> Result<(), String>,
    ) -> Result<(), String> {
        let folder = self.local_folder(name)?;
        // The watcher reads the folder, and it would hold on to a name that is gone.
        let watched = lock(&self.watched)
            .as_ref()
            .is_some_and(|watched| watched.folder == folder);
        if watched {
            self.stop_watching();
        }
        remove(&folder)?;
        {
            let mut settings = lock(&self.settings);
            let mut changed = settings.clone();
            changed.disabled.remove(name);
            changed.favorites.remove(name);
            changed.options.remove(name);
            changed.preferences.presences.remove(name);
            // The folder is gone by now, so a failed save is not worth an error.
            match self.store.save(&changed) {
                Ok(()) => *settings = changed,
                Err(error) => tracing::warn!("cannot forget a deleted presence: {error}"),
            }
        }
        tracing::info!(presence = %name, "deleted a presence");
        self.reload_presences();
        Ok(())
    }

    /// Fails when any loaded presence already has this name.
    fn ensure_free(&self, name: &str) -> Result<(), String> {
        let wanted = identity(name);
        let taken = lock(&self.library)
            .presences
            .iter()
            .any(|presence| identity(&presence.manifest.name) == wanted);
        if taken {
            return Err(format!("A presence named {} already exists.", name.trim()));
        }
        Ok(())
    }

    /// Fails when a presence the user wrote has this name.
    pub(crate) fn ensure_not_local(&self, name: &str) -> Result<(), String> {
        let wanted = identity(name);
        let library = lock(&self.library);
        let taken = library.presences.iter().any(|presence| {
            let other = &presence.manifest.name;
            identity(other) == wanted && library.origin_of(other) == Some(Origin::Local)
        });
        if taken {
            return Err(format!(
                "A presence of yours is already called {}. Rename it to install this one.",
                name.trim()
            ));
        }
        Ok(())
    }

    /// The folder of a presence the user may modify.
    ///
    /// Raises:
    ///     String: The presence does not exist or did not come from the user.
    pub(crate) fn local_folder(&self, name: &str) -> Result<PathBuf, String> {
        let library = lock(&self.library);
        let folder = library
            .folder_of(name)
            .ok_or_else(|| format!("Unknown presence: {name}"))?;
        if library.origin_of(name) != Some(Origin::Local) {
            return Err(NOT_LOCAL.into());
        }
        Ok(folder.to_owned())
    }

    /// The folder of a presence, for opening it in the file manager.
    pub fn presence_folder(&self, name: &str) -> Result<PathBuf, String> {
        lock(&self.library)
            .folder_of(name)
            .map(Path::to_owned)
            .ok_or_else(|| format!("Unknown presence: {name}"))
    }

    /// Runs a presence once and says what it read and what it would show.
    pub fn inspect(&self, name: &str) -> Result<Inspection, String> {
        let prepared = {
            let settings = lock(&self.settings);
            let library = lock(&self.library);
            let language = effective_language(&settings, &self.system_language);
            let presence = library
                .find(name)
                .ok_or_else(|| format!("Unknown presence: {name}"))?;
            library.prepared(presence, &settings, &language)
        };
        let pages = (self.bridge.page_source())(name);
        let context = json!({"now": now(), "options": prepared.settings, "pages": pages});

        let host = Rc::new(self.host.clone());
        let loaded = Plugin::load(prepared.manifest.clone(), &prepared.source, host)
            .and_then(|mut plugin| plugin.set_strings(&prepared.strings).map(|()| plugin));
        let mut plugin = match loaded {
            Ok(plugin) => plugin,
            Err(error) => return Ok(Inspection::failed(error.to_string(), Vec::new())),
        };
        let polled = plugin.poll(&context);
        let console = plugin.take_console();
        Ok(match polled {
            Ok(tick) => Inspection {
                snapshot: tick
                    .snapshot
                    .and_then(|text| serde_json::from_str(&text).ok()),
                activity: tick.activity,
                meta: tick.meta,
                error: None,
                console,
            },
            Err(error) => Inspection::failed(error.to_string(), console),
        })
    }

    /// Starts watching a presence in the user's folder, or stops watching.
    ///
    /// Raises:
    ///     String: The presence does not exist or the user did not write it.
    pub fn watch(self: &Arc<Self>, name: Option<&str>) -> Result<(), String>
    where
        H: Sync,
    {
        let started = match name {
            None => None,
            Some(name) => {
                let folder = self.local_folder(name)?;
                let weak: Weak<Self> = Arc::downgrade(self);
                let key = folder.clone();
                let watcher = Watcher::start(folder.clone(), move || {
                    weak.upgrade().is_some_and(|controller| {
                        controller.hot_reload(&key);
                        true
                    })
                });
                *lock(&self.reloads) = Reloads::default();
                Some(Watched {
                    folder,
                    _watcher: watcher,
                })
            }
        };
        // Replacing joins the old watcher's thread, so it happens outside the lock.
        let previous = std::mem::replace(&mut *lock(&self.watched), started);
        drop(previous);
        Ok(())
    }

    /// Gives every presence the person owns what a code editor reads.
    pub fn prepare_editors(&self) {
        let owned: Vec<_> = {
            let library = lock(&self.library);
            library
                .presences
                .iter()
                .filter_map(|presence| {
                    let name = &presence.manifest.name;
                    (library.origin_of(name) == Some(Origin::Local))
                        .then(|| library.folder_of(name))
                        .flatten()
                        .map(|folder| (folder.to_owned(), presence.manifest.clone()))
                })
                .collect()
        };
        for (folder, manifest) in owned {
            if let Err(error) = editor::prepare(&folder, &manifest) {
                tracing::warn!(presence = %manifest.name, "cannot write the editor's files: {error}");
            }
        }
        // These writes are the application's own, so the next look has nothing to find.
        if let Some(root) = self.roots.first() {
            *lock(&self.folders_seen) = crate::folders::state_of(root);
        }
    }

    /// Stops watching a folder that no longer exists.
    pub(crate) fn forget_missing_watch(&self) {
        let gone = {
            let mut watched = lock(&self.watched);
            if watched
                .as_ref()
                .is_some_and(|watched| !watched.folder.is_dir())
            {
                watched.take()
            } else {
                None
            }
        };
        if gone.is_some() {
            *lock(&self.reloads) = Reloads::default();
        }
        // Joins the watcher's thread, so it happens outside the lock.
        drop(gone);
    }

    /// Stops watching; the application is closing.
    pub(crate) fn stop_watching(&self) {
        let previous = lock(&self.watched).take();
        drop(previous);
    }

    /// Changes the descriptive fields of a presence in the user's folder.
    ///
    /// Raises:
    ///     String: The user did not write the presence, the result is
    ///         not a valid manifest or the file cannot be written.
    pub fn update_manifest(&self, name: &str, changes: &ManifestChanges) -> Result<(), String> {
        let folder = self.local_folder(name)?;
        manifest_edit::apply(&folder, changes)?;
        self.read_changed(&folder)
    }

    /// What is being watched and how the last reload went.
    pub fn dev_status(&self) -> DevStatus {
        let folder = lock(&self.watched)
            .as_ref()
            .map(|watched| watched.folder.clone());
        let watching = folder
            .as_ref()
            .and_then(|folder| lock(&self.library).name_at(folder).map(str::to_owned));
        let reloads = lock(&self.reloads);
        DevStatus {
            watching,
            folder: folder.map(|folder| folder.to_string_lossy().into_owned()),
            last_reload: reloads.at,
            error: reloads.error.clone(),
        }
    }

    /// Applies an edit of a watched folder.
    pub(crate) fn hot_reload(&self, folder: &Path) {
        match self.read_changed(folder) {
            Ok(()) => {
                *lock(&self.reloads) = Reloads {
                    at: Some(now()),
                    error: None,
                }
            }
            Err(message) => {
                tracing::warn!("a watched presence did not load: {message}");
                lock(&self.reloads).error = Some(message);
            }
        }
    }

    pub(crate) fn read_changed(&self, folder: &Path) -> Result<(), String> {
        // A folder that stopped being the user's own is never reloaded in place.
        let local = {
            let library = lock(&self.library);
            library
                .name_at(folder)
                .is_some_and(|name| library.origin_of(name) == Some(Origin::Local))
        };
        if !local {
            return Err(NOT_LOCAL.into());
        }
        // An edited catalog copy is refused here too; the reviewed version keeps running.
        if let Some(message) = self.catalog.as_deref().and_then(|c| c.refusal(folder)) {
            return Err(message);
        }
        let found = load_presence(folder)?;
        // Compile it before it replaces anything: a typo must not stop the
        // version that works.
        let host = Rc::new(self.host.clone());
        Plugin::load(
            found.presence.manifest.clone(),
            &found.presence.source,
            host,
        )
        .map_err(|e| e.to_string())?;

        // The editor follows the manifest. It is help, so failing here stops nothing.
        if let Err(error) = editor::refresh_types(folder, &found.presence.manifest) {
            tracing::warn!("cannot write the editor's types: {error}");
        }
        let new_name = found.presence.manifest.name.clone();
        let (rules, start, old_name) = {
            let settings = lock(&self.settings);
            let mut library = lock(&self.library);
            let old_name = library.name_at(folder).map(str::to_owned);
            library.replace(folder, found)?;
            let language = effective_language(&settings, &self.system_language);
            let rules = rpp_bridge::Rule::from_manifests(
                library
                    .enabled(&settings, &language)
                    .iter()
                    .map(|presence| &presence.manifest),
            );
            let start = library
                .find(&new_name)
                .filter(|_| !settings.disabled.contains(&new_name))
                .map(|presence| library.prepared(presence, &settings, &language));
            (rules, start, old_name)
        };
        self.bridge.set_rules(rules);
        if let Some(old) = old_name.filter(|old| *old != new_name) {
            self.service.stop_presence(&old);
        }
        match start {
            Some(presence) => self.service.start_presence(presence),
            None => {
                self.service.stop_presence(&new_name);
            }
        }
        Ok(())
    }
}
