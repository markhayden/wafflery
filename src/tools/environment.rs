//! Environment values live only at the child-process boundary, never in registration or evidence.
use serde_json::Value;
use std::{collections::BTreeMap, ffi::OsString};
use tokio::process::Command;

pub(super) fn apply(command: &mut Command, requirements: &[Value]) {
    command
        .env_clear()
        .envs(select(requirements, |name| std::env::var_os(name)));
}

const BASE: &[&str] = &[
    "HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "TMPDIR",
    "LANG",
    "LC_ALL",
];

fn select(
    requirements: &[Value],
    mut lookup: impl FnMut(&str) -> Option<OsString>,
) -> BTreeMap<String, OsString> {
    let mut values = BTreeMap::from([("PATH".into(), "/usr/bin:/bin:/usr/sbin:/sbin".into())]);
    for name in BASE {
        if let Some(value) = lookup(name) {
            values.insert((*name).into(), value);
        }
    }
    for requirement in requirements {
        if !matches!(requirement["kind"].as_str(), Some("credential" | "env")) {
            continue;
        }
        let Some(name) = requirement["name"]
            .as_str()
            .filter(|name| forwardable(name))
        else {
            continue;
        };
        if let Some(value) = lookup(name) {
            values.insert(name.into(), value);
        }
    }
    values
}

fn forwardable(name: &str) -> bool {
    let mut bytes = name.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_'))
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        && !BASE.contains(&name)
        && name != "PATH"
        && !["WAFFLERY_", "DYLD_", "LD_"]
            .iter()
            .any(|prefix| name.starts_with(prefix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn inherited() -> BTreeMap<String, OsString> {
        [
            ("HOME", "/synthetic/home"),
            ("XDG_CONFIG_HOME", "/synthetic/config"),
            ("XDG_DATA_HOME", "/synthetic/data"),
            ("XDG_CACHE_HOME", "/synthetic/cache"),
            ("TMPDIR", "/synthetic/tmp"),
            ("LANG", "en_US.UTF-8"),
            ("LC_ALL", "C"),
            ("PATH", "/untrusted/bin"),
            ("EXAMPLE_KEY", "synthetic-key"),
            ("MODEL_DIR", "/synthetic/models"),
            ("UNRELATED_SECRET", "synthetic-unrelated"),
        ]
        .into_iter()
        .map(|(k, v)| (k.into(), v.into()))
        .collect()
    }

    #[test]
    fn base_preserves_default_lookup_and_locale_with_a_fixed_path() {
        let source = inherited();
        let child = select(&[], |name| source.get(name).cloned());
        for name in [
            "HOME",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
            "XDG_CACHE_HOME",
            "TMPDIR",
            "LANG",
            "LC_ALL",
        ] {
            assert_eq!(child.get(name), source.get(name));
        }
        assert_eq!(child["PATH"], "/usr/bin:/bin:/usr/sbin:/sbin");
        assert_eq!(child.len(), 8);
    }

    #[test]
    fn only_declared_credential_and_env_names_are_copied() {
        let source = inherited();
        let requirements = [
            json!({"kind":"credential", "name":"EXAMPLE_KEY"}),
            json!({"kind":"env", "name":"MODEL_DIR"}),
            json!({"kind":"credential", "name":"MISSING_KEY"}),
            json!({"kind":"file", "name":"UNRELATED_SECRET"}),
        ];
        let child = select(&requirements, |name| source.get(name).cloned());
        assert_eq!(child["EXAMPLE_KEY"], "synthetic-key");
        assert_eq!(child["MODEL_DIR"], "/synthetic/models");
        assert!(!child.contains_key("MISSING_KEY"));
        assert!(!child.contains_key("UNRELATED_SECRET"));
    }

    #[test]
    fn excluded_and_non_identifier_names_are_never_looked_up() {
        let excluded = [
            "WAFFLERY_BIN",
            "WAFFLERY_TOKEN",
            "DYLD_INSERT_LIBRARIES",
            "LD_PRELOAD",
            "keychain:example",
            "not an env",
            "1KEY",
            "KEY=VALUE",
            "",
            "É_KEY",
        ];
        let mut requirements: Vec<_> = excluded
            .iter()
            .map(|name| json!({"kind":"credential", "name":name}))
            .collect();
        requirements.extend([
            json!({"kind":"env", "name":"PATH"}),
            json!({"kind":"credential", "name":"HOME"}),
        ]);
        let mut requested = Vec::new();
        let child = select(&requirements, |name| {
            assert!(!excluded.contains(&name));
            assert_ne!(name, "PATH");
            requested.push(name.to_owned());
            None
        });
        assert_eq!(requested.iter().filter(|name| *name == "HOME").count(), 1);
        assert_eq!(child.len(), 1);
        assert_eq!(child["PATH"], "/usr/bin:/bin:/usr/sbin:/sbin");
    }

    #[tokio::test]
    async fn actual_child_can_find_home_and_xdg_configuration_without_other_variables() {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home with spaces");
        let config = dir.path().join("config with spaces");
        std::fs::create_dir_all(home.join(".config")).unwrap();
        std::fs::create_dir_all(&config).unwrap();
        std::fs::write(home.join(".config/example.conf"), "synthetic").unwrap();
        std::fs::write(config.join("example.conf"), "synthetic").unwrap();
        let environment = select(&[], |name| match name {
            "HOME" => Some(home.clone().into_os_string()),
            "XDG_CONFIG_HOME" => Some(config.clone().into_os_string()),
            _ => None,
        });
        let status = Command::new("/bin/sh")
            .args(["-c", "test -f \"$HOME/.config/example.conf\" && test -f \"$XDG_CONFIG_HOME/example.conf\" && test -z \"${UNRELATED_SECRET+x}\""])
            .env("UNRELATED_SECRET", "synthetic-unrelated")
            .env_clear().envs(environment).status().await.unwrap();
        assert!(status.success());
    }
}
