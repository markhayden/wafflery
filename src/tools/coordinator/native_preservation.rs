use super::{Coordinator, Error, Result};
use crate::library::{
    attempts::Attempt,
    blob_links::{BlobLink, Owner},
    blobs::BlobStore,
};
use serde_json::{Value, json};
use std::sync::Arc;
fn entries(attempt: &Attempt) -> Vec<Value> {
    let mut values = vec![];
    let Some(result) = &attempt.result else {
        return values;
    };
    for direction in ["in", "out"] {
        for (index, item) in result["artifacts"][direction]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            if item["artifact"]["type"] == "image" {
                continue;
            }
            // Retry unfinished items without discarding a previously verified owned copy.
            if let Some(saved) = attempt.observations["native_artifacts"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|saved| {
                    saved["direction"] == direction
                        && saved["index"] == index
                        && saved["type"] == item["artifact"]["type"]
                        && saved["state"] == "preserved"
                        && saved["sha256"].is_string()
                })
            {
                values.push(saved.clone());
                continue;
            }
            let kind = attempt.declaration["document"]["artifact_types"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|kind| kind["type"] == item["artifact"]["type"]);
            let portable = kind
                .and_then(|kind| kind["portable"].as_str())
                .is_some_and(|portable| matches!(portable, "json" | "bundle"));
            let (state, reason) = if portable {
                ("preserving", "Waiting for native preservation")
            } else {
                (
                    "workspace-dependent",
                    "The tool declares no portable form for this type.",
                )
            };
            values.push(json!({
                "direction":direction, "index":index, "port":item["port"],
                "type":item["artifact"]["type"], "state":state, "reason":reason
            }));
        }
    }
    values
}
fn summary(values: &[Value]) -> &'static str {
    if values.is_empty() {
        "not-needed"
    } else if values.iter().any(|item| item["state"] == "preserving") {
        "preserving"
    } else if values.iter().any(|item| item["state"] == "failed") {
        "failed"
    } else if values
        .iter()
        .any(|item| item["state"] == "workspace-dependent")
    {
        "workspace-dependent"
    } else {
        "preserved"
    }
}
fn fail_pending(attempt: &mut Attempt, reason: &str) {
    if let Some(values) = attempt.observations["native_artifacts"].as_array_mut() {
        for value in values
            .iter_mut()
            .filter(|value| value["state"] == "preserving")
        {
            value["state"] = json!("failed");
            value["reason"] = json!(reason);
        }
        attempt.preservation["native"] = json!(summary(values));
    }
}
impl Coordinator {
    pub async fn queue_native_preservation(self: &Arc<Self>, id: &str) -> Result<Attempt> {
        let coordinator = self.clone();
        let id = id.to_owned();
        tokio::spawn(async move { coordinator.queue_native_owned(&id).await })
            .await
            .map_err(|error| Error::Invalid(format!("Native queue worker stopped: {error}")))?
    }
    async fn queue_native_owned(self: &Arc<Self>, id: &str) -> Result<Attempt> {
        let _submission = self.submission.lock().await;
        if self.stopping.load(std::sync::atomic::Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        let saved = self.attempts.get(id).await?;
        if saved.result.is_none() {
            return Err(Error::Invalid(
                "Wait for a terminal result before preserving native artifacts".into(),
            ));
        }
        // Repeated clicks must not reset status while the same worker owns this attempt.
        if self.native_queue.contains(id) {
            return self.attempts.get(id).await.map_err(Into::into);
        }
        let values = entries(&saved);
        let pending = values.iter().any(|item| item["state"] == "preserving");
        let native = summary(&values);
        let updated = self
            .attempts
            .update(id, move |attempt| {
                attempt.observations["native_artifacts"] = json!(values);
                attempt.preservation["native"] = json!(native);
                Ok(())
            })
            .await?;
        if pending && !self.native_queue.enqueue(id, &saved.request.tool) {
            let failed = self.attempts.update(id, |attempt| {
                fail_pending(attempt, "Native preservation queue is full. Retry explicitly after current transfers finish.");
                Ok(())
            }).await?;
            self.events.changed();
            return Ok(failed);
        }
        let workers = (0..self.native_queue.workers_needed())
            .map(|_| {
                let coordinator = self.clone();
                tokio::spawn(async move {
                    coordinator.native_worker().await;
                })
            })
            .collect::<Vec<_>>();
        let mut retained = self.workers.lock().await;
        retained.retain(|worker| !worker.is_finished());
        retained.extend(workers);
        drop(retained);
        self.events.changed();
        Ok(updated)
    }
    async fn native_worker(self: Arc<Self>) {
        while let Some(id) = self.native_queue.next() {
            let result = self.preserve_attempt(&id).await;
            if let Err(error) = result {
                let reason = error.to_string();
                let _ = self
                    .attempts
                    .update(&id, move |attempt| {
                        fail_pending(attempt, &reason);
                        Ok(())
                    })
                    .await;
            }
            self.native_queue.finish(&id);
            self.events.changed();
        }
    }
    async fn preserve_attempt(&self, id: &str) -> Result<()> {
        let saved = self.attempts.get(id).await?;
        let values = saved.observations["native_artifacts"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        for entry in values
            .into_iter()
            .filter(|entry| entry["state"] == "preserving")
        {
            let direction = entry["direction"]
                .as_str()
                .filter(|direction| matches!(*direction, "in" | "out"))
                .ok_or_else(|| Error::Invalid("Invalid saved native direction".into()))?;
            let index = entry["index"]
                .as_u64()
                .and_then(|index| usize::try_from(index).ok())
                .ok_or_else(|| Error::Invalid("Invalid saved native occurrence".into()))?;
            let reference = saved
                .result
                .as_ref()
                .and_then(|result| result["artifacts"][direction].get(index))
                .map(|item| item["artifact"].clone())
                .ok_or(Error::Missing("Saved native occurrence is unavailable"))?;
            let key = serde_json::to_string(&reference)?;
            let lock = self.native_queue.reference(key);
            let _reference = lock.lock().await;
            let blob = self
                .preserve_occurrence(id, direction, index, &reference)
                .await;
            let direction = direction.to_owned();
            self.attempts
                .update(id, move |attempt| {
                    if let Some(values) = attempt.observations["native_artifacts"].as_array_mut() {
                        if let Some(value) = values.iter_mut().find(|value| {
                            value["direction"] == direction && value["index"] == index
                        }) {
                            match blob {
                                Ok(link) => {
                                    value["state"] = json!("preserved");
                                    value["reason"] =
                                        json!("Verified native bytes are owned by this library.");
                                    value["sha256"] = json!(link.sha256);
                                }
                                Err(error) => {
                                    value["state"] = json!("failed");
                                    value["reason"] = json!(error.to_string());
                                }
                            }
                        }
                        attempt.preservation["native"] = json!(summary(values));
                    }
                    Ok(())
                })
                .await?;
            self.events.changed();
        }
        Ok(())
    }
    async fn preserve_occurrence(
        &self,
        id: &str,
        direction: &str,
        index: usize,
        reference: &Value,
    ) -> Result<BlobLink> {
        let blobs = BlobStore(self.assets.clone());
        if let Some(link) = blobs.find_reference(reference).await? {
            blobs.read(&link.sha256).await?;
            let link = BlobLink {
                direction: direction.into(),
                occurrence: u32::try_from(index).map_err(|_| {
                    Error::Invalid("Native occurrence exceeds the host limit".into())
                })?,
                ..link
            };
            blobs
                .attach(Owner::Attempt(id.into()), link.clone())
                .await?;
            return Ok(link);
        }
        let saved = self.attempts.get(id).await?;
        if reference["tool"] != saved.declaration["document"]["tool"]["name"] {
            return Err(Error::Missing(
                "No owned native bytes are available for this foreign input. Preserve its originating saved run in the owning tool first.",
            ));
        }
        self.preserve_artifact(id, direction, index).await
    }
}
