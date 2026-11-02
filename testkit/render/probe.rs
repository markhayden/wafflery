//! Adversarial report fixture, independent of host code and credentials.
pub fn html(workspace: &str, target: &str) -> String {
    include_str!("probe.html")
        .replace("__WORKSPACE__", &serde_json::to_string(workspace).unwrap())
        .replace("__TARGET__", &serde_json::to_string(target).unwrap())
}
