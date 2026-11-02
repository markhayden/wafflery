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
