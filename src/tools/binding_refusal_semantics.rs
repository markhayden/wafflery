//! Context checks for schema-validated v0.4 binding refusals; messages stay opaque.
use super::{Error, Result};
use serde_json::Value;

#[derive(Clone, Copy)]
pub enum Context<'a> {
    Operation {
        action: &'a Value,
        arguments: &'a Value,
    },
    ArtifactRead,
    Export,
}

/// The refusal and declaration have already passed their published schemas.
pub fn validate(data: &Value, context: Context<'_>) -> Result<()> {
    if data["reason"] != "binding" {
        return Err(Error::Invalid("Expected a binding refusal".into()));
    }
    let entries = data["bindings"].as_array().unwrap();
    let invalid =
        || Error::Invalid("Binding refusal conflicts with the requested inputs or control".into());
    match context {
        Context::ArtifactRead | Context::Export => {
            let port = match context {
                Context::Export => Some("artifact"),
                _ => None,
            };
            if entries.len() != 1
                || entries[0]["port"].as_str() != port
                || entries[0].get("index").is_some()
                || entries[0]["problem"] != "unresolved"
            {
                return Err(invalid());
            }
        }
        Context::Operation { action, arguments } => {
            if action["kind"] != "operation" {
                return Err(invalid());
            }
            let ports = action["ports"]["in"].as_array().unwrap();
            let mut previous = None;
            for entry in entries {
                let name = entry["port"].as_str().ok_or_else(invalid)?;
                let position = ports
                    .iter()
                    .position(|port| port["name"] == name)
                    .ok_or_else(invalid)?;
                let supplied = arguments.get(name).ok_or_else(invalid)?;
                let index = if ports[position]["many"] == true {
                    let count = supplied.as_array().ok_or_else(invalid)?.len();
                    // JSON Schema integers include integral numbers written as 1.0.
                    entry["index"]
                        .as_f64()
                        .filter(|index| {
                            index.is_finite()
                                && index.fract() == 0.0
                                && *index >= 0.0
                                && *index < count as f64
                        })
                        .ok_or_else(invalid)? as usize
                } else {
                    if entry.get("index").is_some() {
                        return Err(invalid());
                    }
                    0
                };
                let current = (position, index);
                if previous.is_some_and(|prior| prior >= current) {
                    return Err(invalid());
                }
                previous = Some(current);
            }
        }
    }
    Ok(())
}
