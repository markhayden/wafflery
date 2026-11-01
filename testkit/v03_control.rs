//! Control-only v0.3 preparation peer. No host SDK, execution permit, provider or run writes.
use super::{PeerState, v03_fixtures as fixtures};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::{self, BufRead, Read, Write};

fn structured(value: Value) -> Value {
    json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false})
}
fn invalid(message: &str, named: bool) -> Value {
    let mut error = json!({"code":-32602,"message":message});
    if named {
        let mut refusal = fixtures::refusal("invalid_arguments");
        refusal["error"]["reason"] = json!(message);
        error["data"] = refusal;
    }
    error
}
fn input_schema(action: &Value) -> Value {
    let mut schema = action["params_schema"].clone();
    for slot in action["images"].as_array().unwrap() {
        let image = json!({"$ref":"urn:table-manners:images:image-in"});
        schema["properties"][slot["slot"].as_str().unwrap()] = if slot["many"] == true {
            json!({"type":"array","minItems":1,"items":image})
        } else {
            image
        };
        if slot["required"] == true {
            schema["required"]
                .as_array_mut()
                .unwrap()
                .push(slot["slot"].clone());
        }
    }
    schema["properties"]["return_images"] = json!({"enum":["path","inline","resource"]});
    schema["properties"]["image_blocks"] = json!({"type":"boolean"});
    schema
}
fn estimate(document: &Value, session: &Value, request: &Value) -> Result<Value, Value> {
    let envelope = json!({"type":"object","additionalProperties":false,"required":["action","arguments"],"properties":{"action":{"type":"string"},"arguments":{"type":"object"}}});
    if !jsonschema::is_valid(&envelope, request) {
        return Err(invalid(
            "Estimate needs an action and arguments object",
            false,
        ));
    }
    let Some(action) = document["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["name"] == request["action"])
    else {
        return Err(invalid("Unknown scripted action", true));
    };
    // V01 only scripts this no-provider paid control. Image execution/binding joins later tasks.
    if action["name"] != "price_test" {
        return Err(invalid("This control probe only prices price_test", true));
    }
    let arguments = &request["arguments"];
    if !jsonschema::is_valid(&input_schema(action), arguments) {
        return Err(invalid("Arguments do not match the action schema", false));
    }
    let cap = arguments["max_spend_usd"].as_f64().unwrap_or(1.0);
    if cap > 1.0 {
        return Err(invalid(
            "The requested cap exceeds the configured cap",
            true,
        ));
    }
    let refused = if action["readiness"]["state"] != "ready" {
        Some("readiness")
    } else if cap < 0.12 {
        Some("cap")
    } else if session["remaining_usd"]
        .as_f64()
        .is_some_and(|left| left < 0.12)
    {
        Some("budget")
    } else {
        None
    };
    let tool = document["tool"]["name"].as_str().unwrap();
    let mut value = fixtures::estimate(tool, session, refused);
    value["session"] = session.clone();
    value["readiness"] = action["readiness"].clone();
    value["cap_usd"] = json!(cap);
    if action["readiness"]["state"] == "unavailable" {
        value["effective"] =
            json!({"model":"offline-script","config_identity":format!("sha256:{}","1".repeat(64))});
    }
    let mut binding_arguments = arguments.clone();
    let fields = binding_arguments.as_object_mut().unwrap();
    fields.remove("return_images");
    fields.remove("image_blocks");
    fields.insert("max_spend_usd".into(), json!(cap));
    let identity = json!({"action":action["name"],"arguments":binding_arguments,
        "tool":value["tool"],"effective":value["effective"],"session":session["id"]});
    let digest: String = Sha256::digest(serde_json::to_vec(&identity).unwrap())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    value["binding"] = json!(format!("sha256:{digest}"));
    value["expires"] = json!(
        (time::OffsetDateTime::now_utc() + time::Duration::minutes(5))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap()
    );
    Ok(value)
}

pub fn serve(scenario: &str) -> Result<(), Box<dyn std::error::Error>> {
    let (tool, state) = match scenario {
        "v03-ready" | "v03-budget" => ("sample-lab", "ready"),
        "v03-lens" => ("test-lens", "ready"),
        "v03-needs-setup" => ("sample-lab", "needs_setup"),
        "v03-unavailable" => ("sample-lab", "unavailable"),
        _ => return Err("Unknown v0.3 control scenario".into()),
    };
    let document = fixtures::description(tool, state);
    let fresh = PeerState::default().session;
    let mut session = fixtures::session(tool);
    session["id"] = fresh["id"].clone();
    session["started"] = fresh["started"].clone();
    if scenario == "v03-budget" {
        session["budget_usd"] = json!(0.05);
        session["remaining_usd"] = json!(0.05);
    }
    let mut initialized = false;
    let mut input = io::stdin().lock();
    let mut output = io::stdout().lock();
    loop {
        let mut line = Vec::new();
        if (&mut input)
            .take(32 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if line.len() > 32 * 1024 * 1024 {
            return Err("Request exceeds control peer limit".into());
        }
        let message: Value = serde_json::from_slice(&line)?;
        let Some(id) = message.get("id") else {
            continue;
        };
        let params = &message["params"];
        let result = match message["method"].as_str().unwrap_or("") {
            "initialize" => {
                if params["protocolVersion"] != "2025-11-25" {
                    Err(invalid("Only MCP 2025-11-25 is supported", false))
                } else {
                    initialized = true;
                    Ok(
                        json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{},"resources":{}},"serverInfo":{"name":tool,"version":"0.1.0"}}),
                    )
                }
            }
            _ if !initialized => Err(json!({"code":-32601,"message":"Initialize first"})),
            "ping" => Ok(json!({})),
            "tools/list" => {
                let mut tools = vec![
                    json!({"name":"describe","inputSchema":{"type":"object","additionalProperties":false}}),
                    json!({"name":"estimate","inputSchema":{"type":"object","additionalProperties":false,"required":["action","arguments"],"properties":{"action":{"type":"string"},"arguments":{"type":"object"}}}}),
                ];
                for action in document["actions"].as_array().unwrap() {
                    tools.push(json!({"name":action["name"],"inputSchema":input_schema(action)}));
                }
                Ok(json!({"tools":tools}))
            }
            "tools/call" if params["name"] == "describe" => Ok(structured(document.clone())),
            "tools/call" if params["name"] == "estimate" => {
                estimate(&document, &session, &params["arguments"]).map(structured)
            }
            "resources/list" => Ok(
                json!({"resources":[{"uri":format!("{tool}://session"),"name":"Session"},{"uri":format!("{tool}://runs"),"name":"Runs"}]}),
            ),
            "resources/read" => {
                let resource = if params["uri"] == format!("{tool}://session") {
                    Some(session.clone())
                } else if params["uri"] == format!("{tool}://runs") {
                    Some(
                        json!({"contract":"0.3.0","tool":session["tool"],"runs":[],"next_cursor":null}),
                    )
                } else {
                    None
                };
                match resource {
                    Some(value) => Ok(
                        json!({"contents":[{"uri":params["uri"],"mimeType":"application/json","text":value.to_string()}]}),
                    ),
                    None => Err(json!({"code":-32002,"message":"Resource not found"})),
                }
            }
            _ => Err(
                json!({"code":-32601,"message":"Only v0.3 preparation controls are implemented"}),
            ),
        };
        let response = match result {
            Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":value}),
            Err(value) => json!({"jsonrpc":"2.0","id":id,"error":value}),
        };
        writeln!(output, "{response}")?;
        output.flush()?;
    }
    Ok(())
}
