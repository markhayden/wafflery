use super::{Coordinator, Declaration, Error, Result, ToolClient};
use serde_json::{Value, json};
use std::sync::atomic::Ordering;

impl Coordinator {
    /// Discovery is a control request, so an active execution never holds its permit.
    pub async fn refresh(&self, id: &str) -> Result<Declaration> {
        let entry = self
            .tools
            .get(id)
            .ok_or(Error::Missing("Tool not registered"))?;
        let _discovery = entry.discovery.lock().await;
        self.refresh_under_guard(id).await
    }
    /// Caller holds this entry's discovery guard through publication.
    pub(super) async fn refresh_under_guard(&self, id: &str) -> Result<Declaration> {
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        let entry = self
            .tools
            .get(id)
            .ok_or(Error::Missing("Tool not registered"))?;
        let client = entry
            .client
            .as_ref()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        let refreshed = client.describe().await;
        {
            let mut state = entry.state.write().unwrap_or_else(|e| e.into_inner());
            match &refreshed {
                Ok(value) => {
                    state.declaration = Some(value.clone());
                    state.error = client.setup_notice(value);
                }
                Err(error) => {
                    state.declaration = None;
                    state.error = Some(error.to_string());
                }
            }
        }
        self.events.changed();
        refreshed
    }

    pub(super) async fn preserve_not_ready(
        &self,
        client: &ToolClient,
        id: &str,
        error: &Error,
    ) -> Result<bool> {
        let Some((raw, data)) = not_ready(error, client.run_reference()?.is_none()) else {
            return Ok(false);
        };
        self.attempts.update(id, move |attempt| {
            if attempt.result.is_some() || attempt.observations.get("run_reference").is_some_and(|reference| !reference.is_null()) {
                return Err(crate::library::Error::Conflict("A known run cannot become a no-run refusal"));
            }
            attempt.execution = "rejected".into();
            attempt.raw_response = Some(raw);
            attempt.preservation["contract"] = json!("not_applicable");
            attempt.notices.push("The tool refused this call before creating a run. Update the required setup or omit the optional input.".into());
            for requirement in data["requirements"].as_array().unwrap() {
                attempt.notices.push(format!("Setup needed: {}. {}", requirement["name"].as_str().unwrap(), requirement["setup"].as_str().unwrap_or("Check the tool’s configuration.")));
            }
            attempt.observations["refusal"] = data;
            Ok(())
        }).await?;
        Ok(true)
    }
}
fn not_ready(error: &Error, no_run: bool) -> Option<(Value, Value)> {
    let Error::Mcp(rmcp::ServiceError::McpError(rpc)) = error else {
        return None;
    };
    let data = rpc.data.as_ref()?;
    if !no_run
        || rpc.code != rmcp::model::ErrorCode::INVALID_PARAMS
        || data["reason"] != "not_ready"
    {
        return None;
    }
    let schema = super::validator(&json!({"$ref":"urn:table-manners:core:refusal"})).ok()?;
    if !schema.is_valid(data)
        || !data["requirements"]
            .as_array()?
            .iter()
            .all(|requirement| matches!(requirement["state"].as_str(), Some("missing" | "invalid")))
    {
        return None;
    }
    Some((json!({"error":rpc}), data.clone()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_a_valid_pre_run_not_ready_refusal_is_definitive() {
        let valid = json!({"reason":"not_ready","run":null,"requirements":[{"kind":"credential","name":"KEY","state":"missing"}]});
        let error = |data| {
            Error::Mcp(rmcp::ServiceError::McpError(
                rmcp::ErrorData::invalid_params("needs setup", Some(data)),
            ))
        };
        assert!(not_ready(&error(valid.clone()), true).is_some());
        assert!(not_ready(&error(valid.clone()), false).is_none());
        for state in ["present", "valid", "unknown"] {
            let mut data = valid.clone();
            data["requirements"][0]["state"] = json!(state);
            assert!(not_ready(&error(data), true).is_none());
        }
        for field in ["run", "requirements"] {
            let mut data = valid.clone();
            data.as_object_mut().unwrap().remove(field);
            assert!(not_ready(&error(data), true).is_none());
        }
        assert!(not_ready(&Error::Mcp(rmcp::ServiceError::TransportClosed), true).is_none());
        let mut data = valid;
        data["run"] = json!({"id":"already-started"});
        assert!(not_ready(&error(data), true).is_none());
    }
}
