//! Callable schemas for the fake's own declarations and the pinned contract's acyclic inputs.
use serde_json::{Value, json};

pub fn ports(action: &Value) -> Vec<Value> {
    if action["kind"] == "operation" {
        action["ports"]["in"].as_array().unwrap().clone()
    } else {
        action["images"].as_array().unwrap().iter().map(|slot| json!({"name":slot["slot"],"type":"image","required":slot["required"],"many":slot.get("many").cloned().unwrap_or(json!(false))})).collect()
    }
}
pub fn input_schema(action: &Value) -> Value {
    let operation = action["kind"] == "operation";
    let mut schema = action["params_schema"].clone();
    schema["required"] = schema.get("required").cloned().unwrap_or(json!([]));
    for port in ports(action) {
        let mut alternatives = vec![];
        if operation {
            alternatives.push(json!({"$ref":"urn:table-manners:operations:artifact-ref"}));
        }
        match port["type"].as_str().unwrap() {
            "image" => alternatives.push(json!({"$ref":"urn:table-manners:images:image-in"})),
            "location" => alternatives.push(json!({"$ref":"urn:table-manners:images:location"})),
            _ => {}
        }
        let item = json!({"oneOf":alternatives});
        let name = port["name"].as_str().unwrap();
        schema["properties"][name] = if port["many"] == true {
            json!({"type":"array","minItems":1,"items":item})
        } else {
            item
        };
        if port["required"] == true {
            schema["required"].as_array_mut().unwrap().push(json!(name));
        }
    }
    schema["properties"]["return_images"] = json!({"enum":["path","inline","resource"]});
    schema["properties"]["image_blocks"] = json!({"type":"boolean"});
    standalone(&schema)
}

/// Expand only owned schemas/pinned contract documents, never caller schemas or remote URLs.
/// Sibling keywords stay at the reference's level so unevaluatedProperties sees its evaluated fields.
pub fn standalone(schema: &Value) -> Value {
    fn document(id: &str) -> Value {
        let text = table_manners::SCHEMAS
            .iter()
            .find(|(uri, _)| *uri == id)
            .expect("only pinned contract references")
            .1;
        serde_json::from_str(text).unwrap()
    }
    fn expand(value: &Value, root: &Value, stack: &mut Vec<String>) -> Value {
        match value {
            Value::Object(fields) => {
                let mut siblings = fields.clone();
                siblings.remove("$id");
                siblings.remove("$schema");
                if let Some(Value::String(reference)) = siblings.remove("$ref") {
                    let (base, pointer) = reference.split_once('#').unwrap_or((&reference, ""));
                    let other = if base.is_empty() {
                        root.clone()
                    } else {
                        document(base)
                    };
                    let key = format!("{}#{pointer}", other["$id"].as_str().unwrap_or("input"));
                    assert!(
                        !stack.contains(&key),
                        "pinned input schemas must be acyclic"
                    );
                    stack.push(key);
                    let target = if pointer.is_empty() {
                        &other
                    } else {
                        other.pointer(pointer).expect("pinned schema pointer")
                    };
                    let expanded = expand(target, &other, stack);
                    stack.pop();
                    let mut siblings = expand(&Value::Object(siblings), root, stack);
                    let all_of = siblings
                        .as_object_mut()
                        .unwrap()
                        .entry("allOf")
                        .or_insert(json!([]));
                    all_of.as_array_mut().unwrap().push(expanded);
                    siblings
                } else {
                    Value::Object(
                        siblings
                            .into_iter()
                            .map(|(key, value)| (key, expand(&value, root, stack)))
                            .collect(),
                    )
                }
            }
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|value| expand(value, root, stack))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }
    let mut expanded = expand(schema, schema, &mut vec![]);
    expanded["$schema"] = json!("https://json-schema.org/draft/2020-12/schema");
    expanded["type"] = json!("object");
    expanded
}
