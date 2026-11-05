//! Owned control fixtures; independent of host types and real tool output.
use serde_json::{Value, json};

pub fn readiness(state: &str) -> Value {
    let mut requirements = json!([
        {"kind":"config","name":"sample.toml","state":"present"},
        {"kind":"credential","name":"SAMPLE_API_KEY","state":"present","setup":"Set SAMPLE_API_KEY before starting the tool."},
        {"kind":"env","name":"SAMPLE_MODEL_DIR","state":"present"},
        {"kind":"model","name":"offline-script","state":"valid"},
        {"kind":"credential","name":"session credential","state":"unknown"}
    ]);
    match state {
        "needs_setup" => {
            requirements[0]["state"] = json!("missing");
            requirements[0]["setup"] = json!("Create sample.toml or choose a config path.");
        }
        "unavailable" => requirements.as_array_mut().unwrap().push(json!({
            "kind":"dependency","name":"optional test backend","state":"missing",
            "setup":"This build does not include the optional test backend."
        })),
        _ => (),
    }
    json!({"state":state,"requirements":requirements})
}

pub fn description(tool: &str, state: &str) -> Value {
    json!({
        "contract":"0.5.0", "profiles":["images"],
        "tool":{"name":tool,"version":"0.1.0","purpose":"Independent offline contract test peer."},
        "actions":[
            {"name":"mark_band","kind":"task","title":"Add a test band","purpose":"Offline image fixture.",
             "params_schema":{"type":"object","additionalProperties":false,"properties":{"label":{"type":"string","minLength":1}},"required":["label"]},
             "images":[{"slot":"source","role":"source","required":true},{"slot":"references","role":"reference","required":false,"many":true}],
             "locations":[],"outputs":{"images":["candidate"],"locations":[]},
             "cost":{"may_spend":false},"readiness":{"state":"ready","requirements":[]},
             "duration":{"typical_s":1,"max_s":30},"renders":[]},
            {"name":"price_test","kind":"task","title":"Price a scripted call","purpose":"Exercises paid controls without contacting a provider.",
             "params_schema":{"type":"object","additionalProperties":false,"properties":{
                 "label":{"type":"string","minLength":1},"max_spend_usd":{"type":"number","exclusiveMinimum":0}
             },"required":["label"]},
             "images":[],"locations":[],"outputs":{"images":[],"locations":[]},
             "cost":{"may_spend":true,"basis":"estimated"},"readiness":readiness(state),
             "duration":{"typical_s":1,"max_s":30},"renders":[]}
        ],
        "vocabulary":{"disposition":[],"confidence":[],"evidence":[],"score_kind":[]},
        "limits":{"inline_bytes":16777216},"progress_interval_s":1,
        "resources":[format!("{tool}://session"),format!("{tool}://runs{{?cursor}}"),format!("{tool}://run/{{id}}/run.json"),format!("{tool}://run/{{id}}/journal")]
    })
}

pub fn session(tool: &str) -> Value {
    json!({"contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},
        "id":"22222222-2222-4222-8222-222222222222","started":"2026-11-01T00:00:00Z",
        "allowed_dirs":[],"budget_usd":2,"spent_usd":0,"unresolved_usd":0,"reserved_usd":0,"remaining_usd":2})
}

pub fn estimate(tool: &str, session: &Value, refused: Option<&str>) -> Value {
    let mut observation = session.clone();
    if refused == Some("budget") {
        observation["budget_usd"] = json!(0.05);
        observation["remaining_usd"] = json!(0.05);
    }
    let blocked = refused == Some("readiness");
    json!({"contract":"0.5.0","tool":{"name":tool,"version":"0.1.0"},"action":"price_test",
        "binding":format!("sha256:{}","a".repeat(64)),"session_id":observation["id"],
        "effective":{"model":if blocked {Value::Null} else {json!("offline-script")},
                     "config_identity":if blocked {Value::Null} else {json!(format!("sha256:{}","1".repeat(64)))}},
        "worst_case_usd":if blocked {Value::Null} else {json!(0.12)},
        "cap_usd":if refused == Some("cap") {0.01} else {0.5},
        "allowed":refused.is_none(),"refused":refused,
        "readiness":readiness(if blocked {"needs_setup"} else {"ready"}),"session":observation,
        "expires":"2026-11-01T00:05:00Z"})
}

pub fn refusal(reason: &str) -> Value {
    let mut value = json!({"reason":reason,"run":null});
    match reason {
        "not_ready" => value["requirements"] = json!([readiness("needs_setup")["requirements"][0]]),
        "invalid_arguments" => {
            value["error"] = json!({"code":"usage","retryable":false,"reason":"Invalid scripted estimate arguments."})
        }
        _ => (),
    }
    value
}
