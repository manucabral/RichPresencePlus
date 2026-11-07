//! Going back to the version of a catalog presence that an update replaced.

use rpp_plugins::Host;
use semver::Version;

use crate::controller::Controller;

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// The versions that can be restored, newest first.
    ///
    /// Raises:
    ///     String: The catalog is unavailable or did not install the presence.
    pub fn presence_history(&self, name: &str) -> Result<Vec<String>, String> {
        let versions = self
            .managed_catalog()?
            .history(name)
            .map_err(|e| e.to_string())?;
        Ok(versions.iter().map(ToString::to_string).collect())
    }

    /// Brings back an earlier version and runs it if the presence is on.
    ///
    /// Raises:
    ///     String: The version is not valid or not kept, or the presence is not the catalog's.
    pub fn restore_presence(&self, name: &str, version: &str) -> Result<(), String> {
        let version: Version = version
            .parse()
            .map_err(|_| format!("{version} is not a version."))?;
        let catalog = self.managed_catalog()?;
        self.ensure_not_local(name)?;
        let restored = catalog.restore(name, &version).map_err(|error| {
            tracing::warn!(presence = %name, "restoring a version failed: {error}");
            error.to_string()
        })?;
        tracing::info!(presence = %restored.name, version = %restored.version, "restored an earlier version");
        self.reload_presences();
        Ok(())
    }
}
