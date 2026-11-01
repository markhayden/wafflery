#[allow(dead_code)]
#[path = "../build.rs"]
mod build_script;

use std::{
    fs,
    path::Path,
    sync::atomic::{AtomicU64, Ordering},
};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn fixture(root: &Path) {
    use sha2::{Digest, Sha256};
    let input_paths = [
        "ui/src/main.ts",
        "ui/index.html",
        "ui/package.json",
        "ui/bun.lock",
        "ui/tsconfig.json",
        "ui/vite.config.ts",
        "scripts/build-ui.ts",
        "scripts/check-ui.ts",
        "scripts/ui-manifest.ts",
    ];
    let output_paths = ["index.html", "assets/app.js"];
    let hash: String = Sha256::digest(b"fixture")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let mut inputs = serde_json::Map::new();
    let mut outputs = serde_json::Map::new();
    for path in input_paths {
        let target = root.join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, "fixture").unwrap();
        inputs.insert(path.into(), hash.clone().into());
    }
    for path in output_paths {
        let target = root.join("ui/prebuilt").join(path);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, "fixture").unwrap();
        outputs.insert(path.into(), hash.clone().into());
    }
    fs::write(
        root.join("ui/prebuilt/manifest.json"),
        serde_json::to_vec(
            &serde_json::json!({"version": 1, "inputs": inputs, "outputs": outputs}),
        )
        .unwrap(),
    )
    .unwrap();
}

#[test]
fn embedded_index_and_assets_are_available_without_a_filesystem_lookup() {
    let index = std::str::from_utf8(wafflery::host::assets::get("/").unwrap()).unwrap();
    assert!(index.contains("<title>Wafflery</title>"));
    assert!(wafflery::host::assets::get("/missing.js").is_none());
    assert!(wafflery::host::assets::get("/../Cargo.toml").is_none());
}

#[test]
fn stale_inputs_warn_in_debug_but_fail_strict_builds_and_missing_outputs_always_fail() {
    let root = std::env::temp_dir().join(format!(
        "wafflery-build-test-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fixture(&root);
    let verified = build_script::verify(&root).unwrap();
    assert!(!verified.stale);
    fs::write(root.join("ui/src/new.ts"), "export const changed = true;").unwrap();
    assert!(build_script::verify(&root).unwrap().stale);
    assert!(build_script::enforce_freshness(true, false).is_ok());
    assert!(build_script::enforce_freshness(true, true).is_err());
    fs::remove_file(root.join("ui/src/new.ts")).unwrap();
    fs::write(root.join("ui/prebuilt/index.html"), "changed output").unwrap();
    assert!(build_script::verify(&root).unwrap().stale);
    let asset = verified
        .paths
        .iter()
        .find(|path| path.starts_with("assets/"))
        .unwrap();
    fs::remove_file(root.join("ui/prebuilt").join(asset)).unwrap();
    assert!(build_script::verify(&root).is_err());
    fs::remove_dir_all(root).unwrap();
}
