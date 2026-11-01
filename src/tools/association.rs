use super::{
    Error, Result, client::ToolClient, evidence::resource_content, validate_contract, validator,
};
use crate::library::attempts::Attempt;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Default)]
pub(super) struct ProgressAssociation {
    pub request: String,
    pub token: Option<Value>,
    pub reference: Option<Value>,
    pub error: Option<String>,
}
impl ProgressAssociation {
    pub fn observe(&mut self, progress: &Value) {
        if self.token.as_ref() != progress.get("progressToken") || self.error.is_some() {
            return;
        }
        let reference = &progress["_meta"]["table-manners/run"];
        if reference.is_null() {
            return;
        }
        let checked =
            validator(&json!({"$ref":"urn:table-manners:core:run-ref"})).and_then(|schema| {
                schema
                    .validate(reference)
                    .map_err(|e| Error::Invalid(e.to_string()))
            });
        if checked.is_err() || reference["client_run_id"] != self.request {
            self.error = Some("Progress has an invalid run association".into());
        } else if self.reference.as_ref().is_some_and(|old| old != reference) {
            self.error = Some("Progress changed the associated run".into());
        } else {
            self.reference = Some(reference.clone());
        }
    }
}

pub(super) fn check_reference(attempt: &Attempt, reference: &Value) -> Result<()> {
    let name = attempt.declaration["document"]["tool"]["name"]
        .as_str()
        .unwrap_or("");
    let id = reference["id"]
        .as_str()
        .ok_or_else(|| Error::Invalid("Missing run identity".into()))?;
    if reference["client_run_id"] != attempt.request.id
        || reference["session_id"] != attempt.declaration["tool_session"]["id"]
        || reference["session_id"].is_null()
        || reference["resource"] != format!("{name}://run/{id}/run.json")
    {
        return Err(Error::Invalid(
            "Run correlation, session or resource conflicts with this attempt".into(),
        ));
    }
    if let Some(previous) = attempt.observations.get("run_reference") {
        for key in ["id", "resource", "session_id", "client_run_id"] {
            if previous[key] != reference[key] {
                return Err(Error::Invalid(
                    "Result conflicts with the run associated by progress".into(),
                ));
            }
        }
    }
    Ok(())
}

async fn read_json(client: &ToolClient, uri: &str) -> Result<Value> {
    let response = client.read_resource(uri).await?;
    let (_, bytes) = resource_content(&response, uri, 4 * 1024 * 1024)?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// A bounded search is evidence gathering, never permission to retry an absent run.
pub(super) async fn reconcile(client: &ToolClient, attempt: &Attempt) -> Result<Option<Value>> {
    let tool = &attempt.declaration["document"]["tool"];
    let mut reference = attempt.observations.get("run_reference").cloned();
    if reference.is_none() {
        let mut uri = format!("{}://runs", tool["name"].as_str().unwrap());
        let mut seen = HashSet::new();
        let mut complete = false;
        for _ in 0..16 {
            if !seen.insert(uri.clone()) {
                return Err(Error::Invalid("Run pagination repeated a cursor".into()));
            }
            let page = read_json(client, &uri).await?;
            validate_contract("urn:table-manners:core:runs", &page)?;
            if page["tool"]["name"] != tool["name"] || page["tool"]["version"] != tool["version"] {
                return Err(Error::Invalid(
                    "Runs list belongs to a different tool".into(),
                ));
            }
            for run in page["runs"].as_array().unwrap() {
                if run["session_id"] == attempt.declaration["tool_session"]["id"]
                    && run["client_run_id"] == attempt.id
                {
                    if reference.is_some() {
                        return Err(Error::Invalid("Multiple runs claim this request".into()));
                    }
                    reference = Some(run.clone());
                }
            }
            if page["next_cursor"].is_null() {
                complete = true;
                break;
            }
            uri = format!(
                "{}://runs?cursor={}",
                tool["name"].as_str().unwrap(),
                page["next_cursor"].as_str().unwrap()
            );
        }
        if !complete {
            return Err(Error::Invalid(
                "Run pagination exceeded the bounded lookup; association remains unconfirmed"
                    .into(),
            ));
        }
    }
    let Some(reference) = reference else {
        return Ok(None);
    };
    check_reference(attempt, &reference)?;
    let result = read_json(client, reference["resource"].as_str().unwrap()).await?;
    check_reference(attempt, &result["run"])?;
    for key in ["id", "resource", "session_id", "client_run_id"] {
        if result["run"][key] != reference[key] {
            return Err(Error::Invalid("Run resource changed its identity".into()));
        }
    }
    if result["status"] == "running" {
        return Ok(None);
    }
    validate_contract("urn:table-manners:core:result", &result)?;
    super::evidence::check_association(attempt, &result)?;
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn progress_conflicts_are_sticky_even_when_later_notifications_look_valid() {
        let mut association = ProgressAssociation {
            request: "request".into(),
            token: Some(json!("request")),
            ..Default::default()
        };
        let run = json!({"id":"one","resource":"fixture://run/one/run.json","client_run_id":"request","session_id":"bb8f5520-a822-4cdc-a9eb-f1a72018e000"});
        let progress = json!({"progressToken":"request","_meta":{"table-manners/run":run}});
        association.observe(&progress);
        assert_eq!(association.reference, Some(run));
        let mut conflict = progress.clone();
        conflict["_meta"]["table-manners/run"]["id"] = json!("two");
        association.observe(&conflict);
        association.observe(&progress);
        assert!(association.error.is_some());
    }
}
