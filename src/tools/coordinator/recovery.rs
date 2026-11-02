use super::{Coordinator, Error, Result, ToolClient};
use serde_json::{Value, json};

impl Coordinator {
    pub(super) async fn recover_rpc(
        &self,
        client: &ToolClient,
        id: &str,
        error: Error,
    ) -> Result<Value> {
        let Error::Mcp(rmcp::ServiceError::McpError(rpc)) = &error else {
            return Err(error);
        };
        let raw = json!({"error":rpc});
        self.attempts
            .update(id, move |attempt| {
                attempt.raw_response = Some(raw);
                Ok(())
            })
            .await?;
        self.record_reference(client, id).await?;
        // Only the action's original RPC response can prove a pre-run refusal.
        // Errors returned by resource recovery must remain uncertain.
        if self.preserve_no_run(client, id, &error).await? {
            return Err(error);
        }
        if rpc.code == rmcp::model::ErrorCode::INVALID_PARAMS
            && let Some(data) = &rpc.data
            && data["reason"] == "client_run_id_reused"
        {
            super::validator(&json!({"$ref":"urn:table-manners:core:refusal"}))?
                .validate(data)
                .map_err(|error| Error::Invalid(error.to_string()))?;
            if !data["run"].is_null() {
                let reference = data["run"].clone();
                let attempt = self.attempts.get(id).await?;
                crate::tools::association::check_reference(&attempt, &reference)?;
                self.attempts
                    .update(id, move |attempt| {
                        attempt.observations["run_reference"] = reference;
                        Ok(())
                    })
                    .await?;
            }
        }
        let attempt = self.attempts.get(id).await?;
        if attempt
            .observations
            .get("run_reference")
            .is_some_and(|reference| !reference.is_null())
        {
            self.attempts.update(id, |attempt| {
                attempt.notices.push("The call returned an RPC error for an associated run. Run-resource recovery was attempted; nothing was replayed.".into());
                Ok(())
            }).await?;
            return self.recover_associated(client, id).await;
        }
        // A duplicate null reference is not evidence of release. The no-run classifier
        // excludes reused claims; transport failures never reach this RPC-only path.
        Err(error)
    }
    pub(super) async fn recover_associated(&self, client: &ToolClient, id: &str) -> Result<Value> {
        let recovery = async {
            let mut last_error = None;
            for _ in 0..20 {
                self.record_reference(client, id).await?;
                let attempt = self.attempts.get(id).await?;
                match crate::tools::association::reconcile(client, &attempt).await {
                    Ok(Some(result)) => {
                        client.begin_preservation();
                        let uri = result["run"]["resource"].clone();
                        self.attempts
                            .update(id, move |attempt| {
                                attempt.observations["reconciled_from"] = uri;
                                Ok(())
                            })
                            .await?;
                        return Ok(result);
                    }
                    Ok(None) => {}
                    Err(Error::Mcp(rmcp::ServiceError::McpError(ref error)))
                        if error.code.0 == -32002 =>
                    {
                        last_error = Some(Error::Invalid(
                            "Associated run resource is not available yet".into(),
                        ));
                    }
                    Err(error) => return Err(error),
                }
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
            Err(last_error.unwrap_or_else(|| {
                Error::Invalid(
                    "Run outcome remains unconfirmed after bounded resource lookup".into(),
                )
            }))
        };
        tokio::time::timeout(std::time::Duration::from_secs(5), recovery).await?
    }
}
