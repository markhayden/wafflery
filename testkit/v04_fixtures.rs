//! Independently authored v0.4 contract examples; no host or real-peer types.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

fn port(name: &str, kind: &str, required: bool, many: bool) -> Value {
    json!({"name":name,"type":kind,"required":required,"many":many,"purpose":"Synthetic typed fixture input or output."})
}

pub fn description(tool: &str) -> Value {
    let token = format!("{tool}:Token");
    let memo = format!("{tool}:Memo");
    let mut actions = vec![];
    for (name, group, inputs, outputs) in [
        (
            "locate_marker",
            "locate",
            vec![port("image", "image", true, false)],
            vec![port("marker", "location", true, false)],
        ),
        (
            "stamp_marker",
            "shape",
            vec![
                port("image", "image", true, false),
                port("marker", "location", true, false),
            ],
            vec![port("token", &token, true, false)],
        ),
        (
            "soften_token",
            "shape",
            vec![port("token", &token, true, false)],
            vec![port("token", &token, true, false)],
        ),
        (
            "collect_tokens",
            "inspect",
            vec![
                port("tokens", &token, true, true),
                port("guides", "location", false, true),
            ],
            vec![port("tokens", &token, true, true)],
        ),
    ] {
        let mut value = action(name, "operation");
        value["group"] = json!(group);
        value["ports"] = json!({"in":inputs,"out":outputs});
        if name == "soften_token" {
            value["params_schema"]["properties"] =
                json!({"amount":{"type":"number","minimum":0,"maximum":1,"default":0.25}});
        }
        value["outputs"]["images"] = json!(["preview"]);
        if name == "locate_marker" {
            value["outputs"]["locations"] = json!(["found"]);
        }
        value["params_schema"]["properties"]["max_spend_usd"] =
            json!({"type":"number","exclusiveMinimum":0});
        actions.push(value);
    }
    let mut task = action("make_token", "task");
    task["images"] = json!([{"slot":"source","role":"source","required":true}]);
    task["outputs"]["images"] = json!(["preview"]);
    actions.push(task);
    actions.push(action("summarize", "utility"));
    json!({
        "contract":"0.5.0","tool":{"name":tool,"version":"0.1.0","purpose":"Independent, synthetic operation fixtures; no image repair or provider calls."},
        "profiles":["images","operations"],"actions":actions,
        "artifact_types":[
            {"type":"location","schema_versions":[1],"purpose":"A shared marker.","portable":"json","preview":true},
            {"type":token,"schema_versions":[1],"purpose":"An opaque reusable token.","portable":"bundle","preview":true},
            {"type":memo,"schema_versions":[1],"purpose":"A workspace-only note.","portable":"none","preview":false}
        ],
        "vocabulary":{"group":["locate","shape","inspect"],"disposition":[],"confidence":[],"evidence":[],"score_kind":[]},
        "limits":{"inline_bytes":1048576},"progress_interval_s":1,
        "resources":[format!("{tool}://session"),format!("{tool}://runs{{?cursor}}"),format!("{tool}://run/{{id}}/run.json"),format!("{tool}://artifact/{{identity}}"),format!("{tool}://artifact/{{identity}}/preview")]
    })
}

fn action(name: &str, kind: &str) -> Value {
    json!({"name":name,"title":name.replace('_', " "),"kind":kind,"purpose":"Exercise an independently declared activity.",
        "params_schema":{"type":"object","properties":{},"additionalProperties":false},
        "images":[],"locations":[],"outputs":{"images":[],"locations":[]},
        "cost":{"may_spend":false},"readiness":{"state":"ready","requirements":[]},
        "duration":{"typical_s":0.1,"max_s":10},"renders":[]})
}

