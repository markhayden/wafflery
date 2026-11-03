use super::*;
use wafflery::tools::{
    bundle_reader,
    bundle_step::{self, Input},
};
fn description() -> Value {
    let mut description = fixtures::description("sample");
    let mut action = description["actions"][0].clone();
    action["name"] = json!("rasterize");
    action["ports"]["in"] = json!([
        {"name":"pictures","type":"image","many":true,"required":true,"purpose":"Input pictures"},
        {"name":"markers","type":"location","many":true,"required":true,"purpose":"Input markers"}
    ]);
    action["params_schema"]["properties"]["amount"] =
        json!({"type":"number","minimum":0,"maximum":1});
    description["actions"] = json!([action]);
    description
}
async fn parsed() -> (
    tempfile::TempDir,
    AssetStore,
    bundle_reader::ValidatedBundle,
) {
    let (root, assets, bytes) = super::reader::exported().await;
    (root, assets, bundle_reader::read(bytes).unwrap())
}
#[tokio::test]
async fn current_ports_preserve_repeated_inputs_and_separate_parameters() {
    let (_root, assets, bundle) = parsed().await;
    let plan = bundle_step::prepare(&bundle, 0, &description()).unwrap();
    assert_eq!(plan.action, "rasterize");
    assert_eq!(plan.params, json!({"amount":0.75}));
    assert_eq!(plan.inputs.len(), 4);
    assert_eq!(
        plan.inputs
            .iter()
            .map(|i| (i.name.as_str(), i.index))
            .collect::<Vec<_>>(),
        [
            ("pictures", 0),
            ("pictures", 1),
            ("markers", 0),
            ("markers", 1)
        ]
    );
    assert!(matches!(&plan.inputs[0].input, Input::Image { .. }));
    assert!(matches!(&plan.inputs[2].input, Input::Artifact { .. }));
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn installed_description_catches_hidden_required_inputs_and_changed_parameter_schemas() {
    let (_root, assets, bundle) = parsed().await;
    let mut declaration = description();
    declaration["actions"][0]["ports"]["in"].as_array_mut().unwrap().push(json!({"name":"missing","type":"image","many":false,"required":true,"purpose":"Required input"}));
    assert!(
        bundle_step::prepare(&bundle, 0, &declaration)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
    let mut declaration = description();
    declaration["actions"][0]["params_schema"]["properties"]["amount"]["maximum"] = json!(0.5);
    assert!(bundle_step::prepare(&bundle, 0, &declaration).is_err());
    let mut declaration = description();
    declaration["tool"]["name"] = json!("other");
    assert!(bundle_step::prepare(&bundle, 0, &declaration).is_err());
    assert!(bundle_step::prepare(&bundle, 1, &description()).is_err());
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn changed_port_types_cardinality_and_removed_ports_are_refused_before_import() {
    let (_root, assets, bundle) = parsed().await;
    for edit in 0..4 {
        let mut declaration = description();
        match edit {
            0 => declaration["actions"][0]["ports"]["in"][0]["many"] = json!(false),
            1 => declaration["actions"][0]["ports"]["in"][1]["type"] = json!("sample:Token"),
            2 => {
                declaration["actions"][0]["ports"]["in"]
                    .as_array_mut()
                    .unwrap()
                    .remove(1);
            }
            _ => declaration["artifact_types"][0]["schema_versions"] = json!([2]),
        }
        assert!(
            bundle_step::prepare(&bundle, 0, &declaration).is_err(),
            "edit {edit}"
        );
    }
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn explicit_omissions_remain_unresolved_and_never_reuse_old_references() {
    let (_root, assets, mut attempt, _) = fixture().await;
    attempt.id = "omitted-step".into();
    let bundle = bundles::build(
        &assets,
        &attempt,
        &Options {
            omit: BTreeMap::from([("markers".into(), "Choose replacement".into())]),
        },
    )
    .await
    .unwrap();
    let parsed = bundle_reader::read(std::fs::read(bundle.file.path()).unwrap()).unwrap();
    let plan = bundle_step::prepare(&parsed, 0, &description()).unwrap();
    assert_eq!(plan.inputs.len(), 4);
    assert!(plan.inputs[2..].iter().all(
        |i| matches!(&i.input,Input::Unresolved { reason } if reason == "Choose replacement")
    ));
    assets.database.shutdown().await.unwrap();
}
