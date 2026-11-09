//! A `manifest.json` from anyone: the catalog, a folder someone shared, the editor.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rpp_plugins::Manifest;

fuzz_target!(|text: &str| {
    let Ok(manifest) = Manifest::parse(text) else {
        return;
    };
    // What was accepted stays within the limits the rest of the application trusts.
    let _ = manifest.grants();
    if let Some(page) = &manifest.page {
        assert!(page.validate().is_ok());
        assert!(page.sites().count() <= 24);
    }
    assert!(manifest.tags.len() <= 8);
    assert!(manifest.other_names.len() <= 8);
});
