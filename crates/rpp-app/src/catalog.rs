//! Listing, installing and removing presences of the signed catalog.

use rpp_catalog::{Catalog, Error};
use rpp_plugins::{Grant, Host};

use crate::controller::Controller;
use crate::views::{Access, CatalogItem, CatalogView, InstallOutcome};

impl<H: Host + Clone + Send + 'static> Controller<H> {
    /// Lists the approved presences and what is installed from them.
    ///
    /// Raises:
    ///     String: The index cannot be downloaded or is not trusted.
    pub fn catalog(&self, refresh: bool) -> Result<CatalogView, String> {
        let Some(catalog) = self.catalog.as_ref().filter(|c| c.enabled()) else {
            return Ok(CatalogView {
                available: false,
                items: Vec::new(),
            });
        };
        let items = catalog
            .listing(refresh)
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(CatalogItem::from)
            .collect();
        Ok(CatalogView {
            available: true,
            items,
        })
    }

    /// Installs a presence from the catalog, or updates it, and starts it.
    ///
    /// Args:
    ///     accepted: New access the person said yes to; empty the first time.
    ///
    /// Returns:
    ///     `Asks` when an update reaches for more than the installed copy and
    ///     nobody accepted it; the installed copy then keeps running.
    ///
    /// Raises:
    ///     String: The catalog is unavailable, a presence of the user's already
    ///         has that name, a download fails or a file does not match the signed index.
    pub fn install_presence(
        &self,
        name: &str,
        accepted: &[Grant],
    ) -> Result<InstallOutcome, String> {
        let catalog = self.managed_catalog()?;
        self.ensure_not_local(name)?;
        let entry = match catalog.install(name, accepted) {
            Ok(entry) => entry,
            Err(Error::NewAccess(grants)) => {
                tracing::info!(presence = %name, "an update asks for more access");
                return Ok(InstallOutcome::Asks {
                    access: Access::of(&grants),
                    grants,
                });
            }
            Err(error) => {
                tracing::warn!(presence = %name, "installing from the catalog failed: {error}");
                return Err(error.to_string());
            }
        };
        tracing::info!(presence = %entry.name, version = %entry.version, "installed from the catalog");
        self.reload_presences();
        Ok(InstallOutcome::Installed)
    }

    /// Removes a presence that was installed from the catalog.
    ///
    /// Raises:
    ///     String: The catalog did not install it or it cannot be deleted.
    pub fn remove_presence(&self, name: &str) -> Result<(), String> {
        let catalog = self.managed_catalog()?;
        let removed = catalog.remove(name).map_err(|e| e.to_string())?;
        tracing::info!(presence = %removed, "removed a catalog presence");
        self.reload_presences();
        Ok(())
    }

    /// The catalog, when this build carries the keys to trust it.
    pub(crate) fn managed_catalog(&self) -> Result<&Catalog, String> {
        self.catalog
            .as_deref()
            .filter(|c| c.enabled())
            .ok_or_else(|| "The catalog is not available in this build.".to_owned())
    }
}
