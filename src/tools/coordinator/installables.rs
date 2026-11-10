//! The reserved install and uninstall controls (table-manners 0.6 draft, 5.3). They are never runs:
//! no attempt, no history, no spend, no execution permit. An install can take minutes, so it runs in
//! a task the HTTP request does not own; the tool keeps installing if the browser goes away, and the
//! description's installables (state, progress, last_outcome) are how the page follows it.
use super::{Coordinator, Error, Result, ToolClient};
use crate::tools::validator;
use serde_json::{Value, json};
use std::sync::{Arc, atomic::Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Install,
    Uninstall,
}
impl Control {
    fn name(self) -> &'static str {
        match self {
            Control::Install => "install",
            Control::Uninstall => "uninstall",
        }
    }
}

impl Coordinator {
    /// Starts an install in the background, or calls uninstall and waits for its result. The
    /// installable must be one the current description lists. Returns the started marker for an
    /// install and the tool's result for an uninstall.
    pub async fn installable_control(
        self: &Arc<Self>,
        tool: &str,
        installable: &str,
        control: Control,
    ) -> Result<Value> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        let declaration = self.description(tool).await?;
        let listed = declaration.document["installables"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .any(|item| item["id"] == installable);
        if !listed {
            return Err(Error::Missing("The tool does not list this installable"));
        }
        let arguments = json!({ "id": installable });
        validator(&json!({"$ref":"urn:table-manners:core:install-request"}))?
            .validate(&arguments)
            .map_err(|error| Error::Invalid(format!("Invalid install request: {error}")))?;
        let entry = self
            .tools
            .get(tool)
            .ok_or(Error::Missing("Tool not registered"))?;
        let client = entry
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        match control {
            Control::Uninstall => {
                let result = call(&client, control, arguments).await;
                // The state changed (or the refusal says why not): publish what the tool now reports.
                let _ = self.refresh(tool).await;
                result
            }
            Control::Install => {
                let coordinator = self.clone();
                let owned = tool.to_owned();
                tokio::spawn(async move {
                    // The outcome is read back from the description (last_outcome); a transport
                    // failure is surfaced by the next refresh, never retried here.
                    let _ = call(&client, control, arguments).await;
                    if !coordinator.stopping.load(Ordering::Acquire) {
                        let _ = coordinator.refresh(&owned).await;
                    }
                });
                // Let the tool mark the installable as installing before the page refreshes.
                let _ = self.refresh(tool).await;
                Ok(json!({ "id": installable, "state": "installing" }))
            }
        }
    }
}

/// One reserved control call, its result checked against the contract.
pub(super) async fn call(client: &ToolClient, control: Control, arguments: Value) -> Result<Value> {
    let request = serde_json::from_value(json!({
        "method": "tools/call",
        "params": { "name": control.name(), "arguments": arguments }
    }))?;
    match client.peer()?.send_request(request).await {
        Ok(result) => {
            let response = serde_json::to_value(result)?;
            if response["isError"] == true {
                return Err(Error::Invalid(format!(
                    "The tool's {} control failed without a result",
                    control.name()
                )));
            }
            let body = response
                .get("structuredContent")
                .cloned()
                .ok_or_else(|| Error::Invalid("Install control omitted structuredContent".into()))?;
            validator(&json!({"$ref":"urn:table-manners:core:install-result"}))?
                .validate(&body)
                .map_err(|error| Error::Invalid(format!("Invalid install result: {error}")))?;
            Ok(body)
        }
        Err(rmcp::ServiceError::McpError(error))
            if error.code == rmcp::model::ErrorCode::INVALID_PARAMS && error.data.is_some() =>
        {
            let data = error.data.unwrap_or_default();
            validator(&json!({"$ref":"urn:table-manners:core:refusal"}))?
                .validate(&data)
                .map_err(|error| Error::Invalid(format!("Invalid install refusal: {error}")))?;
            Err(Error::Invalid(match data["reason"].as_str() {
                Some("unknown_installable") => "The tool does not know this installable".into(),
                Some("not_ready") => "The tool cannot install this yet; see its requirements".into(),
                _ => "The tool refused the install control".into(),
            }))
        }
        Err(error) => Err(error.into()),
    }
}
