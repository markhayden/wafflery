pub mod client;
pub mod registration;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("Tool connection failed: {0}")]
    Mcp(#[from] rmcp::ServiceError),
    #[error("Tool response could not be decoded: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Tool did not answer within the control-request deadline")]
    Timeout(#[from] tokio::time::error::Elapsed),
}
pub type Result<T> = std::result::Result<T, Error>;

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> std::result::Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let (_, text) = table_manners::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or("Only embedded contract schema references are permitted")?;
        Ok(serde_json::from_str(text)?)
    }
}
pub fn validator(schema: &serde_json::Value) -> Result<jsonschema::Validator> {
    jsonschema::options()
        .with_retriever(Embedded)
        .should_validate_formats(true)
        .build(schema)
        .map_err(|e| Error::Invalid(format!("Invalid schema: {e}")))
}
pub fn validate_contract(kind: &str, value: &serde_json::Value) -> Result<()> {
    if value["contract"] != "0.1.0" {
        return Err(Error::Invalid(
            "Unsupported table-manners contract; this build accepts exactly 0.1.0".into(),
        ));
    }
    let (_, text) = table_manners::SCHEMAS
        .iter()
        .find(|(id, _)| *id == kind)
        .ok_or_else(|| Error::Invalid("Unknown contract schema".into()))?;
    validator(&serde_json::from_str(text)?)?
        .validate(value)
        .map_err(|e| Error::Invalid(format!("Invalid contract document: {e}")))
}
