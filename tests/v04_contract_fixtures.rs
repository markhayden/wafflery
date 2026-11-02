#[path = "../testkit/v04_fixtures.rs"]
mod fixtures;
use serde_json::Value;

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let (_, text) = table_manners_v04::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or_else(|| format!("unknown v0.4 schema {uri}"))?;
        Ok(serde_json::from_str(text)?)
    }
}

fn validator(name: &str) -> jsonschema::Validator {
    let (_, text) = table_manners_v04::SCHEMAS
        .iter()
        .find(|(id, _)| *id == name)
        .unwrap();
    jsonschema::options()
        .with_retriever(Embedded)
        .should_validate_formats(true)
        .build(&serde_json::from_str::<Value>(text).unwrap())
        .unwrap()
}

#[test]
fn descriptions_are_independent_v04_operations_peers() {
    assert_eq!(table_manners_v04::CONTRACT_VERSION, "0.4.0");
    for tool in ["sample-lab", "test-prism"] {
        let description = fixtures::description(tool);
        validator("urn:table-manners:core:description")
            .validate(&description)
            .unwrap();
        assert_eq!(description["contract"], "0.4.0");
        assert_eq!(description["tool"]["name"], tool);
    }
}

#[test]
fn every_v04_fixture_matches_its_published_schema() {
    for tool in ["sample-lab", "test-prism"] {
        let documents = fixtures::documents(tool);
        let covered: std::collections::BTreeSet<_> = documents.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            covered,
            [
                "urn:table-manners:core:description",
                "urn:table-manners:core:result",
                "urn:table-manners:core:refusal",
                "urn:table-manners:core:session",
                "urn:table-manners:operations:artifact",
                "urn:table-manners:operations:export-request",
                "urn:table-manners:operations:export-response",
                "urn:table-manners:operations:import-request",
                "urn:table-manners:operations:import-response",
            ]
            .into_iter()
            .collect()
        );
        for (schema, value) in documents {
            validator(schema).validate(&value).unwrap_or_else(|error| {
                panic!("{tool}: {schema}: {error}; fixture={value}");
            });
        }
    }
}

#[test]
fn kinds_groups_and_ports_are_declared_without_name_heuristics() {
    let document = fixtures::description("test-prism");
    let actions = document["actions"].as_array().unwrap();
    let kinds: std::collections::BTreeSet<_> = actions
        .iter()
        .map(|a| a["kind"].as_str().unwrap())
        .collect();
    assert_eq!(
        kinds,
        ["operation", "task", "utility"].into_iter().collect()
    );
    let types = document["artifact_types"].as_array().unwrap();
    let mut optional_many = false;
    for action in actions {
        if let Some(group) = action.get("group") {
            assert!(
                document["vocabulary"]["group"]
                    .as_array()
                    .unwrap()
                    .contains(group)
            );
        }
        if action["kind"] != "operation" {
            assert!(action.get("ports").is_none());
            continue;
        }
        assert_eq!(action["images"], serde_json::json!([]));
        assert_eq!(action["locations"], serde_json::json!([]));
        for direction in ["in", "out"] {
            let mut names = std::collections::BTreeSet::new();
            for port in action["ports"][direction].as_array().unwrap() {
                let name = port["name"].as_str().unwrap();
                assert!(names.insert(name), "duplicate port {name}");
                assert!(action["params_schema"]["properties"].get(name).is_none());
                assert!(port["type"] == "image" || types.iter().any(|t| t["type"] == port["type"]));
                optional_many |=
                    direction == "in" && port["many"] == true && port["required"] == false;
            }
        }
    }
    assert!(optional_many);
    assert!(
        types
            .iter()
            .any(|t| t["type"] == "location" && t["portable"] == "json")
    );
    assert!(
        types
            .iter()
            .any(|t| t["portable"] == "none" && t["preview"] == false)
    );
    assert!(
        types
            .iter()
            .any(|t| t["portable"] == "bundle" && t["preview"] == true)
    );
}

