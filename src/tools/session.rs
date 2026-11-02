use super::{Error, Result, client::ToolClient, evidence::resource_content, validate_contract};
use serde_json::Value;

pub(super) fn validate_observation(
    value: &Value,
    tool: &Value,
    previous: Option<&Value>,
) -> Result<()> {
    validate_contract("urn:table-manners:core:session", value)?;
    if value["tool"]["name"] != tool["name"] || value["tool"]["version"] != tool["version"] {
        return Err(Error::Invalid("Session belongs to a different tool".into()));
    }
    let spent = value["spent_usd"].as_f64().unwrap();
    let reserved = value["reserved_usd"].as_f64().unwrap();
    if value["unresolved_usd"].as_f64().unwrap() > spent + 1e-6 {
        return Err(Error::Invalid(
            "Session unresolved spend exceeds total spend".into(),
        ));
    }
    if let Some(budget) = value["budget_usd"].as_f64()
        && (value["remaining_usd"].as_f64().unwrap() - (budget - spent - reserved).max(0.0)).abs()
            > 1e-6
    {
        return Err(Error::Invalid(
            "Session remaining budget conflicts with its ledger".into(),
        ));
    }
    if let Some(previous) = previous {
        if previous["id"] != value["id"] {
            return Err(Error::Invalid("Tool session changed. Reconnect explicitly before running again; previous spending is not reset.".into()));
        }
        if previous["started"] != value["started"] || previous["budget_usd"] != value["budget_usd"]
        {
            return Err(Error::Invalid(
                "Tool changed its ledger configuration without a new session".into(),
            ));
        }
    }
    Ok(())
}

impl ToolClient {
    /// Caller holds observed_session through validation and publication. Snapshot reads use
    /// this separate short-lived lock, so a pending RPC never rolls the display backward.
    pub(super) fn publish_session(&self, previous: &mut Option<Value>, value: Value) {
        *previous = Some(value.clone());
        *self.last_session.write().unwrap_or_else(|e| e.into_inner()) = Some(value);
    }

    /// Observations are serialized so concurrent control requests cannot move the ledger backward.
    pub async fn observe_session(&self, tool: &Value) -> Result<Value> {
        let mut previous = self.observed_session.lock().await;
        let name = tool["name"]
            .as_str()
            .ok_or_else(|| Error::Invalid("Missing tool name".into()))?;
        let uri = format!("{name}://session");
        let response = self.read_resource(&uri).await?;
        let (_, bytes) = resource_content(&response, &uri, 64 * 1024)?;
        let value = serde_json::from_slice(&bytes)?;
        validate_observation(&value, tool, previous.as_ref())?;
        self.publish_session(&mut previous, value.clone());
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn observation() -> Value {
        json!({"contract":"0.4.0","tool":{"name":"fixture","version":"1"},
            "id":"bb8f5520-a822-4cdc-a9eb-f1a72018e000","started":"2026-01-01T00:00:00Z",
            "allowed_dirs":[],"budget_usd":2,"spent_usd":0.16,"unresolved_usd":0.04,"reserved_usd":0.04,"remaining_usd":1.8})
    }
    #[test]
    fn checks_atomic_budget_arithmetic_and_session_continuity() {
        let first = observation();
        let tool = &first["tool"];
        validate_observation(&first, tool, None).unwrap();
        for (key, value) in [
            ("remaining_usd", json!(2)),
            ("unresolved_usd", json!(1)),
            ("id", json!("cc8f5520-a822-4cdc-a9eb-f1a72018e000")),
            ("budget_usd", json!(3)),
        ] {
            let mut changed = first.clone();
            changed[key] = value;
            assert!(
                validate_observation(&changed, tool, Some(&first)).is_err(),
                "{key}"
            );
        }
        let mut uncapped = first.clone();
        uncapped["budget_usd"] = Value::Null;
        uncapped["remaining_usd"] = Value::Null;
        validate_observation(&uncapped, tool, None).unwrap();
        uncapped["remaining_usd"] = json!(0);
        assert!(validate_observation(&uncapped, tool, None).is_err());
    }
}
