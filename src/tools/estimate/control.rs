//! A control deadline ends waiting in the UI, not ownership of a remote file reader.
use super::{ValidatedEstimate, validate_refusal};
use crate::tools::{
    Error, Result,
    client::{Declaration, ToolClient},
    session::validate_granted_observation,
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::time::{Instant, timeout_at};

impl ToolClient {
    // The caller owns a preparation lease and estimate slot until this future completes.
    // Never cancel this future after dispatch: even a timeout drains its remote request.
    pub(crate) async fn estimate_control(
        &self,
        declaration: &Declaration,
        action: &str,
        arguments: &Value,
        cap: f64,
        deadline: Instant,
    ) -> Result<ValidatedEstimate> {
        let mut previous = timeout_at(deadline, self.observed_session.lock()).await?;
        if Instant::now() >= deadline {
            return Err(Error::Conflict("Estimate deadline elapsed before dispatch"));
        }
        let baseline = previous
            .as_ref()
            .ok_or(Error::Missing("No tool session observation"))?;
        if baseline["id"] != declaration.tool_session["id"]
            || self.generation() != declaration.generation
        {
            return Err(Error::Conflict(
                "Tool session or declaration changed before estimate",
            ));
        }
        let request = serde_json::from_value(json!({"method":"tools/call","params":{
            "name":"estimate","arguments":{"action":action,"arguments":arguments}
        }}))?;
        let call = self.peer()?.send_request(request);
        tokio::pin!(call);
        let response = match timeout_at(deadline, &mut call).await {
            Ok(response) => response,
            Err(timeout) => {
                // New ledger reads may proceed. A late quote never publishes an observation.
                drop(previous);
                self.drain_estimate(&mut call).await;
                return Err(timeout.into());
            }
        };
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                drop(previous);
                if let rmcp::ServiceError::McpError(ref rpc) = error {
                    validate_refusal(rpc)?;
                } else {
                    // A broken pipe or SDK error does not prove the tool stopped reading.
                    self.wait_estimate_child_exit().await;
                }
                return Err(error.into());
            }
        };
        if Instant::now() >= deadline || self.generation() != declaration.generation {
            return Err(Error::Conflict(
                "Estimate expired or the tool declaration changed",
            ));
        }
        let quote = ValidatedEstimate::from_response(
            &serde_json::to_value(response)?,
            &declaration.document["tool"],
            action,
            cap,
            previous.as_ref().unwrap(),
            time::OffsetDateTime::now_utc(),
        )?;
        validate_granted_observation(
            &quote.observation()["session"],
            &declaration.document["tool"],
            &self.descriptor.allow_dirs,
            previous.as_ref(),
        )?;
        if Instant::now() >= deadline {
            return Err(Error::Conflict(
                "Estimate deadline elapsed during validation",
            ));
        }
        self.publish_session(&mut previous, quote.observation()["session"].clone());
        Ok(quote)
    }
    async fn drain_estimate<F>(&self, response: F)
    where
        F: std::future::Future<
                Output = std::result::Result<rmcp::model::ServerResult, rmcp::ServiceError>,
            >,
    {
        tokio::pin!(response);
        loop {
            tokio::select! {
                result = &mut response => {
                    if !matches!(result, Ok(_) | Err(rmcp::ServiceError::McpError(_))) {
                        self.wait_estimate_child_exit().await;
                    }
                    return;
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    if self.has_exited().await.unwrap_or(false) { return; }
                }
            }
        }
    }
    async fn wait_estimate_child_exit(&self) {
        while !self.has_exited().await.unwrap_or(false) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }
}

#[cfg(test)]
#[path = "control_tests.rs"]
mod tests;
