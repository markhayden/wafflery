use serde_json::Value;
pub fn archive() -> Value {
    serde_json::from_str(include_str!("../../testkit/contract-v03/schemas.json")).unwrap()
}
struct Reviewed;
impl jsonschema::Retrieve for Reviewed {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let snapshot = archive();
        let text = snapshot["schemas"][uri.as_str()]["text"]
            .as_str()
            .ok_or("unknown reviewed schema")?;
        Ok(serde_json::from_str(text)?)
    }
}
pub fn validator(kind: &str) -> jsonschema::Validator {
    let snapshot = archive();
    let id = format!("urn:table-manners:core:{kind}");
    jsonschema::options()
        .with_retriever(Reviewed)
        .should_validate_formats(true)
        .build(
            &serde_json::from_str::<Value>(snapshot["schemas"][&id]["text"].as_str().unwrap())
                .unwrap(),
        )
        .unwrap()
}
