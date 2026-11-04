//! A local CLI handoff, never an invocation bundle or a command the host executes.
use super::{Result, registration::Descriptor};
use crate::library::{assets::AssetStore, attempts::Attempt};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf};
pub const ARGUMENTS_FILE: &str = "wafflery-arguments.json";
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Launch {
    pub executable: Option<String>,
    pub cwd: Option<String>,
    pub config: Option<String>,
    pub unresolved_arguments: bool,
}
/// Never read configuration or retain unknown argv values, which might contain credentials.
pub fn launch_context(descriptor: &Descriptor) -> Launch {
    let mut config = descriptor
        .config
        .as_ref()
        .and_then(|path| path.to_str())
        .map(str::to_owned);
    let mut unresolved = descriptor.args.first().is_none_or(|arg| arg != "serve");
    let mut args = descriptor.args.iter().skip(1);
    while let Some(arg) = args.next() {
        let value = if arg == "--config" {
            args.next().map(String::as_str)
        } else {
            arg.strip_prefix("--config=")
        };
        match value {
            Some(value) if !value.is_empty() && !value.starts_with('-') && config.is_none() => {
                let path = PathBuf::from(value);
                let path = if path.is_absolute() {
                    path
                } else {
                    descriptor.cwd.join(path)
                };
                config = path.to_str().map(str::to_owned);
                unresolved |= config.is_none();
            }
            _ => unresolved = true,
        }
    }
    let executable = descriptor.executable.to_str().map(str::to_owned);
    let cwd = descriptor.cwd.to_str().map(str::to_owned);
    unresolved |=
        executable.is_none() || cwd.is_none() || (descriptor.config.is_some() && config.is_none());
    if unresolved && descriptor.config.is_none() {
        config = None;
    }
    Launch {
        executable,
        cwd,
        config,
        unresolved_arguments: unresolved,
    }
}
#[derive(Debug, Serialize)]
pub struct Handoff {
    pub state: &'static str,
    pub blockers: Vec<String>,
    pub invocation: Option<Value>,
    pub arguments: Option<Value>,
    pub arguments_filename: &'static str,
    pub argv: Option<Vec<String>>,
    pub command: Option<String>,
    pub cwd: Option<String>,
    pub allowed_dirs: Vec<String>,
    pub requirements: Vec<String>,
}
impl Default for Handoff {
    fn default() -> Self {
        Self {
            state: "unavailable",
            blockers: vec![],
            invocation: None,
            arguments: None,
            arguments_filename: ARGUMENTS_FILE,
            argv: None,
            command: None,
            cwd: None,
            allowed_dirs: vec![],
            requirements: vec![],
        }
    }
}
/// A word as the shell reads it: bare when it holds only characters the shell leaves alone,
/// otherwise single-quoted.
pub fn shell_word(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/._-=:@+,%".contains(&b))
    {
        value.to_owned()
    } else {
        shell_quote(value)
    }
}
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
pub async fn build(assets: &AssetStore, attempt: &Attempt) -> Result<Handoff> {
    let _storage = assets.storage.read().await;
    let mut handoff = Handoff::default();
    let Some(result) = attempt
        .result
        .as_ref()
        .filter(|result| result["contract"] == "0.4.0")
    else {
        handoff
            .blockers
            .push("This run hasn't finished, so its final settings aren't known yet.".into());
        return Ok(handoff);
    };
    let invocation = &result["invocation"];
    if invocation.is_null() {
        handoff
            .blockers
            .push("The tool refused this run's inputs, so there is nothing to repeat.".into());
        return Ok(handoff);
    }
    if attempt.declaration["document"]["contract"] != "0.4.0"
        || !super::validator(&json!({"$ref":"urn:table-manners:core:result#/$defs/invocation"}))?
            .is_valid(invocation)
        || invocation["action"] != attempt.request.action
        || result["action"] != attempt.request.action
    {
        handoff
            .blockers
            .push("The settings saved with this run are missing or don't match it.".into());
        return Ok(handoff);
    }
    handoff.invocation = Some(invocation.clone());
    handoff.requirements = invocation["requirements"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut arguments = invocation["arguments"].clone();
    let action = attempt.declaration["document"]["actions"]
        .as_array()
        .and_then(|actions| {
            actions
                .iter()
                .find(|action| action["name"] == attempt.request.action)
        });
    let mut directories = BTreeSet::new();
    if let Some(action) = action {
        let ports = if action["kind"] == "operation" {
            &action["ports"]["in"]
        } else {
            &action["images"]
        };
        for port in ports.as_array().into_iter().flatten() {
            if action["kind"] == "operation" && port["type"] != "image" {
                continue;
            }
            let name = if action["kind"] == "operation" {
                port["name"].as_str()
            } else {
                port["slot"].as_str()
            };
            let Some(name) = name else {
                handoff
                    .blockers
                    .push("An image input has no declared name.".into());
                continue;
            };
            let Some(value) = arguments.get_mut(name) else {
                if port["required"] == true {
                    handoff
                        .blockers
                        .push(format!("Required image input {name} is missing."));
                }
                continue;
            };
            if port["many"] == true {
                if let Some(items) = value.as_array_mut() {
                    for (index, item) in items.iter_mut().enumerate() {
                        materialize_image(
                            assets,
                            attempt,
                            name,
                            index,
                            item,
                            &mut directories,
                            &mut handoff.blockers,
                        )
                        .await;
                    }
                } else {
                    handoff
                        .blockers
                        .push(format!("Image input {name} is not a list."));
                }
            } else {
                materialize_image(
                    assets,
                    attempt,
                    name,
                    0,
                    value,
                    &mut directories,
                    &mut handoff.blockers,
                )
                .await;
            }
        }
    } else {
        handoff
            .blockers
            .push("The saved action description is unavailable.".into());
    }
    handoff.arguments = Some(arguments);
    let launch = serde_json::from_value::<Launch>(attempt.observations["host_launch"].clone()).ok();
    let Some(launch) = launch else {
        handoff.blockers.push("Wafflery didn't record how it started the tool for this run. The settings are still in Run details, under Advanced.".into());
        return Ok(handoff);
    };
    handoff.cwd = launch.cwd.clone();
    if launch.unresolved_arguments {
        handoff.blockers.push("Wafflery started the tool with its own storage settings, and the command can't include those yet.".into());
    }
    let executable = launch.executable.filter(|path| {
        std::path::Path::new(path).is_absolute() && std::path::Path::new(path).is_file()
    });
    let cwd = launch.cwd.filter(|path| {
        std::path::Path::new(path).is_absolute() && std::path::Path::new(path).is_dir()
    });
    if executable.is_none() || cwd.is_none() {
        handoff
            .blockers
            .push("The original executable or working directory is unavailable.".into());
    }
    if let Some(cwd) = &cwd {
        directories.insert(cwd.clone());
    }
    handoff.allowed_dirs = directories.iter().cloned().collect();
    if !handoff.blockers.is_empty() {
        return Ok(handoff);
    }
    let mut argv = vec![
        executable.unwrap(),
        "call".into(),
        attempt.request.action.clone(),
        "--arguments".into(),
        PathBuf::from(cwd.as_ref().unwrap())
            .join(ARGUMENTS_FILE)
            .to_string_lossy()
            .into_owned(),
    ];
    if let Some(config) = launch.config {
        argv.extend(["--config".into(), config]);
    }
    for directory in directories {
        argv.extend(["--allow-dir".into(), directory]);
    }
    // The command a person pastes carries the arguments inline (`--arguments -` and a quoted
    // here-document), so nothing has to be downloaded and moved first; `argv` keeps the file
    // form for scripts. One flag per line, words quoted only where the shell needs it.
    let mut lines = vec![format!(
        "{} call {} --arguments -",
        shell_word(&argv[0]),
        shell_word(&attempt.request.action)
    )];
    for pair in argv[5..].chunks(2) {
        lines.push(
            pair.iter()
                .map(|arg| shell_word(arg))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    let inline = serde_json::to_string_pretty(handoff.arguments.as_ref().unwrap_or(&Value::Null))
        .unwrap_or_else(|_| "null".into());
    handoff.command = Some(format!(
        "cd {} && \\\n  {} <<'WAFFLERY_ARGUMENTS'\n{}\nWAFFLERY_ARGUMENTS",
        shell_word(&cwd.unwrap()),
        lines.join(" \\\n    "),
        inline
    ));
    handoff.argv = Some(argv);
    handoff.state = "local";
    Ok(handoff)
}
async fn materialize_image(
    assets: &AssetStore,
    attempt: &Attempt,
    port: &str,
    index: usize,
    value: &mut Value,
    directories: &mut BTreeSet<String>,
    blockers: &mut Vec<String>,
) {
    // An image artifact reference depends on the tool store. A preview is never its input bytes.
    if value["type"] == "image" && value["resource"].is_string() {
        let valid = super::validator(&json!({"$ref":"urn:table-manners:operations:artifact-ref"}))
            .is_ok_and(|schema| schema.is_valid(value));
        if !valid {
            blockers.push(format!(
                "Invalid saved image artifact reference for {port} item {}.",
                index + 1
            ));
        }
        return;
    }
    if !value
        .as_object()
        .is_some_and(|object| object.len() == 1 && object.contains_key("expect"))
    {
        blockers.push(format!(
            "The resolved image input for {port} item {} contains unexpected locators or fields.",
            index + 1
        ));
        return;
    }
    let saved = attempt.inputs[port]
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item["argument_index"].as_u64() == Some(index as u64))
        })
        .unwrap_or(&Value::Null);
    let Some(id) = saved["asset"]["id"].as_str() else {
        blockers.push(format!(
            "Owned image bytes are unavailable for {port} item {}.",
            index + 1
        ));
        return;
    };
    let valid_reference =
        super::validator(&json!({"$ref":"urn:table-manners:images:image-reference"}))
            .is_ok_and(|validator| validator.is_valid(&value["expect"]));
    if !valid_reference
        || ["file_sha256", "width", "height", "projection", "topology"]
            .iter()
            .any(|key| value["expect"][key] != saved["image"][key])
    {
        blockers.push(format!(
            "Saved image coordinates or fingerprint conflict for {port} item {}.",
            index + 1
        ));
        return;
    }
    match assets.read(id).await {
        Ok((asset, _))
            if value["expect"]["file_sha256"] == asset.sha256
                && value["expect"]["width"] == asset.width
                && value["expect"]["height"] == asset.height =>
        {
            let path = assets
                .root()
                .join("assets/sha256")
                .join(&asset.sha256[..2])
                .join(&asset.sha256);
            let Some(path) = path.to_str() else {
                blockers.push("An owned image path cannot be represented in JSON.".into());
                return;
            };
            directories.insert(
                assets
                    .root()
                    .join("assets/sha256")
                    .to_string_lossy()
                    .into_owned(),
            );
            value["path"] = json!(path);
        }
        _ => blockers.push(format!(
            "Owned image bytes are missing or corrupt for {port} item {}.",
            index + 1
        )),
    }
}
