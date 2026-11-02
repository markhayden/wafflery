use super::*;
use crate::tools::estimate::ValidatedEstimate;
use serde_json::Value;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(crate) struct EstimateTicket(Arc<AtomicBool>);
impl EstimateTicket {
    pub fn invalidate(&self) {
        self.0.store(true, Ordering::Release);
    }
}
#[derive(Clone, serde::Serialize)]
pub struct PreparedEstimate {
    pub preparation: Preparation,
    pub estimate: Value,
}
impl Retained {
    pub fn begin_estimate(
        &self,
        id: &str,
        current: &Declaration,
        session: &str,
    ) -> Result<(Arc<PreparedCall>, EstimateTicket)> {
        let call = self.lease(id, current, session)?;
        let mut state = self.lock();
        let draft = state
            .drafts
            .get_mut(id)
            .filter(|draft| Arc::ptr_eq(&draft.call, &call))
            .ok_or(Error::Missing("Preparation is missing or expired"))?;
        let ticket = EstimateTicket(Arc::new(AtomicBool::new(false)));
        // Only an accepted re-estimation clears the old quote. A rejected busy/API request does not.
        draft.quote = None;
        draft.estimate_ticket = Some(ticket.clone());
        Ok((call, ticket))
    }
    pub fn save_estimate(
        &self,
        call: &Arc<PreparedCall>,
        ticket: &EstimateTicket,
        current: &Declaration,
        session: &str,
        quote: ValidatedEstimate,
    ) -> Result<PreparedEstimate> {
        let mut state = self.lock();
        let draft = state
            .drafts
            .get_mut(&call.request.id)
            .filter(|draft| Arc::ptr_eq(&draft.call, call) && matches(call, current, session))
            .ok_or(Error::Missing("Preparation is missing, expired or changed"))?;
        if !draft
            .estimate_ticket
            .as_ref()
            .is_some_and(|active| Arc::ptr_eq(&active.0, &ticket.0))
            || ticket.0.load(Ordering::Acquire)
            || quote.expired()
        {
            return Err(Error::Missing("Estimate is expired or superseded"));
        }
        draft.quote = Some(quote);
        view(draft)
    }
    pub fn estimate(&self, id: &str) -> Result<PreparedEstimate> {
        let mut state = self.lock();
        let draft = state
            .drafts
            .get_mut(id)
            .ok_or(Error::Missing("Preparation is missing or expired"))?;
        let result = view(draft);
        if result.is_err() {
            draft.quote = None;
        }
        result
    }
}
fn view(draft: &Draft) -> Result<PreparedEstimate> {
    let quote = draft
        .quote
        .as_ref()
        .filter(|quote| !quote.expired())
        .ok_or(Error::Missing("No current estimate for this preparation"))?;
    if draft
        .estimate_ticket
        .as_ref()
        .is_none_or(|ticket| ticket.0.load(Ordering::Acquire))
    {
        return Err(Error::Missing("Estimate request timed out"));
    }
    let value = quote.observation();
    let estimate = serde_json::json!({
        "effective": {"model":value["effective"]["model"],"config_identity":value["effective"]["config_identity"]},
        "worst_case_usd":value["worst_case_usd"],"cap_usd":value["cap_usd"],
        "allowed":value["allowed"],"refused":value["refused"],"readiness":value["readiness"],
        "session":value["session"],"expires":value["expires"]
    });
    Ok(PreparedEstimate {
        preparation: draft.receipt.clone(),
        estimate,
    })
}

#[cfg(test)]
use crate::tools::fixtures;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    async fn fixture() -> (
        tempfile::TempDir,
        crate::library::assets::AssetStore,
        Submission,
        Declaration,
        Retained,
    ) {
        let (dir, assets, request, mut declaration) = super::super::super::tests::fixture().await;
        declaration.document["tool"] = json!({"name":"fixture","version":"0.1.0"});
        declaration.tool_session = fixtures::session("fixture");
        let retained = Retained::new();
        retained
            .insert(
                PreparedCall::new(&assets, request.clone(), &declaration, "session")
                    .await
                    .unwrap(),
            )
            .unwrap();
        (dir, assets, request, declaration, retained)
    }
    fn quote(declaration: &Declaration) -> ValidatedEstimate {
        let mut value = fixtures::estimate("fixture", &declaration.tool_session, None);
        value["action"] = json!("mark");
        value["effective"]["x-fixture"] = json!({"path":"/private/input","binding":"private"});
        value["expires"] = json!(
            (time::OffsetDateTime::now_utc() + time::Duration::minutes(1))
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap()
        );
        ValidatedEstimate::validate(
            value,
            &declaration.document["tool"],
            "mark",
            0.5,
            &declaration.tool_session,
            time::OffsetDateTime::now_utc(),
        )
        .unwrap()
    }
    #[tokio::test]
    async fn quote_reads_preserve_lifetimes_and_omit_bindings_paths_and_extensions() {
        let (_dir, assets, request, declaration, retained) = fixture().await;
        let (lease, ticket) = retained
            .begin_estimate(&request.id, &declaration, "session")
            .unwrap();
        let view = retained
            .save_estimate(
                &lease,
                &ticket,
                &declaration,
                "session",
                quote(&declaration),
            )
            .unwrap();
        let first = serde_json::to_value(&view).unwrap();
        assert!(!first.to_string().contains("binding"));
        assert!(!first.to_string().contains("/private/input"));
        assert!(first["estimate"]["effective"].get("x-fixture").is_none());
        assert_eq!(
            first,
            serde_json::to_value(retained.estimate(&request.id).unwrap()).unwrap()
        );
        tokio::time::pause();
        tokio::time::advance(Duration::from_secs(61)).await;
        assert!(retained.estimate(&request.id).is_err());
        tokio::time::resume();
        drop(lease);
        retained.clear();
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn timeout_and_reestimation_cannot_leave_or_erase_another_requests_quote() {
        let (_dir, assets, request, declaration, retained) = fixture().await;
        let (lease, old) = retained
            .begin_estimate(&request.id, &declaration, "session")
            .unwrap();
        retained
            .save_estimate(&lease, &old, &declaration, "session", quote(&declaration))
            .unwrap();
        old.invalidate();
        assert!(retained.estimate(&request.id).is_err());
        let (next_lease, next) = retained
            .begin_estimate(&request.id, &declaration, "session")
            .unwrap();
        assert!(retained.estimate(&request.id).is_err());
        assert!(
            retained
                .save_estimate(&lease, &old, &declaration, "session", quote(&declaration))
                .is_err()
        );
        retained
            .save_estimate(
                &next_lease,
                &next,
                &declaration,
                "session",
                quote(&declaration),
            )
            .unwrap();
        old.invalidate();
        assert!(retained.estimate(&request.id).is_ok());
        let (third_lease, _third) = retained
            .begin_estimate(&request.id, &declaration, "session")
            .unwrap();
        assert!(
            retained.estimate(&request.id).is_err(),
            "an accepted re-estimation clears even an allowed quote"
        );
        assert!(
            retained
                .save_estimate(
                    &next_lease,
                    &next,
                    &declaration,
                    "session",
                    quote(&declaration)
                )
                .is_err()
        );
        drop(third_lease); // A failed control leaves no quote to fall back to.
        retained.discard(&request.id);
        assert!(
            retained
                .save_estimate(
                    &next_lease,
                    &next,
                    &declaration,
                    "session",
                    quote(&declaration)
                )
                .is_err()
        );
        drop(lease);
        drop(next_lease);
        assets.database.shutdown().await.unwrap();
    }
}
