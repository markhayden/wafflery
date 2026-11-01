use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs, path::Path};

pub struct VerifiedUi {
    pub paths: Vec<String>,
    pub stale: bool,
}

fn regular_files(root: &Path, prefix: &str) -> Result<Vec<String>, String> {
    let mut paths = Vec::new();
    let entries = fs::read_dir(root.join(prefix)).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "Non-UTF8 UI path")?;
        let path = if prefix.is_empty() {
            name
        } else {
            format!("{prefix}/{name}")
        };
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_symlink() {
            return Err(format!("UI paths must not be symlinks: {path}"));
        }
        if kind.is_dir() {
            paths.extend(regular_files(root, &path)?);
        } else if kind.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn hash_files(root: &Path, paths: &[String]) -> Result<BTreeMap<String, String>, String> {
    paths
        .iter()
        .map(|path| {
            let bytes = fs::read(root.join(path)).map_err(|e| format!("{path}: {e}"))?;
            Ok((
                path.clone(),
                Sha256::digest(bytes)
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect(),
            ))
        })
        .collect()
}

pub fn verify(root: &Path) -> Result<VerifiedUi, String> {
    let prebuilt = root.join("ui/prebuilt");
    if fs::symlink_metadata(&prebuilt)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
    {
        return Err("Prebuilt directory must not be a symlink".into());
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(prebuilt.join("manifest.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if manifest["version"] != 1 {
        return Err("Unsupported UI manifest version".into());
    }
    let expected_inputs: BTreeMap<String, String> =
        serde_json::from_value(manifest["inputs"].clone()).map_err(|e| e.to_string())?;
    let expected_outputs: BTreeMap<String, String> =
        serde_json::from_value(manifest["outputs"].clone()).map_err(|e| e.to_string())?;
    let paths: Vec<_> = regular_files(&prebuilt, "")?
        .into_iter()
        .filter(|path| path != "manifest.json")
        .collect();
    if !paths.iter().any(|path| path == "index.html") {
        return Err("Missing prebuilt index.html; run bun run scripts/build-ui.ts".into());
    }
    if expected_outputs.keys().any(|path| !paths.contains(path)) {
        return Err("Missing prebuilt asset; run bun run scripts/build-ui.ts".into());
    }
    let mut input_paths = regular_files(root, "ui/src")?;
    input_paths.extend(
        [
            "ui/index.html",
            "ui/package.json",
            "ui/bun.lock",
            "ui/tsconfig.json",
            "ui/vite.config.ts",
            "scripts/build-ui.ts",
            "scripts/check-ui.ts",
            "scripts/ui-manifest.ts",
        ]
        .map(str::to_owned),
    );
    let stale = hash_files(&prebuilt, &paths)? != expected_outputs
        || hash_files(root, &input_paths)? != expected_inputs;
    Ok(VerifiedUi { paths, stale })
}

pub fn enforce_freshness(stale: bool, strict: bool) -> Result<(), &'static str> {
    if stale && strict {
        Err("Prebuilt UI is stale; run bun run scripts/build-ui.ts")
    } else {
        Ok(())
    }
}

fn main() {
    for path in [
        "ui/src",
        "ui/prebuilt",
        "ui/index.html",
        "ui/package.json",
        "ui/bun.lock",
        "ui/tsconfig.json",
        "ui/vite.config.ts",
        "scripts/build-ui.ts",
        "scripts/check-ui.ts",
        "scripts/ui-manifest.ts",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=WAFFLERY_VERIFY_UI");
    println!("cargo:rerun-if-env-changed=CI");
    let root = env::var("CARGO_MANIFEST_DIR").expect("Cargo provides package root");
    let root = Path::new(&root);
    let verified = verify(root).unwrap_or_else(|error| panic!("Cannot embed UI: {error}"));
    let strict = env::var("PROFILE").as_deref() == Ok("release")
        || env::var("WAFFLERY_VERIFY_UI").as_deref() == Ok("1")
        || env::var("CI").is_ok_and(|value| !value.is_empty() && value != "false" && value != "0");
    enforce_freshness(verified.stale, strict).expect("UI freshness required for release/CI");
    if verified.stale {
        println!(
            "cargo:warning=Prebuilt UI is stale; use Vite for development or bun run scripts/build-ui.ts"
        );
    }
    let mut code = String::from("pub static ASSETS: &[(&str, &[u8])] = &[\n");
    for path in verified.paths {
        code.push_str(&format!(
            "({:?}, include_bytes!({:?})),\n",
            format!("/{path}"),
            root.join("ui/prebuilt").join(path)
        ));
    }
    code.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").expect("Cargo provides output directory"))
            .join("ui_assets.rs"),
        code,
    )
    .expect("write embedded UI table");
}
