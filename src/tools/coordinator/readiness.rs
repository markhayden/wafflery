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
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        let refreshed = client.describe().await;
        match &refreshed {
            Ok(current) => self.preparations.reconcile(id, current, &client.session),
            Err(_) => self.preparations.invalidate_tool(id),
        }
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

    pub(super) async fn preserve_no_run(
        &self,
        client: &ToolClient,
        id: &str,
        error: &Error,
    ) -> Result<bool> {
        let saved = self.attempts.get(id).await?;
        let action = saved.declaration["document"]["actions"]
            .as_array()
            .and_then(|actions| {
                actions
                    .iter()
                    .find(|action| action["name"] == saved.request.action)
            })
            .unwrap_or(&Value::Null);
        let Some((raw, data)) = no_run_refusal(
            error,
            client.run_reference()?.is_none(),
            action,
            &saved.arguments,
        ) else {
            return Ok(false);
        };
        self.attempts.update(id, move |attempt| {
            if attempt.result.is_some() || attempt.observations.get("run_reference").is_some_and(|reference| !reference.is_null()) {
                return Err(crate::library::Error::Conflict("A known run cannot become a no-run refusal"));
            }
            attempt.execution = "rejected".into();
            attempt.raw_response = Some(raw);
            attempt.preservation["contract"] = json!("not_applicable");
            attempt.observations["spent_usd"] = json!(0);
            attempt.notices.push("The tool refused this call before creating a run. No spend was incurred; correct the inputs or setup before starting a new attempt.".into());
            for requirement in data["requirements"].as_array().into_iter().flatten() {
                attempt.notices.push(format!("Setup needed: {}. {}", requirement["name"].as_str().unwrap(), requirement["setup"].as_str().unwrap_or("Check the tool’s configuration.")));
            }
            attempt.observations["refusal"] = data;
            Ok(())
        }).await?;
        Ok(true)
    }
}
fn no_run_refusal(
    error: &Error,
    no_run: bool,
    action: &Value,
    arguments: &Value,
) -> Option<(Value, Value)> {
    let Error::Mcp(rmcp::ServiceError::McpError(rpc)) = error else {
        return None;
    };
    if !no_run {
        return None;
    }
    let data = match rpc.data.as_ref().filter(|data| !data.is_null()) {
        None => json!({"reason":"rpc_error","run":null}),
        Some(data) => {
            // Reuse refers to an earlier owner, even when its run is still null.
            // Malformed named refusals are not evidence that a claim was released.
            if rpc.code != rmcp::model::ErrorCode::INVALID_PARAMS
                || !matches!(
                    data["reason"].as_str(),
                    Some("not_ready" | "invalid_arguments" | "binding")
                )
            {
                return None;
            }
            let schema =
                super::validator(&json!({"$ref":"urn:table-manners:core:refusal"})).ok()?;
            if !schema.is_valid(data) {
                return None;
            }
            if data["reason"] == "not_ready"
                && !data["requirements"].as_array()?.iter().all(|requirement| {
                    matches!(requirement["state"].as_str(), Some("missing" | "invalid"))
                })
            {
                return None;
            }
            if data["reason"] == "binding" {
                use crate::tools::binding_refusal_semantics::{Context, validate};
                validate(data, Context::Operation { action, arguments }).ok()?;
            }
            data.clone()
        }
    };
    Some((json!({"error":rpc}), data))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn no_run_refusal(error: &Error, no_run: bool) -> Option<(Value, Value)> {
        super::no_run_refusal(error, no_run, &Value::Null, &Value::Null)
    }
    #[test]
    fn binding_release_requires_the_saved_operation_reference_and_no_known_run() {
        let action = json!({"kind":"operation","ports":{"in":[{"name":"masks","many":true}]}});
        let artifact = json!({"type":"sample-lab:Mask","schema_version":1,"tool":"sample-lab",
            "identity":format!("sha256:{}","1".repeat(64)), "frame":null,
            "resource":"sample-lab://artifact/example"});
        let arguments = json!({"masks":[artifact.clone(),artifact]});
        let data = json!({"reason":"binding","run":null,"bindings":[{"port":"masks","index":1,
            "problem":"unresolved","expected":"sample-lab:Mask v1","got":"missing",
            "remedy":"Import its bundle."}]});
        let error = |data| {
            Error::Mcp(rmcp::ServiceError::McpError(
                rmcp::ErrorData::invalid_params("Cannot bind", Some(data)),
            ))
        };
        let saved = super::no_run_refusal(&error(data.clone()), true, &action, &arguments).unwrap();
        assert_eq!(saved.1, data);
        assert_eq!(saved.0["error"]["data"], data);
        assert!(super::no_run_refusal(&error(data.clone()), false, &action, &arguments).is_none());
        assert!(
            super::no_run_refusal(
                &error(data.clone()),
                true,
                &json!({"kind":"task"}),
                &arguments
            )
            .is_none()
        );
        for invalid in [
            json!({}),
            json!({"masks":[arguments["masks"][0]]}),
            json!({"masks":[arguments["masks"][0],{"path":"/image.png"}]}),
        ] {
            assert!(super::no_run_refusal(&error(data.clone()), true, &action, &invalid).is_none());
        }
        for index in [json!(null), json!(2), json!(-1), json!(0.5)] {
            let mut invalid = data.clone();
            invalid["bindings"][0]["index"] = index;
            assert!(super::no_run_refusal(&error(invalid), true, &action, &arguments).is_none());
        }
    }
    #[test]
    fn only_a_valid_pre_run_not_ready_refusal_is_definitive() {
        let valid = json!({"reason":"not_ready","run":null,"requirements":[{"kind":"credential","name":"KEY","state":"missing"}]});
        let error = |data| {
            Error::Mcp(rmcp::ServiceError::McpError(
                rmcp::ErrorData::invalid_params("needs setup", Some(data)),
            ))
        };
        assert!(no_run_refusal(&error(valid.clone()), true).is_some());
        assert!(no_run_refusal(&error(valid.clone()), false).is_none());
        for state in ["present", "valid", "unknown"] {
            let mut data = valid.clone();
            data["requirements"][0]["state"] = json!(state);
            assert!(no_run_refusal(&error(data), true).is_none());
        }
        for field in ["run", "requirements"] {
            let mut data = valid.clone();
            data.as_object_mut().unwrap().remove(field);
            assert!(no_run_refusal(&error(data), true).is_none());
        }
        assert!(no_run_refusal(&Error::Mcp(rmcp::ServiceError::TransportClosed), true).is_none());
        let mut data = valid;
        data["run"] = json!({"id":"already-started"});
        assert!(no_run_refusal(&error(data), true).is_none());
    }
    #[test]
    fn reused_or_malformed_refusals_never_release_the_original_claim() {
        let error = |data| {
            Error::Mcp(rmcp::ServiceError::McpError(
                rmcp::ErrorData::invalid_params("refused", Some(data)),
            ))
        };
        let valid = json!({"reason":"invalid_arguments","run":null,"error":{"code":"input","reason":"Bad input","retryable":false}});
        assert!(no_run_refusal(&error(valid.clone()), true).is_some());
        assert!(no_run_refusal(&error(valid.clone()), false).is_none());
        let mut retryable = valid.clone();
        retryable["error"]["retryable"] = json!(true);
        let mut provider = valid.clone();
        provider["error"]["code"] = json!("credential");
        for data in [
            retryable,
            provider,
            json!({"reason":"client_run_id_reused","run":null}),
            json!({"reason":"client_run_id_reused","run":{"id":"claimed"}}),
            json!({"reason":"invalid_arguments","run":null}),
            json!({"reason":"unknown","run":null}),
        ] {
            assert!(no_run_refusal(&error(data), true).is_none());
        }
        let wrong_code = Error::Mcp(rmcp::ServiceError::McpError(
            rmcp::ErrorData::internal_error("wrong code", Some(valid)),
        ));
        assert!(no_run_refusal(&wrong_code, true).is_none());
        let plain = Error::Mcp(rmcp::ServiceError::McpError(
            rmcp::ErrorData::invalid_params("bad input", None),
        ));
        assert!(no_run_refusal(&plain, true).is_some());
        assert!(no_run_refusal(&plain, false).is_none());
    }
}
