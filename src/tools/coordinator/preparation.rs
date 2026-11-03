use super::{Coordinator, Error, Preparation, Result, Submission, SubmissionChange};
use std::sync::{Arc, atomic::Ordering};

impl Coordinator {
    pub async fn prepare(self: &Arc<Self>, request: Submission) -> Result<Preparation> {
        let coordinator = self.clone();
        tokio::spawn(async move { coordinator.prepare_owned(request).await })
            .await
            .map_err(|error| Error::Invalid(format!("Preparation task failed: {error}")))?
    }
    async fn prepare_owned(&self, request: Submission) -> Result<Preparation> {
        super::super::prepared::validate_id(&request.id)?;
        let _submission = self.submission.lock().await;
        let _change = SubmissionChange(self.events.clone());
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        if self.attempts.find(&request).await?.is_some() {
            return Err(Error::Conflict(
                "This ID already belongs to a saved attempt",
            ));
        }
        let entry = self
            .tools
            .get(&request.tool)
            .ok_or(Error::Missing("Tool not registered"))?;
        let _discovery = entry.discovery.lock().await;
        let client = entry
            .client()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        let current = self.refresh_under_guard(&request.tool).await?;
        self.validate_bundle_source(&request, &current.document)
            .await?;
        if let Some(existing) = self
            .preparations
            .existing(&request, &current, &client.session)?
        {
            return Ok(existing);
        }
        self.validate_lineage(&request).await?;
        // Reserve under submission before creating any copies; duplicated bindings each count.
        if request.bound_images().len() > 32 {
            return Err(Error::Invalid(
                "A preparation supports at most 32 images".into(),
            ));
        }
        let mut bytes = 0;
        for (_, _, id) in request.bound_images() {
            bytes += u64::from(self.assets.metadata(id).await?.size);
        }
        self.preparations.capacity(bytes)?;
        let prepared = super::super::prepared::PreparedCall::new(
            &self.assets,
            request,
            &current,
            &client.session,
        )
        .await?;
        self.preparations.insert(prepared)
    }
    pub async fn preparation(&self, id: &str) -> Result<Preparation> {
        let _submission = self.submission.lock().await;
        self.preparations.get(id)
    }
    pub async fn discard_preparation(&self, id: &str) -> Result<()> {
        let _submission = self.submission.lock().await;
        self.preparations.discard(id);
        Ok(())
    }
    pub async fn submit_prepared(
        self: &Arc<Self>,
        request: Submission,
    ) -> Result<crate::library::attempts::Attempt> {
        let coordinator = self.clone();
        tokio::spawn(async move { coordinator.submit_owned(request, true).await })
            .await
            .map_err(|error| Error::Invalid(format!("Prepared submission task failed: {error}")))?
    }
}
