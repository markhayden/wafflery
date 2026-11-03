//! Startup-only fixture. Reads synthetic configuration and reports names/presence, never values.
use serde_json::{Value, json};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub struct Configuration {
    root: PathBuf,
    path: PathBuf,
    settings: Value,
    scenario: String,
    session: Value,
}
impl Configuration {
    pub fn load(
        root: &Path,
        scenario: &str,
        arguments: Vec<String>,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        if !scenario.starts_with("configuration") {
            return Ok(None);
        }
        let path = match arguments.as_slice() {
            [] => std::env::var_os("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from(std::env::var_os("HOME").unwrap()).join(".config"))
                .join("sample-lab.json"),
            [flag, path] if flag == "--config" => PathBuf::from(path),
            _ => return Err("invalid fixture config arguments".into()),
        };
        let settings = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Value::Null,
            Err(error) => return Err(error.into()),
        };
        Ok(Some(Self {
            root: root.into(),
            path,
            settings,
            scenario: scenario.into(),
            session: Value::Null,
        }))
    }
    fn requirements(&self) -> Vec<Value> {
        let mut requirements = vec![
            json!({"kind":"config","name":self.path,"state":if self.settings.is_null() {"missing"} else {"present"}}),
        ];
        for (kind, field, fallback) in [
            ("credential", "credential", "SCRIPTED_KEY"),
            ("env", "env", "SCRIPTED_MODEL_DIR"),
        ] {
            let name = self.settings[field].as_str().unwrap_or(fallback);
            requirements.push(json!({"kind":kind,"name":name,"state":if std::env::var_os(name).is_some() {"present"} else {"missing"}}));
        }
        if self.settings["extra_when_configured"] == true && requirements[1]["state"] == "present" {
            requirements.push(json!({"kind":"credential","name":"SCRIPTED_EXTRA_KEY","state":if std::env::var_os("SCRIPTED_EXTRA_KEY").is_some() {"present"} else {"missing"}}));
        }
        requirements
    }
    pub fn describe(&self, mut document: Value) -> Value {
        let requirements = self.requirements();
        document["actions"][0]["readiness"] = json!({"state":if requirements.iter().any(|r| r["state"] == "missing") {"needs_setup"} else {"ready"},"requirements":requirements});
        if self.scenario == "configuration-fails" {
            document["contract"] = json!("unsupported");
        }
        document
    }
    pub fn started(&mut self, session: Value) -> std::io::Result<()> {
        self.session = session;
        self.record("start")
    }
    fn record(&self, event: &str) -> std::io::Result<()> {
        let present: Vec<_> = [
            "SCRIPTED_KEY",
            "SCRIPTED_MODEL_DIR",
            "SCRIPTED_EXTRA_KEY",
            "SCRIPTED_UNRELATED",
        ]
        .into_iter()
        .filter(|name| std::env::var_os(name).is_some())
        .collect();
        writeln!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.root.join("startup.ndjson"))?,
            "{}",
            json!({"event":event,"session":self.session,"present":present})
        )
    }
}
impl Drop for Configuration {
    fn drop(&mut self) {
        if !self.session.is_null() {
            let _ = self.record("exit");
        }
    }
}

