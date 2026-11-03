use serde_json::{Value, json};
use wafflery::tools::chain_edges;
fn image() -> Value {
    json!({"file_sha256":"a".repeat(64),"width":160,"height":100,"projection":"flat","topology":"bounded"})
}
fn reference(kind: &str) -> Value {
    json!({"type":kind,"schema_version":1,"tool":"sample","identity":format!("sha256:{}","a".repeat(64)),"frame":"opaque-grid","on":image(),"resource":format!("sample://artifact/sha256:{}","a".repeat(64))})
}
fn step(arguments: Value, inputs: Value, artifacts: Value, images: Value) -> Value {
    json!({"invocation":{"arguments":arguments},"inputs":inputs,"artifacts":[],"reference_only":[],"unportable":[],"result":{"tool":{"name":"sample"},"artifacts":{"in":[],"out":artifacts},"images":{"in":[],"out":images}}})
}
fn chain(steps: Vec<Value>) -> Value {
    json!({"steps":steps})
}
#[test]
fn native_edges_keep_both_consumer_occurrences_and_ignore_literal_location_values() {
    let token = reference("sample:Token");
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"token","artifact":token}]),
        json!([]),
    );
    let consumer = step(
        json!({"tokens":[token,token],"marker":{"kind":"line","image":image()}}),
        json!([]),
        json!([]),
        json!([]),
    );
    let mut consumer = consumer;
    consumer["result"]["artifacts"]["in"] = json!([{"port":"tokens","artifact":token},{"port":"tokens","artifact":token},{"port":"marker","artifact":reference("location")}]);
    let edges = chain_edges::resolve(&chain(vec![producer, consumer])).unwrap();
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].consumer, 1);
    assert_eq!(edges[0].name, "tokens");
    assert_eq!(edges[0].index, 0);
    assert_eq!(edges[1].index, 1);
    assert_eq!(edges[0].producer.step, 0);
    assert_eq!(edges[0].producer.port, "token");
    assert_eq!(edges[0].producer.item, 0);
}
#[test]
fn earlier_only_and_multiple_matching_outputs_are_ambiguous_even_with_equal_identities() {
    let token = reference("sample:Token");
    let mut consumer = step(json!({"token":token}), json!([]), json!([]), json!([]));
    consumer["result"]["artifacts"]["in"] = json!([{"port":"token","artifact":token}]);
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"first","artifact":token}]),
        json!([]),
    );
    assert!(
        chain_edges::resolve(&chain(vec![consumer.clone(), producer.clone()]))
            .unwrap()
            .is_empty()
    );
    let mut duplicated = producer.clone();
    duplicated["result"]["artifacts"]["out"]
        .as_array_mut()
        .unwrap()
        .push(json!({"port":"second","artifact":token}));
    for producers in [
        vec![duplicated, consumer.clone()],
        vec![producer.clone(), producer, consumer],
    ] {
        let error = chain_edges::resolve(&chain(producers))
            .unwrap_err()
            .to_string();
        assert!(error.contains("ambiguous"));
        assert!(error.contains("single-step"));
    }
}
#[test]
fn mirrored_image_profiles_are_one_output_but_different_ports_or_items_stay_distinct() {
    let r = reference("image");
    let picture = image();
    let mut producer = step(
        json!({}),
        json!([]),
        json!([{"port":"picture","artifact":r}]),
        json!([{"role":"picture","image":picture,"resource":"sample://picture"}]),
    );
    producer["result"]["artifacts"]["out"][0]["preview"] =
        producer["result"]["images"]["out"][0].clone();
    let consumer = step(
        json!({"source":{"expect":picture}}),
        json!([{"slot":"source","image":picture,"member":"inputs/a.png"}]),
        json!([]),
        json!([]),
    );
    let edges = chain_edges::resolve(&chain(vec![producer.clone(), consumer.clone()])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].producer.aliases.len(), 2);
    producer["result"]["images"]["out"][0]["role"] = json!("other");
    assert!(chain_edges::resolve(&chain(vec![producer.clone(), consumer.clone()])).is_err());
    producer["result"]["images"]["out"] = json!([]);
    producer["result"]["artifacts"]["out"]
        .as_array_mut()
        .unwrap()
        .push(json!({"port":"picture","artifact":r}));
    assert!(chain_edges::resolve(&chain(vec![producer, consumer])).is_err());
}
#[test]
fn image_profile_edges_keep_port_local_item_and_global_profile_occurrence() {
    let picture = image();
    let mut other = picture.clone();
    other["file_sha256"] = json!("b".repeat(64));
    let producer = step(
        json!({}),
        json!([]),
        json!([]),
        json!([{"role":"unrelated","image":other,"resource":"sample://other"},{"role":"pictures","image":picture,"resource":"sample://pictures"}]),
    );
    let consumer = step(
        json!({"source":{"expect":picture},"private":{"expect":picture}}),
        json!([{"port":"source","image":picture,"member":"inputs/a.png"}]),
        json!([]),
        json!([]),
    );
    let edges = chain_edges::resolve(&chain(vec![producer, consumer])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].producer.port, "pictures");
    assert_eq!(edges[0].producer.item, 0);
    assert_eq!(edges[0].producer.aliases[0].index, 1);
}
#[test]
fn frames_and_native_types_must_match_while_external_inputs_have_no_edge() {
    let token = reference("sample:Token");
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"token","artifact":token}]),
        json!([]),
    );
    for edit in 0..2 {
        let mut r = token.clone();
        if edit == 0 {
            r["type"] = json!("sample:Other");
        } else {
            r["frame"] = json!("different-grid");
        }
        let mut consumer = step(json!({"token":r}), json!([]), json!([]), json!([]));
        consumer["result"]["artifacts"]["in"] = json!([{"port":"token","artifact":r}]);
        assert!(
            chain_edges::resolve(&chain(vec![producer.clone(), consumer]))
                .unwrap()
                .is_empty()
        );
    }
    let mut changed = reference("image");
    changed["on"]["topology"] = json!("wrap");
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"picture","artifact":changed}]),
        json!([]),
    );
    let consumer = step(
        json!({"image":{"expect":image()}}),
        json!([{"port":"image","image":image(),"member":"inputs/a.png"}]),
        json!([]),
        json!([]),
    );
    assert!(
        chain_edges::resolve(&chain(vec![producer, consumer]))
            .unwrap()
            .is_empty()
    );
}
#[test]
fn an_image_artifact_ref_without_root_metadata_can_name_an_images_profile_output() {
    let picture = image();
    let mut r = reference("image");
    r.as_object_mut().unwrap().remove("on");
    let producer = step(
        json!({}),
        json!([]),
        json!([]),
        json!([{"role":"result","image":picture,"resource":"sample://result"}]),
    );
    let mut consumer = step(json!({"input":r}), json!([]), json!([]), json!([]));
    consumer["result"]["artifacts"]["in"] = json!([{"port":"input","artifact":r}]);
    assert_eq!(
        chain_edges::resolve(&chain(vec![producer, consumer]))
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn optional_root_hints_do_not_hide_native_edges_and_image_owners_do_not_cross() {
    let token = reference("sample:Token");
    let mut argument = token.clone();
    argument.as_object_mut().unwrap().remove("on");
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"token","artifact":token}]),
        json!([]),
    );
    let mut consumer = step(json!({"input":argument}), json!([]), json!([]), json!([]));
    consumer["result"]["artifacts"]["in"] = json!([{"port":"input","artifact":argument}]);
    assert_eq!(
        chain_edges::resolve(&chain(vec![producer, consumer]))
            .unwrap()
            .len(),
        1
    );
    let picture = image();
    let mut r = reference("image");
    r["tool"] = json!("other");
    r["resource"] = json!(format!("other://artifact/sha256:{}", "a".repeat(64)));
    let producer = step(
        json!({}),
        json!([]),
        json!([]),
        json!([{"role":"result","image":picture,"resource":"sample://result"}]),
    );
    let mut consumer = step(json!({"input":r}), json!([]), json!([]), json!([]));
    consumer["result"]["artifacts"]["in"] = json!([{"port":"input","artifact":r}]);
    assert!(
        chain_edges::resolve(&chain(vec![producer, consumer]))
            .unwrap()
            .is_empty()
    );
}
#[test]
fn explicit_preview_links_coalesce_roles_even_when_the_artifact_port_has_a_different_name() {
    let picture = image();
    let preview = json!({"role":"result","image":picture,"resource":"sample://result"});
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"picture","artifact":reference("image"),"preview":preview}]),
        json!([preview]),
    );
    let consumer = step(
        json!({"source":{"expect":picture}}),
        json!([{"slot":"source","image":picture,"member":"inputs/a.png"}]),
        json!([]),
        json!([]),
    );
    let edges = chain_edges::resolve(&chain(vec![producer.clone(), consumer.clone()])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].producer.aliases[1].port, "result");
    let mut duplicate = producer;
    duplicate["result"]["images"]["out"]
        .as_array_mut()
        .unwrap()
        .push(preview);
    assert!(chain_edges::resolve(&chain(vec![duplicate, consumer])).is_err());
}
fn saved(id: &str, result: Value, lineage: Value) -> wafflery::library::attempts::Attempt {
    serde_json::from_value(json!({"id":id,"sequence":1,"created_ms":0,"request":{"id":id,"tool":"sample","action":"test","revision":"r","params":{},"images":{},"lineage":lineage},"arguments":{},"declaration":{},"inputs":{},"wire":null,"execution":"completed","result":result,"raw_response":null,"observations":{},"preservation":{},"outputs":[],"journal":null,"notices":[]})).unwrap()
}
#[test]
fn exporting_edges_requires_the_recorded_selected_producer_not_a_coincidental_equal_identity() {
    let token = reference("sample:Token");
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"token","artifact":token}]),
        json!([]),
    );
    let mut consumer = step(json!({"input":token}), json!([]), json!([]), json!([]));
    consumer["result"]["artifacts"]["in"] = json!([{"port":"input","artifact":token}]);
    let manifest = chain(vec![producer.clone(), consumer.clone()]);
    let selected = vec![
        saved("producer", producer["result"].clone(), json!({})),
        saved(
            "consumer",
            consumer["result"].clone(),
            json!({"input":[{"attempt":"producer","port":"token","occurrence":0}]}),
        ),
    ];
    assert_eq!(
        chain_edges::check_selection(&manifest, &selected)
            .unwrap()
            .len(),
        1
    );
    for lineage in [
        json!({}),
        json!({"input":[{"attempt":"unselected-source","port":"token","occurrence":0}]}),
        json!({"input":[{"attempt":"producer","port":"other","occurrence":0}]}),
        json!({"input":[{"example":"external-example","port":"token","occurrence":0}]}),
    ] {
        let mut selected = selected.clone();
        selected[1].request.lineage = serde_json::from_value(lineage).unwrap();
        assert!(
            chain_edges::check_selection(&manifest, &selected)
                .unwrap_err()
                .to_string()
                .contains("single-step")
        );
    }
    let reversed = chain(vec![consumer, producer]);
    let reversed_selection = vec![selected[1].clone(), selected[0].clone()];
    assert!(chain_edges::check_selection(&reversed, &reversed_selection).is_err());
}
#[test]
fn mirrored_image_profile_lineage_uses_its_actual_role_and_index_not_the_artifact_index() {
    let picture = image();
    let mut other = picture.clone();
    other["file_sha256"] = json!("b".repeat(64));
    let preview = json!({"role":"result","image":picture,"resource":"sample://result"});
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"picture","artifact":reference("image"),"preview":preview}]),
        json!([{"role":"other","image":other,"resource":"sample://other"},preview]),
    );
    let consumer = step(
        json!({"input":{"expect":picture}}),
        json!([{"port":"input","image":picture,"member":"inputs/a.png"}]),
        json!([]),
        json!([]),
    );
    let selected = vec![
        saved("producer", producer["result"].clone(), json!({})),
        saved(
            "consumer",
            consumer["result"].clone(),
            json!({"input":[{"attempt":"producer","port":"result","occurrence":1}]}),
        ),
    ];
    assert_eq!(
        chain_edges::check_selection(&chain(vec![producer, consumer]), &selected)
            .unwrap()
            .len(),
        1
    );
}
#[test]
fn by_value_locations_stay_literal_even_when_their_origin_is_selected() {
    let r = reference("location");
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"marker","artifact":r}]),
        json!([]),
    );
    let mut consumer = step(
        json!({"marker":{"kind":"line","image":image()}}),
        json!([]),
        json!([]),
        json!([]),
    );
    consumer["result"]["artifacts"]["in"] = json!([{"port":"marker","artifact":r}]);
    let selected = vec![
        saved("producer", producer["result"].clone(), json!({})),
        saved(
            "consumer",
            consumer["result"].clone(),
            json!({"marker":[{"attempt":"producer","port":"marker","occurrence":0}]}),
        ),
    ];
    assert!(
        chain_edges::check_selection(&chain(vec![producer, consumer]), &selected)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn pixel_digest_hints_and_integral_number_spellings_do_not_change_image_edges_or_mirrors() {
    let picture = image();
    let mut hinted = picture.clone();
    hinted["pixel_sha256"] = json!("b".repeat(64));
    hinted["width"] = json!(160.0);
    let producer = step(
        json!({}),
        json!([]),
        json!([]),
        json!([{"role":"result","image":hinted,"resource":"sample://result"}]),
    );
    let consumer = step(
        json!({"input":{"expect":picture}}),
        json!([{"slot":"input","image":picture,"member":"inputs/a.png"}]),
        json!([]),
        json!([]),
    );
    assert_eq!(
        chain_edges::resolve(&chain(vec![producer, consumer.clone()]))
            .unwrap()
            .len(),
        1
    );
    let preview = json!({"role":"result","image":hinted,"resource":"sample://result"});
    let mut mirrored = preview.clone();
    mirrored["image"]["pixel_sha256"] = json!("c".repeat(64));
    let producer = step(
        json!({}),
        json!([]),
        json!([{"port":"picture","artifact":reference("image"),"preview":preview}]),
        json!([mirrored]),
    );
    let edges = chain_edges::resolve(&chain(vec![producer, consumer])).unwrap();
    assert_eq!(edges.len(), 1);
    assert_eq!(edges[0].producer.aliases.len(), 2);
}
#[test]
fn an_overlarge_connection_search_refuses_instead_of_blocking_review() {
    let row = json!({"port":"token","artifact":reference("sample:Token")});
    let producer = step(json!({}), json!([]), json!(vec![row; 2048]), json!([]));
    let mut consumer = step(
        json!({"tokens":vec![reference("sample:Other");513]}),
        json!([]),
        json!([]),
        json!([]),
    );
    consumer["result"]["artifacts"]["in"] =
        json!([{"port":"tokens","artifact":reference("sample:Other")}]);
    assert!(
        chain_edges::resolve(&chain(vec![producer, consumer]))
            .unwrap_err()
            .to_string()
            .contains("connection search")
    );
}

#[test]
fn mirrored_profile_search_uses_the_same_work_budget() {
    let producer = step(
        json!({}),
        json!([]),
        json!(vec![
            json!({"port":"picture","artifact":reference("image"),"preview":null});
            2048
        ]),
        json!(vec![
            json!({"role":"picture","image":image(),"resource":"sample://picture"});
            513
        ]),
    );
    assert!(
        chain_edges::resolve(&chain(vec![producer]))
            .unwrap_err()
            .to_string()
            .contains("connection search")
    );
}
