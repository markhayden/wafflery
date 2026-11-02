//! Scripted setup states; no credentials or providers are read.
use serde_json::{Value, json};
use std::path::Path;

pub fn description(root: &Path, scenario: &str) -> Value {
    let mut document = super::scenarios::description(scenario);
    if scenario == "renders" {
        document["actions"][0]["renders"] = json!(["echo", "hold", "stuck", "early-error", "document", "document-gated", "document-missing", "document-extra", "document-inside", "document-probe"].map(|mode| json!({
            "name":mode,"kind":"document","command":[super::fixtures::TOOL,"--render-test",mode,"{run.dir}"]
        })));
        for mode in [
            "server",
            "server-probe",
            "server-delayed",
            "server-extra",
            "server-timeout",
            "server-bad",
            "server-exit",
        ] {
            document["actions"][0]["renders"].as_array_mut().unwrap().push(json!({
                "name":mode,"kind":"server","ready_timeout_s":2,
                "command":[super::fixtures::TOOL,"--render-test",mode,"{run.dir}","--frame-ancestor","{frame_ancestor}"]
            }));
        }
    }
    if scenario == "estimates" {
        let mut paid = document["actions"][0].clone();
        paid["name"] = json!("price_test");
        paid["title"] = json!("Price a scripted call");
        paid["purpose"] = json!(if root.join("estimate-declaration-changed").exists() {
            "Updated estimate declaration."
        } else {
            "Estimate copied inputs without a provider call."
        });
        paid["images"]
            .as_array_mut()
            .unwrap()
            .push(json!({"slot":"references","role":"reference","required":false,"many":true}));
        paid["readiness"] = super::paid_scenarios::readiness(root);
        paid["cost"] = json!({"may_spend":true,"basis":"estimated"});
        paid["params_schema"]["properties"]["max_spend_usd"] =
            json!({"type":"number","exclusiveMinimum":0});
        document["actions"].as_array_mut().unwrap().push(paid);
    }
    if !scenario.starts_with("readiness") {
        return document;
    }
    document["actions"][0]["params_schema"]["properties"]["filler"] =
        json!({"type":"boolean","title":"Use optional filler","default":false});
    document["actions"][0]["readiness"] =
        json!({"state":"needs_setup","requirements":[missing_filler()]});
    let state =
        std::fs::read_to_string(root.join("credential-state")).unwrap_or_else(|_| "present".into());
    let state = match state.trim() {
        "present" | "valid" | "invalid" | "missing" | "unknown" => state.trim(),
        _ => "unknown",
    };
    let mut paid =
        super::control_fixtures::description(super::fixtures::TOOL, "ready")["actions"][1].clone();
    paid["readiness"] = json!({
        "state": if matches!(state, "missing" | "invalid") { "needs_setup" } else { "ready" },
        "requirements":[{"kind":"credential","name":"EXAMPLE_API_KEY","state":state,
            "setup":"Configure the provider credential outside the playground, then restart the tool."}]
    });
    document["actions"].as_array_mut().unwrap().push(paid);
    document
}
fn missing_filler() -> Value {
    json!({"kind":"config","name":"optional filler","state":"missing",
        "setup":"Configure the optional filler or leave Use optional filler off."})
}
pub fn refusal(scenario: &str, arguments: &Value) -> Option<Value> {
    (scenario.starts_with("readiness") && arguments["filler"] == true)
        .then(|| json!({"reason":"not_ready","run":null,"requirements":[missing_filler()]}))
}