#[test]
fn invalid_profile_port_and_invocation_shapes_are_not_examples_of_conformance() {
    use serde_json::json;
    let description = validator("urn:table-manners:core:description");
    for (pointer, value) in [
        ("/profiles", json!(["operations"])),
        ("/actions/0/kind", json!("task")),
        (
            "/actions/0/images",
            json!([{"slot":"image","role":"source","required":true}]),
        ),
        ("/actions/0/ports/in/0/many", json!("true")),
        ("/artifact_types/0/schema_versions", json!([2])),
    ] {
        let mut bad = fixtures::description("sample-lab");
        // An existing pointer makes this mutation fail loudly if fixture layout changes.
        *bad.pointer_mut(pointer).unwrap() = value;
        assert!(
            !description.is_valid(&bad),
            "unexpected valid mutation {pointer}"
        );
    }
    let mut missing_ports = fixtures::description("sample-lab");
    missing_ports["actions"][0]
        .as_object_mut()
        .unwrap()
        .remove("ports");
    assert!(!description.is_valid(&missing_ports));
    let mut utility_ports = fixtures::description("sample-lab");
    utility_ports["actions"][5]["ports"] = json!({"in":[],"out":[]});
    assert!(!description.is_valid(&utility_ports));
    let schema = validator("urn:table-manners:core:result");
    for status in ["completed", "error", "cancelled"] {
        let mut bad = fixtures::result("sample-lab", status);
        bad["invocation"] = Value::Null;
        assert!(!schema.is_valid(&bad), "null invocation for {status}");
        bad.as_object_mut().unwrap().remove("invocation");
        assert!(!schema.is_valid(&bad), "missing invocation for {status}");
    }
    let mut bad = fixtures::result("sample-lab", "completed");
    bad["invocation"]["requirements"] = json!(["/private/config.toml"]);
    assert!(!schema.is_valid(&bad));
}

#[test]
fn references_previews_and_salvaged_outputs_remain_distinct() {
    for tool in ["sample-lab", "test-prism"] {
        let description = fixtures::description(tool);
        for kind in [
            "image".to_owned(),
            "location".into(),
            format!("{tool}:Token"),
            format!("{tool}:Memo"),
        ] {
            let resource = fixtures::artifact(tool, &kind);
            let reference = &resource["artifact"];
            assert_eq!(reference["tool"], tool);
            assert_eq!(
                reference["resource"],
                format!(
                    "{tool}://artifact/{}",
                    reference["identity"].as_str().unwrap()
                )
            );
            if kind == "image" {
                assert_eq!(
                    reference["identity"],
                    format!(
                        "sha256:{}",
                        reference["on"]["file_sha256"].as_str().unwrap()
                    )
                );
            } else {
                let declaration = description["artifact_types"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|t| t["type"] == kind)
                    .unwrap();
                assert_eq!(
                    resource["preview"].is_null(),
                    declaration["preview"] == false
                );
            }
            if !resource["preview"].is_null() {
                assert_eq!(resource["preview"]["role"], "preview");
                assert_ne!(reference["resource"], resource["preview"]["resource"]);
            }
        }
        for status in ["completed", "cancelled", "error"] {
            let result = fixtures::result(tool, status);
            let output = &result["artifacts"]["out"][0];
            assert_eq!(result["invocation"]["action"], result["action"]);
            assert_eq!(
                result["invocation"]["arguments"]["token"],
                result["artifacts"]["in"][0]["artifact"]
            );
            assert!(
                result["images"]["out"]
                    .as_array()
                    .unwrap()
                    .contains(&output["preview"])
            );
            assert!(
                !output["artifact"].is_null(),
                "salvaged output lost for {status}"
            );
        }
    }
}

#[test]
fn binding_refusals_distinguish_list_items_from_resource_reads() {
    use serde_json::json;
    let schema = validator("urn:table-manners:core:refusal");
    for problem in ["wrong_type", "wrong_version", "wrong_frame", "unresolved"] {
        let refused = fixtures::refusal(problem, Some("tokens"), Some(1));
        assert_eq!(refused["run"], Value::Null);
        assert_eq!(refused["bindings"][0]["index"], 1);
        assert_eq!(refused["bindings"][0]["problem"], problem);
        assert!(schema.is_valid(&refused));
    }
    let mut missing = fixtures::refusal("unresolved", None, None);
    assert!(schema.is_valid(&missing));
    assert!(missing["bindings"][0].get("index").is_none());
    missing["bindings"][0]["index"] = json!(0);
    assert!(!schema.is_valid(&missing));
    let mut missing = fixtures::refusal("unresolved", Some("tokens"), Some(1));
    missing["bindings"][0]
        .as_object_mut()
        .unwrap()
        .remove("remedy");
    assert!(!schema.is_valid(&missing));
    let mut bad = fixtures::refusal("wrong_type", Some("tokens"), Some(1));
    bad["bindings"][0]["index"] = json!(-1);
    assert!(!schema.is_valid(&bad));
}

#[test]
fn results_cover_tasks_locations_and_many_outputs() {
    use std::collections::BTreeSet;
    let documents = fixtures::documents("sample-lab");
    let names: BTreeSet<_> = documents
        .iter()
        .filter(|(id, _)| *id == "urn:table-manners:core:result")
        .map(|(_, result)| result["action"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "collect_tokens",
            "locate_marker",
            "make_token",
            "soften_token",
            "stamp_marker",
            "summarize"
        ]
        .into_iter()
        .collect()
    );
}

