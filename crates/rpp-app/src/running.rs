//! Making the running presences match what the settings and folders say.

use rpp_bridge::Rule;
use rpp_config::Problem;
use rpp_plugins::Host;

use crate::controller::{Controller, lock};
use crate::library::{Library, effective_language};

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Reads the presence folders again and applies what changed.
    pub fn reload_presences(&self) -> Vec<Problem> {
        // Taken before reading: a change while it reads is seen at the next look.
        if let Some(root) = self.roots.first() {
            *lock(&self.folders_seen) = crate::folders::state_of(root);
        }
        let fresh = Library::load(&self.roots, self.catalog.as_deref());
        let problems = fresh.problems.clone();
        let restart = {
            let settings = lock(&self.settings);
            let mut library = lock(&self.library);
            let changed = library.changed_since(&fresh);
            *library = fresh;
            let language = effective_language(&settings, &self.system_language);
            changed
                .iter()
                .filter(|name| settings.enabled.contains(*name))
                .filter_map(|name| {
                    let presence = library.find(name)?;
                    Some(library.prepared(presence, &settings, &language))
                })
                .collect::<Vec<_>>()
        };
        self.sync_running();
        for presence in restart {
            self.service.start_presence(presence);
        }
        self.forget_missing_watch();
        problems
    }

    /// Starts the presences that should run and stops the ones that should not.
    pub(crate) fn sync_running(&self) {
        // Locks are taken in one order, settings then library, and released
        // before the service is touched: stopping a presence waits for a thread.
        let (rules, wanted) = {
            let settings = lock(&self.settings);
            let library = lock(&self.library);
            let language = effective_language(&settings, &self.system_language);
            let wanted = library.enabled(&settings, &language);
            let rules = Rule::from_manifests(wanted.iter().map(|presence| &presence.manifest));
            (rules, wanted)
        };
        self.bridge.set_rules(rules);

        let running = self.service.running();
        for name in &running {
            if !wanted
                .iter()
                .any(|presence| &presence.manifest.name == name)
            {
                self.service.stop_presence(name);
            }
        }
        for presence in wanted {
            if !running.contains(&presence.manifest.name) {
                self.service.start_presence(presence);
            }
        }
    }
}
