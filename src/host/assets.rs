//! Committed UI bytes embedded at build time; no JS runtime required.

include!(concat!(env!("OUT_DIR"), "/ui_assets.rs"));

pub fn get(path: &str) -> Option<&'static [u8]> {
    let path = if path == "/" { "/index.html" } else { path };
    ASSETS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, bytes)| *bytes)
}

/// A workspace address (`/rascal/remove/<run>`, the UI's route) rather than a file: no
/// extension in its last segment. The host answers it with the page, which reads the address.
pub fn is_page(path: &str) -> bool {
    !path.starts_with("/api/") && !path.rsplit('/').next().unwrap_or("").contains('.')
}
