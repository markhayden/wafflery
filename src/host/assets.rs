//! Committed UI bytes embedded at build time; no JS runtime required.

include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));

pub fn get(path: &str) -> Option<&'static [u8]> {
    let path = if path == "/" { "/index.html" } else { path };
    ASSETS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| *bytes)
}