pub fn documents(tool: &str) -> Vec<(&'static str, Value)> {
    let mut docs = vec![("urn:table-manners:core:description", description(tool))];
    for status in ["completed", "cancelled", "error", "invalid"] {
        docs.push(("urn:table-manners:core:result", result(tool, status)));
    }
    for action in [
        "make_token",
        "locate_marker",
        "stamp_marker",
        "collect_tokens",
        "summarize",
    ] {
        docs.push((
            "urn:table-manners:core:result",
            example_result(tool, action),
        ));
    }
    for kind in [
        "image".to_owned(),
        "location".into(),
        format!("{tool}:Token"),
        format!("{tool}:Memo"),
    ] {
        docs.push((
            "urn:table-manners:operations:artifact",
            artifact(tool, &kind),
        ));
    }
    for problem in ["wrong_type", "wrong_version", "wrong_frame", "unresolved"] {
        docs.push((
            "urn:table-manners:core:refusal",
            refusal(problem, Some("tokens"), Some(1)),
        ));
    }
    docs.push((
        "urn:table-manners:core:refusal",
        refusal("unresolved", None, None),
    ));
    let token = artifact(tool, &format!("{tool}:Token"));
    let native = &token["artifact"];
    docs.push((
        "urn:table-manners:operations:export-request",
        json!({"artifact":native}),
    ));
    let bytes = serde_json::to_vec(&token).unwrap();
    docs.push(("urn:table-manners:operations:export-response", json!({
        "contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},"artifact":native,
        "file":{"path":"/fixture/exports/token.bundle","bytes":bytes.len(),"sha256":digest(&bytes),"media_type":format!("application/x-{tool}-token")}
    })));
    for already in [false, true] {
        docs.push(("urn:table-manners:operations:import-response", json!({
            "contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},"artifact":native,"already_present":already
        })));
    }
    docs.push((
        "urn:table-manners:operations:import-request",
        json!({"file":{"path":"/fixture/inputs/token.bundle"},"sha256":digest(&bytes)}),
    ));
    docs.push(("urn:table-manners:core:session", json!({
        "contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},"id":"11111111-1111-4111-8111-111111111111",
        "started":"2026-11-02T00:00:00Z","allowed_dirs":["/fixture/inputs"],
        "budget_usd":null,"spent_usd":0,"unresolved_usd":0,"reserved_usd":0,"remaining_usd":null
    })));
    docs
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn image_reference() -> Value {
    json!({"file_sha256":digest(include_bytes!("../ui/tests/fixtures/sample.png")),
        "width":160,"height":100,"projection":"flat","topology":"bounded"})
}

pub fn artifact(tool: &str, kind: &str) -> Value {
    artifact_variant(tool, kind, 0.0)
}

pub fn artifact_variant(tool: &str, kind: &str, amount: f64) -> Value {
    let image = image_reference();
    let record = if kind == "location" {
        json!({"image":image,"kind":"line","parts":[{"column":0.5,"width_px":2}],"straddles_wrap":false})
    } else if kind == "image" {
        image.clone()
    } else {
        json!({"synthetic_type":kind,"payload":"opaque fixture record","amount":amount})
    };
    let identity = if kind == "image" {
        format!("sha256:{}", image["file_sha256"].as_str().unwrap())
    } else {
        format!("sha256:{}", digest(&serde_json::to_vec(&record).unwrap()))
    };
    let resource = format!("{tool}://artifact/{identity}");
    let no_preview = kind.ends_with(":Memo");
    // The built-in image is its own picture. Other previews are actual encoded PNGs,
    // not fabricated fingerprints or native artifact bytes relabelled as images.
    let preview = if kind == "image" {
        image.clone()
    } else {
        let bytes = preview_png(kind, amount);
        json!({"file_sha256":digest(&bytes),"width":160,"height":100,"projection":"flat","topology":"bounded"})
    };
    let mut reference = json!({"type":kind,"schema_version":1,"tool":tool,"identity":identity,
        "frame":if no_preview {Value::Null} else {json!("fixture-grid")},"resource":resource});
    if !no_preview {
        reference["on"] = image.clone();
    }
    json!({"contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},"artifact":reference,
        "preview":if no_preview {Value::Null} else {json!({"role":"preview","image":preview,"resource":format!("{resource}/preview")})},
        "record":record})
}

pub fn result(tool: &str, status: &str) -> Value {
    let token = artifact(tool, &format!("{tool}:Token"));
    let output = artifact_variant(tool, &format!("{tool}:Token"), 0.25);
    let invalid = status == "invalid";
    let invocation = json!({"action":"soften_token","arguments":{"token":token["artifact"],"amount":0.25},
        "effective":{"model":null,"config_identity":format!("sha256:{}", digest(b"built-in-fixture-defaults"))},"requirements":[]});
    json!({"contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},"action":"soften_token",
        "run":{"id":"fixture-001","resource":format!("{tool}://run/fixture-001/run.json"),"dir":null,"session_id":null,"client_run_id":null},
        "status":status,"error":if matches!(status, "invalid" | "error") {json!({"code":if invalid {"usage"} else {"internal"},"retryable":false,"reason":"Synthetic fixture outcome."})} else {Value::Null},
        "started":"2026-11-02T00:00:00Z","finished":"2026-11-02T00:00:00Z","duration_ms":0,
        "input_identity":format!("sha256:{}", digest(b"synthetic-inputs")),"invocation":if invalid {Value::Null} else {invocation},
        "spend":{"usd":0,"unresolved_usd":0,"basis":"measured"},"cache":"off",
        "images":{"in":[],"out":if invalid {json!([])} else {json!([output["preview"]])}},"locations":{"used":[],"found":[]},
        "artifacts":{"in":if invalid {json!([])} else {json!([{"port":"token","artifact":token["artifact"],"preview":null}])},
            "out":if invalid {json!([])} else {json!([{"port":"token","artifact":output["artifact"],"preview":output["preview"]}])}},
        "notes":[],"body":{"illustrative":true}})
}

pub fn refusal(problem: &str, port: Option<&str>, index: Option<usize>) -> Value {
    let mut binding = json!({"port":port,"problem":problem,"expected":"A compatible Token v1.","got":"A synthetic incompatible reference."});
    if let Some(index) = index {
        binding["index"] = json!(index);
    }
    if problem == "unresolved" {
        binding["remedy"] = json!("Import the preserved artifact, then choose it again.");
    }
    json!({"reason":"binding","run":null,"bindings":[binding]})
}

pub fn preview_png(kind: &str, amount: f64) -> Vec<u8> {
    let mut pixels = image::RgbaImage::from_pixel(160, 100, image::Rgba([25, 25, 25, 255]));
    let half_width = if kind == "location" {
        1
    } else {
        12 + (amount * 40.0) as u32
    };
    for y in 0..100 {
        for x in 80 - half_width..80 + half_width {
            pixels.put_pixel(x, y, image::Rgba([40, 150, 220, 255]));
        }
    }
    let mut encoded = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(pixels)
        .write_to(&mut encoded, image::ImageFormat::Png)
        .unwrap();
    encoded.into_inner()
}

fn example_result(tool: &str, name: &str) -> Value {
    let image = artifact(tool, "image");
    let marker = artifact(tool, "location");
    let token = artifact(tool, &format!("{tool}:Token"));
    let changed = artifact_variant(tool, &format!("{tool}:Token"), 0.25);
    let memo = artifact(tool, &format!("{tool}:Memo"));
    let entry = |port: &str, value: &Value, preview: bool| json!({"port":port,"artifact":value["artifact"],"preview":if preview {value["preview"].clone()} else {Value::Null}});
    let (arguments, inputs, outputs) = match name {
        "make_token" => (
            json!({"source":{"expect":image_reference()}}),
            json!([]),
            json!([entry("token", &token, true)]),
        ),
        "locate_marker" => (
            json!({"image":image["artifact"]}),
            json!([entry("image", &image, false)]),
            json!([entry("marker", &marker, true)]),
        ),
        "stamp_marker" => (
            json!({"image":image["artifact"],"marker":marker["artifact"]}),
            json!([
                entry("image", &image, false),
                entry("marker", &marker, false)
            ]),
            json!([entry("token", &token, true)]),
        ),
        "collect_tokens" => (
            json!({"tokens":[token["artifact"],changed["artifact"]]}),
            json!([
                entry("tokens", &token, false),
                entry("tokens", &changed, false)
            ]),
            json!([
                entry("tokens", &token, true),
                entry("tokens", &changed, true)
            ]),
        ),
        "summarize" => (json!({}), json!([]), json!([entry("memo", &memo, true)])),
        _ => unreachable!("fixture action is explicitly selected"),
    };
    let mut value = result(tool, "completed");
    value["action"] = json!(name);
    value["run"]["id"] = json!(name);
    value["run"]["resource"] = json!(format!("{tool}://run/{name}/run.json"));
    value["invocation"]["action"] = json!(name);
    value["invocation"]["arguments"] = arguments;
    value["input_identity"] = json!(format!(
        "sha256:{}",
        digest(&serde_json::to_vec(&value["invocation"]).unwrap())
    ));
    value["images"]["in"] = if matches!(name, "make_token" | "locate_marker" | "stamp_marker") {
        json!([{"slot":if name == "make_token" {"source"} else {"image"},"role":"source","image":image_reference()}])
    } else {
        json!([])
    };
    value["images"]["out"] = json!(
        outputs
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|entry| (!entry["preview"].is_null()).then_some(entry["preview"].clone()))
            .collect::<Vec<_>>()
    );
    value["locations"]["found"] = if name == "locate_marker" {
        json!([marker["record"]])
    } else {
        json!([])
    };
    value["locations"]["used"] = if name == "stamp_marker" {
        json!([marker["record"]])
    } else {
        json!([])
    };
    value["artifacts"] = json!({"in":inputs,"out":outputs});
    value
}