#[test]
fn changed_token_is_not_confused_with_its_input() {
    let result = fixtures::result("sample-lab", "completed");
    assert_eq!(
        result["artifacts"]["in"][0]["artifact"]["type"],
        result["artifacts"]["out"][0]["artifact"]["type"]
    );
    assert_ne!(
        result["artifacts"]["in"][0]["artifact"]["identity"],
        result["artifacts"]["out"][0]["artifact"]["identity"]
    );
}

#[test]
fn control_paths_fit_the_session_grants_and_export_bytes() {
    use sha2::{Digest, Sha256};
    let documents = fixtures::documents("sample-lab");
    let session = &documents
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:core:session")
        .unwrap()
        .1;
    let import = &documents
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:operations:import-request")
        .unwrap()
        .1;
    let path = std::path::Path::new(import["file"]["path"].as_str().unwrap());
    assert!(
        session["allowed_dirs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|dir| path.starts_with(dir.as_str().unwrap()))
    );
    let export = &documents
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:operations:export-response")
        .unwrap()
        .1;
    let bytes = serde_json::to_vec(&fixtures::artifact("sample-lab", "sample-lab:Token")).unwrap();
    let digest: String = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(export["file"]["bytes"], bytes.len());
    assert_eq!(export["file"]["sha256"], digest);
    assert_eq!(import["sha256"], export["file"]["sha256"]);
}

#[test]
fn result_ports_invocations_and_profile_mirrors_agree_with_the_declaration() {
    for tool in ["sample-lab", "test-prism"] {
        let description = fixtures::description(tool);
        for (_, result) in fixtures::documents(tool)
            .into_iter()
            .filter(|(id, _)| *id == "urn:table-manners:core:result")
        {
            if result["invocation"].is_null() {
                continue;
            }
            let action = description["actions"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["name"] == result["action"])
                .unwrap();
            let arguments = &result["invocation"]["arguments"];
            let mut parameters = arguments.clone();
            for direction in ["in", "out"] {
                let entries = result["artifacts"][direction].as_array().unwrap();
                if action["kind"] == "operation" {
                    for port in action["ports"][direction].as_array().unwrap() {
                        let name = port["name"].as_str().unwrap();
                        if direction == "in" {
                            if port["required"] == true {
                                assert!(arguments.get(name).is_some());
                            }
                            parameters.as_object_mut().unwrap().remove(name);
                        }
                        let bound: Vec<_> = entries.iter().filter(|e| e["port"] == name).collect();
                        if port["required"] == true {
                            assert!(!bound.is_empty());
                        }
                        if port["many"] != true {
                            assert!(bound.len() <= 1);
                        }
                        if direction == "in" && port["many"] == true && !bound.is_empty() {
                            let refs: Vec<_> =
                                bound.iter().map(|e| e["artifact"].clone()).collect();
                            assert_eq!(arguments[name], serde_json::json!(refs));
                        }
                    }
                    for entry in entries {
                        let port = action["ports"][direction]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|p| p["name"] == entry["port"])
                            .expect("result names a declared port");
                        assert_eq!(entry["artifact"]["type"], port["type"]);
                    }
                }
                for entry in entries {
                    let kind = entry["artifact"]["type"].as_str().unwrap();
                    if direction == "out" && kind != "image" {
                        let declared = description["artifact_types"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|t| t["type"] == kind)
                            .expect("declared artifact type");
                        assert_eq!(entry["preview"].is_null(), declared["preview"] == false);
                    }
                    if direction == "out" && !entry["preview"].is_null() {
                        assert!(
                            result["images"]["out"]
                                .as_array()
                                .unwrap()
                                .contains(&entry["preview"])
                        );
                    }
                    if kind == "image" && direction == "in" {
                        assert!(
                            result["images"]["in"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|i| i["slot"] == entry["port"]
                                    && i["image"] == entry["artifact"]["on"])
                        );
                    }
                    if kind == "location" {
                        let key = if direction == "in" { "used" } else { "found" };
                        assert!(
                            result["locations"][key]
                                .as_array()
                                .unwrap()
                                .contains(&fixtures::artifact(tool, "location")["record"])
                        );
                    }
                }
            }
            for slot in action["images"].as_array().unwrap() {
                let name = slot["slot"].as_str().unwrap();
                if slot["required"] == true {
                    assert!(arguments.get(name).is_some());
                    assert!(
                        result["images"]["in"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|i| i["slot"] == name)
                    );
                }
                parameters.as_object_mut().unwrap().remove(name);
            }
            assert!(jsonschema::is_valid(&action["params_schema"], &parameters));
        }
        let many = &description["actions"][3]["ports"]["in"][0];
        let refusal = fixtures::refusal("wrong_frame", Some("tokens"), Some(1));
        assert_eq!(many["name"], refusal["bindings"][0]["port"]);
        assert_eq!(many["many"], true);
    }
}
