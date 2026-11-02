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
pub(super) fn view_for_dispatch(draft: &Draft, current: &Declaration) -> Result<()> {
    current_quote(draft)?.authorize(
        &current.document["tool"],
        &draft.call.request.action,
        draft.call.arguments(),
        &current.tool_session,
    )
}
fn current_quote(draft: &Draft) -> Result<&ValidatedEstimate> {
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
    Ok(quote)
}
fn view(draft: &Draft) -> Result<PreparedEstimate> {
    let quote = current_quote(draft)?;
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
    async fn paid_fixture() -> (
        tempfile::TempDir,
        crate::library::assets::AssetStore,
        Submission,
        Declaration,
        Retained,
    ) {
        let (dir, assets, mut request, mut declaration) =
            super::super::super::tests::fixture().await;
        request.params["max_spend_usd"] = json!(0.5);
        declaration.document["tool"] = json!({"name":"fixture","version":"0.1.0"});
        declaration.document["actions"][0]["cost"] = json!({"may_spend":true});
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
    fn revised_quote(declaration: &Declaration, scenario: &str) -> ValidatedEstimate {
        let mut value = quote(declaration).observation().clone();
        match scenario {
            "refused" => {
                value["allowed"] = json!(false);
                value["refused"] = json!("cap");
                value["cap_usd"] = json!(0.01);
            }
            "cap" => value["cap_usd"] = json!(0.6),
            "session" => {
                value["session_id"] = json!("33333333-3333-4333-8333-333333333333");
                value["session"]["id"] = value["session_id"].clone();
            }
            "action" => value["action"] = json!("another"),
            "tool" => {
                value["tool"]["name"] = json!("another");
                value["session"]["tool"] = value["tool"].clone();
            }
            "version" => {
                value["tool"]["version"] = json!("9.0.0");
                value["session"]["tool"] = value["tool"].clone();
            }
            _ => (),
        }
        ValidatedEstimate::validate(
            value.clone(),
            &value["tool"],
            value["action"].as_str().unwrap(),
            value["cap_usd"].as_f64().unwrap(),
            &value["session"],
            time::OffsetDateTime::now_utc(),
        )
        .unwrap()
    }
    #[tokio::test]
    async fn priced_consumption_rejects_refused_foreign_expired_and_invalidated_quotes() {
        for scenario in [
            "refused",
            "cap",
            "session",
            "action",
            "tool",
            "version",
            "expired",
            "invalidated",
        ] {
            let (_dir, assets, request, declaration, retained) = paid_fixture().await;
            let (lease, ticket) = retained
                .begin_estimate(&request.id, &declaration, "session")
                .unwrap();
            retained
                .save_estimate(
                    &lease,
                    &ticket,
                    &declaration,
                    "session",
                    revised_quote(&declaration, scenario),
                )
                .unwrap();
            drop(lease);
            if scenario == "expired" {
                tokio::time::pause();
                tokio::time::advance(std::time::Duration::from_secs(61)).await;
                tokio::time::resume();
            }
            if scenario == "invalidated" {
                ticket.invalidate();
            }
            assert!(
                retained.take(&request, &declaration, "session").is_err(),
                "{scenario}"
            );
            assert!(
                retained.get(&request.id).is_ok(),
                "{scenario}: may explicitly estimate again"
            );
            retained.clear();
            assets.database.shutdown().await.unwrap();
        }
    }
    #[tokio::test]
    async fn priced_consumption_is_single_use_and_persists_selected_evidence_atomically() {
        let (_dir, assets, request, declaration, retained) = paid_fixture().await;
        let (lease, ticket) = retained
            .begin_estimate(&request.id, &declaration, "session")
            .unwrap();
        retained
            .save_estimate(
                &lease,
                &ticket,
                &declaration,
                "session",
                quote(&declaration),
            )
            .unwrap();
        assert!(
            retained.take(&request, &declaration, "session").is_err(),
            "reader still owns paths"
        );
        let arguments = lease.arguments().clone();
        drop(lease);
        let (call, quote) = retained.take(&request, &declaration, "session").unwrap();
        let quote = quote.unwrap();
        assert!(retained.take(&request, &declaration, "session").is_err());
        assert!(retained.get(&request.id).is_err());
        let store = crate::library::attempts::AttemptStore::new(assets.database.clone());
        let saved = call.persist(&store, Some(&quote)).await.unwrap();
        assert_eq!(saved.arguments, arguments);
        assert_eq!(
            saved.observations["estimate"]["binding"],
            quote.observation()["binding"]
        );
        assert_eq!(saved.observations["estimate"]["cap_usd"], 0.5);
        assert_eq!(
            saved.observations["estimate"]["effective"]["model"],
            "offline-script"
        );
        assert!(
            saved.observations["estimate"]["effective"]
                .get("x-fixture")
                .is_none()
        );
        assert_eq!(
            store.get(&request.id).await.unwrap().observations,
            saved.observations
        );
        super::super::super::super::materialized::discard(assets.root(), &saved).unwrap();
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn changed_or_replaced_priced_copies_never_create_an_attempt() {
        for scenario in ["bytes", "missing", "symlink", "hardlink"] {
            let (dir, assets, request, declaration, retained) = paid_fixture().await;
            let (lease, ticket) = retained
                .begin_estimate(&request.id, &declaration, "session")
                .unwrap();
            retained
                .save_estimate(
                    &lease,
                    &ticket,
                    &declaration,
                    "session",
                    quote(&declaration),
                )
                .unwrap();
            let path =
                std::path::PathBuf::from(lease.arguments()["source"]["path"].as_str().unwrap());
            drop(lease);
            let mut bytes = std::fs::read(&path).unwrap();
            let outside = dir.path().join("outside.png");
            std::fs::write(&outside, &bytes).unwrap();
            match scenario {
                "bytes" => {
                    bytes[0] ^= 0xff;
                    std::fs::write(&path, &bytes).unwrap();
                }
                "missing" => std::fs::remove_file(&path).unwrap(),
                "symlink" => {
                    std::fs::remove_file(&path).unwrap();
                    std::os::unix::fs::symlink(&outside, &path).unwrap();
                }
                "hardlink" => {
                    std::fs::remove_file(&path).unwrap();
                    std::fs::hard_link(&outside, &path).unwrap();
                }
                _ => unreachable!(),
            }
            let (call, quote) = retained.take(&request, &declaration, "session").unwrap();
            let store = crate::library::attempts::AttemptStore::new(assets.database.clone());
            assert!(
                call.persist(&store, quote.as_ref()).await.is_err(),
                "{scenario}"
            );
            assert!(
                store
                    .history(None, 50, None, None)
                    .await
                    .unwrap()
                    .is_empty()
            );
            assert!(
                retained.get(&request.id).is_err(),
                "a failed consumed draft needs fresh preparation"
            );
            assert!(outside.exists(), "unowned inputs remain untouched");
            assets.database.shutdown().await.unwrap();
        }
    }
    #[tokio::test]
    async fn paid_verification_handles_empty_optional_slots_and_checks_later_list_copies() {
        for damaged in [false, true] {
            let (_dir, assets, mut request, mut declaration) =
                super::super::super::tests::fixture().await;
            request.params["max_spend_usd"] = json!(0.5);
            declaration.document["tool"] = json!({"name":"fixture","version":"0.1.0"});
            declaration.document["actions"][0]["cost"] = json!({"may_spend":true});
            declaration.tool_session = fixtures::session("fixture");
            declaration.document["actions"][0]["images"]
                .as_array_mut()
                .unwrap()
                .extend([
                    json!({"slot":"references","role":"reference","many":true,"required":false}),
                    json!({"slot":"optional","role":"mask","required":false}),
                ]);
            request.images.insert(
                "references".into(),
                vec![request.images["source"][0].clone(); 2],
            );
            request.images.insert("optional".into(), vec![]);
            let call = PreparedCall::new(&assets, request, &declaration, "session")
                .await
                .unwrap();
            assert!(call.arguments().get("optional").is_none());
            if damaged {
                let path = call.arguments()["references"][1]["path"].as_str().unwrap();
                let mut bytes = std::fs::read(path).unwrap();
                bytes[0] ^= 0xff;
                std::fs::write(path, bytes).unwrap();
            }
            let store = crate::library::attempts::AttemptStore::new(assets.database.clone());
            let result = call.persist(&store, Some(&quote(&declaration))).await;
            assert_eq!(result.is_ok(), !damaged, "{result:?}");
            if let Ok(saved) = result {
                assert_eq!(saved.inputs["references"].as_array().unwrap().len(), 2);
                super::super::super::super::materialized::discard(assets.root(), &saved).unwrap();
            } else {
                assert!(
                    store
                        .history(None, 50, None, None)
                        .await
                        .unwrap()
                        .is_empty()
                );
            }
            assets.database.shutdown().await.unwrap();
        }
    }
}
