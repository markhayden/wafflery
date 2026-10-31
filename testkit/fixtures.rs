//! Owned schema fixtures for the independent offline peer; never real-tool output.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const TOOL: &str = "sample-lab";
pub const SAMPLE: &[u8] = include_bytes!("../ui/tests/fixtures/sample.png");

pub fn description(version: &str) -> Value {
    json!({
        "contract": version,
        "profiles": ["images"],
        "tool": {"name": TOOL, "version": "0.1.0", "purpose": "Offline test tool. Adds an illustrative band to exercise the workspace; it does not repair images."},
        "actions": [{
            "name": "mark_band", "title": "Add a test band", "purpose": "Creates a deterministic visual change for testing inputs, inspection and saved runs.",
            "params_schema": {"type": "object", "additionalProperties": false, "properties": {
                "label": {"type": "string", "title": "Label", "description": "A note to identify this experiment.", "minLength": 1, "maxLength": 120},
                "tone": {"type": "string", "title": "Band color", "enum": ["blue", "amber"], "default": "blue"},
                "strength": {"type": "number", "title": "Strength", "minimum": 0, "maximum": 1, "default": 0.5}
            }, "required": ["label"]},
            "images": [{"slot": "source", "role": "source", "required": true}],
            "locations": [], "outputs": {"images": ["candidate"], "locations": []},
            "cost": {"may_spend": false, "needs_env": []},
            "duration": {"typical_s": 1, "max_s": 30}, "renders": []
        }],
        "vocabulary": {"disposition": [], "confidence": [], "evidence": [], "score_kind": []},
        "limits": {"inline_bytes": 16777216},
        "progress_interval_s": 1,
        "resources": [format!("{TOOL}://runs"), format!("{TOOL}://run/{{id}}/run.json"), format!("{TOOL}://run/{{id}}/journal")]
    })
}

pub fn reference(bytes: &[u8], width: u32, height: u32) -> Value {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    json!({"file_sha256": digest, "width": width, "height": height, "projection": "flat", "topology": "bounded"})
}

pub fn result(status: &str) -> Value {
    let reference = reference(SAMPLE, 160, 100);
    json!({
        "contract": "0.1.0", "tool": {"name": TOOL, "version": "0.1.0"}, "action": "mark_band",
        "run": {"id": "fixture-001", "resource": format!("{TOOL}://run/fixture-001/run.json"), "dir": null},
        "status": status, "error": null, "started": "2026-01-01T00:00:00.000Z", "finished": "2026-01-01T00:00:00.000Z", "duration_ms": 0,
        "input_identity": format!("sha256:{}", "0".repeat(64)), "spend": {"usd": 0, "basis": "measured"}, "cache": "off",
        "images": {"in": [{"slot": "source", "role": "source", "image": reference}], "out": []},
        "locations": {"used": [], "found": []}, "notes": ["Owned schema fixture; no real provider or image repair."], "body": {"illustrative": true}
    })
}
