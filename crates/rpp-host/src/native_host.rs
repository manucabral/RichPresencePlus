//! Registering the native messaging host with the browsers.

use std::path::Path;

use windows_registry::CURRENT_USER;

/// A browser that reads native messaging hosts from the registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    /// Google Chrome.
    Chrome,
    /// Microsoft Edge.
    Edge,
    /// Mozilla Firefox.
    Firefox,
    /// Chromium, and browsers built on it that read its key, such as Yandex.
    Chromium,
    /// Brave.
    Brave,
    /// Vivaldi.
    Vivaldi,
}

impl Browser {
    /// Every browser this module registers with.
    pub const ALL: [Self; 6] = [
        Self::Chrome,
        Self::Edge,
        Self::Firefox,
        Self::Chromium,
        Self::Brave,
        Self::Vivaldi,
    ];

    /// Key under `HKEY_CURRENT_USER` that holds this browser's hosts.
    fn key(self) -> &'static str {
        match self {
            Self::Chrome => r"Software\Google\Chrome\NativeMessagingHosts",
            Self::Edge => r"Software\Microsoft\Edge\NativeMessagingHosts",
            Self::Firefox => r"Software\Mozilla\NativeMessagingHosts",
            Self::Chromium => r"Software\Chromium\NativeMessagingHosts",
            Self::Brave => r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts",
            Self::Vivaldi => r"Software\Vivaldi\NativeMessagingHosts",
        }
    }

    /// Tells whether this browser reads the Firefox kind of host manifest.
    pub fn is_firefox(self) -> bool {
        self == Self::Firefox
    }
}

/// The registry entries of one native messaging host.
#[derive(Debug, Clone)]
pub struct NativeHosts {
    /// Prepended to every browser key; empty for the real ones.
    root: String,
    name: String,
}

impl NativeHosts {
    /// The entries the browsers read for the host `name` of the host manifest.
    pub fn system(name: &str) -> Self {
        Self {
            root: String::new(),
            name: name.to_owned(),
        }
    }

    fn path(&self, browser: Browser) -> String {
        if self.root.is_empty() {
            format!(r"{}\{}", browser.key(), self.name)
        } else {
            format!(r"{}\{}\{}", self.root, browser.key(), self.name)
        }
    }

    /// Points a browser to the host manifest at the absolute path `manifest`.
    pub fn register(&self, browser: Browser, manifest: &Path) -> Result<(), String> {
        CURRENT_USER
            .create(self.path(browser))
            .and_then(|key| key.set_string("", manifest.to_string_lossy()))
            .map_err(|e| e.to_string())
    }

    /// The manifest path a browser reads, if the entry exists.
    pub fn registered(&self, browser: Browser) -> Option<String> {
        CURRENT_USER
            .open(self.path(browser))
            .and_then(|key| key.get_string(""))
            .ok()
    }

    /// Removes a browser's entry. A missing entry is not an error.
    pub fn unregister(&self, browser: Browser) -> Result<(), String> {
        if self.registered(browser).is_none() {
            return Ok(());
        }
        CURRENT_USER
            .remove_tree(self.path(browser))
            .map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A key of its own directly under `Software`, removed whole when the test ends.
    struct Scratch(NativeHosts);

    impl Scratch {
        fn new() -> Self {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            Self(NativeHosts {
                root: format!(
                    r"Software\RichPresencePlusTest-{}-{nanos}",
                    std::process::id()
                ),
                name: "com.example.host".into(),
            })
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = CURRENT_USER.remove_tree(&self.0.root);
        }
    }

    #[test]
    fn entries_are_written_read_and_removed() {
        let scratch = Scratch::new();
        let manifest = Path::new(r"C:\Users\a\AppData\Local\rpp\native-host-chromium.json");
        for browser in Browser::ALL {
            assert_eq!(scratch.0.registered(browser), None);
        }
        scratch.0.register(Browser::Edge, manifest).unwrap();
        assert_eq!(
            scratch.0.registered(Browser::Edge).as_deref(),
            Some(manifest.to_string_lossy().as_ref())
        );
        assert_eq!(scratch.0.registered(Browser::Chrome), None);
        // Writing again replaces the path.
        let moved = Path::new(r"D:\rpp\native-host-chromium.json");
        scratch.0.register(Browser::Edge, moved).unwrap();
        assert_eq!(
            scratch.0.registered(Browser::Edge).as_deref(),
            Some(r"D:\rpp\native-host-chromium.json")
        );
        scratch.0.unregister(Browser::Edge).unwrap();
        assert_eq!(scratch.0.registered(Browser::Edge), None);
        scratch.0.unregister(Browser::Edge).unwrap();
    }

    #[test]
    fn each_browser_has_its_own_key() {
        let hosts = NativeHosts::system("com.manucabral.richpresenceplus");
        assert_eq!(
            hosts.path(Browser::Chrome),
            r"Software\Google\Chrome\NativeMessagingHosts\com.manucabral.richpresenceplus"
        );
        assert_eq!(
            hosts.path(Browser::Edge),
            r"Software\Microsoft\Edge\NativeMessagingHosts\com.manucabral.richpresenceplus"
        );
        assert_eq!(
            hosts.path(Browser::Firefox),
            r"Software\Mozilla\NativeMessagingHosts\com.manucabral.richpresenceplus"
        );
        assert_eq!(
            hosts.path(Browser::Chromium),
            r"Software\Chromium\NativeMessagingHosts\com.manucabral.richpresenceplus"
        );
        assert_eq!(
            hosts.path(Browser::Brave),
            r"Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.manucabral.richpresenceplus"
        );
        assert_eq!(
            hosts.path(Browser::Vivaldi),
            r"Software\Vivaldi\NativeMessagingHosts\com.manucabral.richpresenceplus"
        );
        assert!(Browser::Firefox.is_firefox() && !Browser::Edge.is_firefox());
    }
}
