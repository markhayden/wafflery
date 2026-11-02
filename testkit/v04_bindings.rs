//! Resolve the fake's declared inputs; binding never allocates runs or writes artifacts.
use super::{
    v04_fixtures as fixtures,
    v04_store::{Store, inline_bytes},
};
use serde_json::{Value, json};
#[path = "v04_schemas.rs"]
pub mod schemas;

#[derive(Debug)]
pub enum Failure {
    Rpc(Value),
    Invalid(String),
}
#[derive(Debug)]
pub struct Input {
    pub port: String,
    pub document: Value,
}
#[derive(Debug)]
pub struct Prepared {
    pub action: Value,
    pub arguments: Value,
    pub inputs: Vec<Input>,
    pub return_images: String,
    pub image_blocks: bool,
}

fn usage() -> Failure {
    Failure::Rpc(
        json!({"code":-32602,"message":"Arguments do not match the action's declared schema"}),
    )
}
fn problem(
    port: &str,
    index: Option<usize>,
    kind: &str,
    expected: &str,
    reference: &Value,
) -> Value {
    let mut value = json!({"port":port,"problem":kind,"expected":format!("{expected} v1 in the synthetic fixture frame"),"got":format!("{} {}",reference["type"].as_str().unwrap(),reference["identity"].as_str().unwrap())});
    if let Some(index) = index {
        value["index"] = json!(index);
    }
    if kind == "unresolved" {
        value["remedy"] = json!(if expected == "image" {
            "Choose the original image file or provide its bytes inline."
        } else {
            "Import the preserved artifact into this tool, then choose it again."
        });
    }
    value
}
fn without_pixel_hint(value: &Value) -> Value {
    let mut value = value.clone();
    if let Some(object) = value.as_object_mut() {
        object.remove("pixel_sha256");
    }
    value
}
fn comparable_reference(value: &Value) -> Value {
    let mut value = value.clone();
    if let Some(on) = value.get_mut("on") {
        *on = without_pixel_hint(on);
    }
    value
}
fn resolve_reference(
    store: &Store,
    tool: &str,
    expected: &str,
    reference: &Value,
) -> Result<Value, &'static str> {
    if reference["type"] != expected {
        return Err("wrong_type");
    }
    if reference["schema_version"] != 1 {
        return Err("wrong_version");
    }
    if reference["tool"] != tool {
        return Err(if expected.contains(':') {
            "wrong_type"
        } else {
            "unresolved"
        });
    }
    let document = store
        .read(reference["identity"].as_str().unwrap())
        .map_err(|_| "unresolved")?;
    if document["artifact"]["type"] != expected {
        return Err("wrong_type");
    }
    if document["artifact"]["frame"] != reference["frame"]
        || without_pixel_hint(&document["artifact"]["on"]) != without_pixel_hint(&reference["on"])
    {
        return Err("wrong_frame");
    }
    if comparable_reference(&document["artifact"]) != comparable_reference(reference) {
        return Err("unresolved");
    }
    Ok(document)
}
fn source_image(store: &Store, tool: &str, value: &Value) -> Result<Value, String> {
    let bytes = if let Some(path) = value["path"].as_str() {
        store.read_path(path)
    } else {
        inline_bytes(value["inline"]["data"].as_str().unwrap())
    }
    .map_err(|error| {
        error["message"]
            .as_str()
            .unwrap_or("Image bytes unavailable")
            .to_owned()
    })?;
    if bytes != include_bytes!("../ui/tests/fixtures/sample.png")
        || (value.get("inline").is_some() && value["inline"]["media_type"] != "image/png")
    {
        return Err(
            "This synthetic peer only accepts its owned sample PNG; no example was substituted"
                .into(),
        );
    }
    let document = fixtures::artifact(tool, "image");
    if let Some(expect) = value.get("expect") {
        for field in ["file_sha256", "width", "height", "projection", "topology"] {
            if expect[field] != document["record"][field] {
                return Err("Image fingerprint or frame differs from expect".into());
            }
        }
    }
    Ok(document)
}

pub fn prepare(
    store: &Store,
    tool: &str,
    action: &str,
    arguments: Value,
) -> Result<Prepared, Failure> {
    let description = fixtures::description(tool);
    let action = description["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["name"] == action)
        .ok_or_else(usage)?
        .clone();
    let ports = schemas::ports(&action);
    let schema = schemas::input_schema(&action);
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .expect("pinned synthetic action schema");
    if !validator.is_valid(&arguments) {
        return Err(usage());
    }
    let return_images = arguments["return_images"]
        .as_str()
        .unwrap_or("resource")
        .to_owned();
    let image_blocks = arguments["image_blocks"].as_bool().unwrap_or(false);
    let mut resolved = arguments.as_object().unwrap().clone();
    resolved.remove("return_images");
    resolved.remove("image_blocks");
    for (name, parameter) in action["params_schema"]["properties"].as_object().unwrap() {
        if !resolved.contains_key(name)
            && let Some(default) = parameter.get("default")
        {
            resolved.insert(name.clone(), default.clone());
        }
    }
    let mut inputs = vec![];
    let mut problems = vec![];
    let mut invalid = None;
    for port in &ports {
        let name = port["name"].as_str().unwrap();
        let Some(given) = arguments.get(name) else {
            continue;
        };
        let many = port["many"] == true;
        let values: Vec<&Value> = if many {
            given.as_array().unwrap().iter().collect()
        } else {
            vec![given]
        };
        let mut normalized = vec![];
        for (index, value) in values.into_iter().enumerate() {
            let expected = port["type"].as_str().unwrap();
            let is_reference = value.get("identity").is_some();
            let document = if is_reference {
                match resolve_reference(store, tool, expected, value) {
                    Ok(document) => document,
                    Err(kind) => {
                        problems.push(problem(name, many.then_some(index), kind, expected, value));
                        continue;
                    }
                }
            } else {
                let candidate = if expected == "image" {
                    source_image(store, tool, value)
                } else {
                    let marker = fixtures::artifact(tool, "location");
                    let mut comparable = value.clone();
                    comparable["image"] = without_pixel_hint(&value["image"]);
                    if comparable == marker["record"] {
                        Ok(marker)
                    } else {
                        Err("This synthetic peer only accepts its midpoint marker in the owned fixture frame".into())
                    }
                };
                match candidate {
                    Ok(document) => document,
                    Err(message) => {
                        invalid.get_or_insert(message);
                        continue;
                    }
                }
            };
            normalized.push(if is_reference {
                document["artifact"].clone()
            } else if expected == "image" {
                json!({"expect":document["record"]})
            } else {
                value.clone()
            });
            inputs.push(Input {
                port: name.into(),
                document,
            });
        }
        if many {
            resolved.insert(name.into(), json!(normalized));
        } else if let Some(value) = normalized.pop() {
            resolved.insert(name.into(), value);
        }
    }
    if !problems.is_empty() {
        return Err(Failure::Rpc(
            json!({"code":-32602,"message":"Artifact inputs did not bind","data":{"reason":"binding","run":null,"bindings":problems}}),
        ));
    }
    if let Some(message) = invalid {
        return Err(Failure::Invalid(message));
    }
    Ok(Prepared {
        action,
        arguments: Value::Object(resolved),
        inputs,
        return_images,
        image_blocks,
    })
}