/// Optional scripted paid-peer config. Resolve once; later edits require a new child.
/// Requested directories are the already-validated canonical launch grants.
pub fn startup_access(
    arguments: &[String],
    requested: &[PathBuf],
) -> Result<(Vec<PathBuf>, String), Box<dyn std::error::Error>> {
    use sha2::{Digest, Sha256};
    let path = match arguments {
        [] => return Ok((requested.to_vec(), format!("sha256:{}", "1".repeat(64)))),
        [flag, path] if flag == "--config" => Path::new(path),
        _ => return Err("invalid scripted access configuration arguments".into()),
    };
    let settings = fs::read(path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let configured = settings
        .as_ref()
        .filter(|value| value.is_object())
        .and_then(|value| match value.get("allowed_dirs") {
            None => Some(requested.to_vec()),
            Some(Value::Array(paths)) => paths
                .iter()
                .map(|value| {
                    let path = Path::new(value.as_str()?);
                    if !path.is_absolute() {
                        return None;
                    }
                    let canonical = path.canonicalize().ok()?;
                    canonical.is_dir().then_some(canonical)
                })
                .collect::<Option<Vec<_>>>(),
            _ => None,
        })
        .unwrap_or_default();
    let mut overlap = std::collections::BTreeSet::new();
    for configured in configured {
        for requested in requested {
            if configured.starts_with(requested) {
                overlap.insert(configured.clone());
            } else if requested.starts_with(&configured) {
                overlap.insert(requested.clone());
            }
        }
    }
    let directories: Vec<_> = overlap.into_iter().collect();
    let resolved = json!({"model":"offline-script","cap_usd":1.0,"budget_usd":0.2,
        "configured":true,"allowed_dirs":directories});
    let digest: String = Sha256::digest(serde_json::to_vec(&resolved)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let identity = format!("sha256:{digest}");
    Ok((directories, identity))
}

/// Deliberately malformed wire observations for independent host-boundary tests.
/// This never changes the peer's actual grants or ledger.
pub fn session_response(root: &Path, channel: &str, actual: &Value) -> Value {
    let mut response = actual.clone();
    if let Ok(bytes) = fs::read(root.join(format!("{channel}-grants.json"))) {
        response["allowed_dirs"] = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    }
    response
}

#[cfg(test)]
mod access_tests {
    use super::*;

    #[test]
    fn startup_access_intersects_configuration_and_grants_and_changes_identity() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let input = root.join("inputs");
        let narrow = input.join("selected");
        fs::create_dir_all(&narrow).unwrap();
        let config = root.join("access.json");
        fs::write(&config, json!({"allowed_dirs":[narrow]}).to_string()).unwrap();
        let args = vec!["--config".into(), config.display().to_string()];
        let (actual, identity) = startup_access(&args, std::slice::from_ref(&input)).unwrap();
        assert_eq!(actual, vec![narrow.clone()]);
        fs::write(&config, json!({"allowed_dirs":[root]}).to_string()).unwrap();
        let (replacement, changed_identity) =
            startup_access(&args, std::slice::from_ref(&input)).unwrap();
        assert_eq!(replacement, vec![input.clone()]);
        assert_ne!(identity, changed_identity);
        assert_eq!(startup_access(&args, &[input]).unwrap().1, changed_identity);
    }

    #[test]
    fn denied_or_unloadable_configuration_has_no_input_access() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let config = root.join("access.json");
        let args = vec!["--config".into(), config.display().to_string()];
        assert!(
            startup_access(&args, std::slice::from_ref(&root))
                .unwrap()
                .0
                .is_empty()
        );
        fs::write(&config, "{}").unwrap();
        assert_eq!(
            startup_access(&args, std::slice::from_ref(&root))
                .unwrap()
                .0,
            vec![root.clone()]
        );
        for document in [
            Value::Null,
            json!([]),
            json!("not an object"),
            json!({"allowed_dirs":[]}),
            json!({"allowed_dirs":null}),
            json!({"allowed_dirs":[root,"relative"]}),
            json!({"allowed_dirs":[config]}),
            json!({"allowed_dirs":["relative"]}),
            json!({"allowed_dirs":[root.join("missing")]}),
            json!({"allowed_dirs":42}),
        ] {
            fs::write(&config, document.to_string()).unwrap();
            assert!(
                startup_access(&args, std::slice::from_ref(&root))
                    .unwrap()
                    .0
                    .is_empty()
            );
        }
        fs::write(&config, "not JSON").unwrap();
        assert!(startup_access(&args, &[root]).unwrap().0.is_empty());
    }

    #[test]
    fn startup_access_is_canonical_deduplicated_and_never_adds_an_ungranted_path() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let input = root.join("inputs");
        let outside = root.join("outside");
        fs::create_dir(&input).unwrap();
        fs::create_dir(&outside).unwrap();
        let alias = root.join("alias");
        std::os::unix::fs::symlink(&input, &alias).unwrap();
        let config = root.join("access.json");
        fs::write(
            &config,
            json!({"allowed_dirs":[alias,input,outside]}).to_string(),
        )
        .unwrap();
        let args = vec!["--config".into(), config.display().to_string()];
        assert_eq!(
            startup_access(&args, std::slice::from_ref(&input))
                .unwrap()
                .0,
            vec![input.clone()]
        );
        assert!(startup_access(&args, &[]).unwrap().0.is_empty());
        assert_eq!(
            startup_access(&[], std::slice::from_ref(&input)).unwrap().0,
            vec![input]
        );
        assert!(startup_access(&["--unknown".into()], &[]).is_err());
    }
}
