use serde_json::Value;
struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let (_, text) = table_manners::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or("unknown contract schema")?;
        Ok(serde_json::from_str(text)?)
    }
}
pub fn validator(kind: &str) -> jsonschema::Validator {
    let id = format!("urn:table-manners:core:{kind}");
    let (_, text) = table_manners::SCHEMAS
        .iter()
        .find(|(key, _)| *key == id)
        .unwrap();
    jsonschema::options()
        .with_retriever(Embedded)
        .should_validate_formats(true)
        .build(&serde_json::from_str::<Value>(text).unwrap())
        .unwrap()
}
