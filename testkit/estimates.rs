//! Independent estimate controls; filesystem gates let tests observe the remote reader.
use super::{PeerState, reply, send};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    io::{self, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

pub fn start(
    root: PathBuf,
    state: Arc<Mutex<PeerState>>,
    output: Arc<Mutex<io::Stdout>>,
    message: Value,
) {
    std::thread::spawn(move || {
        let arguments = &message["params"]["arguments"]["arguments"];
        let description = super::readiness::description(&root, "estimates");
        let action = description["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|action| action["name"] == "price_test")
            .unwrap();
        if let Err(error) =
            super::scenarios::validate(&super::scenarios::input_schema(action), arguments)
        {
            super::reject(&output, &message["id"], -32602, &error);
            return;
        }
        let label = arguments["label"].as_str().unwrap_or("");
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("estimates.ndjson"))
            .unwrap();
        writeln!(journal, "{}", message["params"]).unwrap();
        std::fs::write(root.join("estimate-started"), arguments.to_string()).unwrap();
        if label == "disconnect" {
            let mut output = output.lock().unwrap();
            writeln!(output, "not a JSON-RPC response").unwrap();
            output.flush().unwrap();
        }
        if matches!(label, "hold" | "disconnect") {
            while !root.join("release-estimate").exists() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        let cap = arguments["max_spend_usd"].as_f64().unwrap_or(1.0);
        if cap > 1.0 {
            let mut data = super::control_fixtures::refusal("invalid_arguments");
            data["error"]["reason"] = json!("The requested cap exceeds the configured cap of $1.");
            send(
                &output,
                json!({"jsonrpc":"2.0","id":message["id"],"error":{"code":-32602,"message":"Requested cap exceeds the configured cap","data":data}}),
            );
            return;
        }
        if label.ends_with("error") {
            let mut error = json!({"code":-32602,"message":"Scripted estimate rejection"});
            if label == "structured-error" {
                error["data"] = super::control_fixtures::refusal("invalid_arguments");
            }
            send(
                &output,
                json!({"jsonrpc":"2.0","id":message["id"],"error":error}),
            );
            return;
        }
        let roots = state.lock().unwrap().allow_dirs.clone();
        let images = match super::scenarios::prepare_images(arguments, &roots) {
            Ok((images, _, _)) => images,
            Err(error) => {
                refuse_input(&output, &message["id"], &error.to_string());
                return;
            }
        };
        let path = &arguments["source"]["path"];
        let digest = images[0]["image"]["file_sha256"].as_str().unwrap();
        std::fs::write(root.join("estimate-read"), digest).unwrap();
        if matches!(label, "ledger" | "drain") {
            let mut state = state.lock().unwrap();
            let spent = state.session["spent_usd"]
                .as_f64()
                .unwrap()
                .max(if label == "drain" { 0.19 } else { 0.08 });
            state.session["spent_usd"] = json!(spent);
            super::paid_scenarios::balance(&mut state.session);
        }
        let session = state.lock().unwrap().session.clone();
        let ready = super::paid_scenarios::readiness(&root);
        let refused = if label == "readiness" || ready["state"] != "ready" {
            Some("readiness")
        } else if cap < 0.12 {
            Some("cap")
        } else if session["remaining_usd"].as_f64().unwrap() + 1e-6 < 0.12 {
            Some("budget")
        } else {
            None
        };
        let mut value = super::control_fixtures::estimate(super::fixtures::TOOL, &session, refused);
        value["effective"]["x-sample-lab"] = json!({"path":path,"binding":"internal-debug"});
        value["session"] = session.clone();
        if label != "readiness" {
            value["readiness"] = ready;
        }
        value["cap_usd"] = json!(cap);
        value["binding"] = match binding(
            arguments,
            &value["tool"],
            &value["effective"],
            &session,
            &images,
        ) {
            Ok(binding) => json!(binding),
            Err(error) => {
                refuse_input(&output, &message["id"], &error.to_string());
                return;
            }
        };
        value["expires"] = json!(
            (time::OffsetDateTime::now_utc() + time::Duration::minutes(5))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        );
        if label == "wrong-session" {
            value["session_id"] = json!("33333333-3333-4333-8333-333333333333");
        }
        state
            .lock()
            .unwrap()
            .estimates
            .insert(value["binding"].as_str().unwrap().to_owned(), value.clone());
        reply(
            &output,
            &message["id"],
            json!({"structuredContent":value,"content":[],"isError":false}),
        );
    });
}

fn refuse_input(output: &Arc<Mutex<io::Stdout>>, id: &Value, reason: &str) {
    send(
        output,
        json!({"jsonrpc":"2.0","id":id,"error":{
            "code":-32602,"message":"Scripted estimate input was refused",
            "data":{"reason":"invalid_arguments","run":null,
                "error":{"code":"input","retryable":false,"reason":reason}}
        }}),
    );
}

pub fn binding(
    arguments: &Value,
    tool: &Value,
    effective: &Value,
    session: &Value,
    images: &Value,
) -> io::Result<String> {
    let mut bound = arguments.clone();
    let object = bound
        .as_object_mut()
        .ok_or_else(|| io::Error::other("missing arguments"))?;
    object.remove("return_images");
    object.remove("image_blocks");
    let images = images
        .as_array()
        .ok_or_else(|| io::Error::other("missing resolved inputs"))?;
    let references = match arguments.get("references") {
        Some(value) => value
            .as_array()
            .ok_or_else(|| io::Error::other("invalid references"))?
            .as_slice(),
        None => &[],
    };
    if images.len() != references.len() + 1 {
        return Err(io::Error::other("resolved input count differs"));
    }
    let normalize = |input: &Value, resolved: &Value, slot: &str| -> io::Result<Value> {
        if !input.is_object() || resolved["slot"] != slot {
            return Err(io::Error::other("resolved input order or slot differs"));
        }
        let fingerprint = resolved["image"]["file_sha256"]
            .as_str()
            .ok_or_else(|| io::Error::other("missing resolved fingerprint"))?;
        Ok(json!({"file_sha256":fingerprint,"expect":input["expect"]}))
    };
    bound["source"] = normalize(&arguments["source"], &images[0], "source")?;
    if arguments.get("references").is_some() {
        bound["references"] = references
            .iter()
            .zip(&images[1..])
            .map(|(input, resolved)| normalize(input, resolved, "references"))
            .collect::<io::Result<Value>>()?;
    }
    Ok(format!(
        "sha256:{}",
        hash(
            &serde_json::to_vec(&json!([
                "price_test",
                bound,
                tool,
                effective["model"],
                effective["config_identity"],
                session["id"]
            ]))
            .map_err(io::Error::other)?
        )
    ))
}

fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod binding_tests {
    use super::*;
    fn bind(arguments: &Value, images: &Value) -> io::Result<String> {
        binding(
            arguments,
            &json!({"name":"sample-lab","version":"1"}),
            &json!({"model":"offline-script","config_identity":"configured"}),
            &json!({"id":"session"}),
            images,
        )
    }
    fn image(slot: &str, digit: &str) -> Value {
        json!({"slot":slot,"image":{"file_sha256":digit.repeat(64)}})
    }
    #[test]
    fn bindings_use_resolved_fingerprints_without_reopening_paths() {
        let files = json!([image("source", "a"), image("references", "b")]);
        let mut arguments = json!({"source":{"path":"/missing/source"},"references":[{"path":"/missing/reference"}],"label":"sample"});
        let first = bind(&arguments, &files).unwrap();
        arguments["source"] = json!({"inline":{"data":"deliberately not decoded again"}});
        arguments["references"][0] = json!({"resource":"sample://unread"});
        arguments["return_images"] = json!("inline");
        arguments["image_blocks"] = json!(true);
        assert_eq!(bind(&arguments, &files).unwrap(), first);
        let changed = json!([image("source", "a"), image("references", "c")]);
        assert_ne!(bind(&arguments, &changed).unwrap(), first);
    }
    #[test]
    fn ordered_references_and_expected_frames_remain_part_of_the_binding() {
        let mut args = json!({"source":{"path":"/missing/source"},"references":[
            {"path":"/missing/first"},{"path":"/missing/second"}
        ]});
        let images = json!([
            image("source", "a"),
            image("references", "b"),
            image("references", "c")
        ]);
        let original = bind(&args, &images).unwrap();
        let reordered = json!([
            image("source", "a"),
            image("references", "c"),
            image("references", "b")
        ]);
        assert_ne!(bind(&args, &reordered).unwrap(), original);
        args["references"][0]["expect"] = json!({"projection":"equirectangular","topology":"wrap"});
        assert_ne!(bind(&args, &images).unwrap(), original);
    }
    #[test]
    fn missing_duplicate_or_unexpected_resolved_inputs_cannot_make_a_binding() {
        let args = json!({"source":{"path":"/missing/source"},"references":[{"path":"/missing/reference"}]});
        for images in [
            json!([]),
            json!([image("source", "a")]),
            json!([image("source", "a"), image("source", "a")]),
            json!([image("source", "a"), image("extra", "b")]),
            json!([
                image("source", "a"),
                image("references", "b"),
                image("extra", "c")
            ]),
        ] {
            assert!(bind(&args, &images).is_err());
        }
    }
}
