//! Owned serial execution worker; stdio integration follows its direct lifecycle tests.
use super::{
    calls::{Calls, Cancellation, Pending, Stage},
    v04_store::Store,
};
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{Arc, Mutex, mpsc},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
type Output = Arc<dyn Fn(Value) -> bool + Send + Sync>;
struct Active {
    id: Value,
    cancellation: Arc<Cancellation>,
    progress: Arc<Mutex<Progress>>,
}
struct Job {
    id: Value,
    pending: Pending,
    progress: Arc<Mutex<Progress>>,
    cancellation: Arc<Cancellation>,
}
pub struct Jobs {
    calls: Arc<Calls>,
    store: Arc<Store>,
    sender: Option<mpsc::Sender<Job>>,
    active: Arc<Mutex<Vec<Active>>>,
    worker: Option<JoinHandle<()>>,
    output: Output,
}
struct Progress {
    token: Option<Value>,
    reference: Option<Value>,
    started: Instant,
    last: f64,
    output: Output,
}
impl Progress {
    fn emit(&mut self, message: &str, cancellation: &Cancellation) {
        if cancellation.cancelled() {
            return;
        }
        let Some(token) = &self.token else {
            return;
        };
        self.last = self
            .started
            .elapsed()
            .as_secs_f64()
            .max(self.last + 0.000001);
        let mut params = json!({"progressToken":token,"progress":self.last,"message":message});
        if let Some(reference) = &self.reference {
            params["_meta"] = json!({"table-manners/run":reference});
        }
        if !(self.output)(
            json!({"jsonrpc":"2.0","method":"notifications/progress","params":params}),
        ) {
            cancellation.cancel();
        }
    }
}
impl Jobs {
    pub fn new(
        root: &Path,
        calls: Arc<Calls>,
        store: Arc<Store>,
        hold: bool,
        output: impl Fn(Value) -> bool + Send + Sync + 'static,
    ) -> Self {
        let (sender, receiver) = mpsc::channel::<Job>();
        let active = Arc::new(Mutex::new(Vec::<Active>::new()));
        let registered = active.clone();
        let executor = calls.clone();
        let execution_store = store.clone();
        let release = root.join("release-operation");
        let output: Output = Arc::new(output);
        let responses = output.clone();
        let worker = thread::spawn(move || {
            while let Ok(job) = receiver.recv() {
                let response = executor.execute(
                    &execution_store,
                    job.pending,
                    &job.cancellation,
                    |info, stage| {
                        if stage == Stage::Allocated {
                            let mut reference = info.reference.clone();
                            reference.as_object_mut().unwrap().remove("dir");
                            job.progress.lock().unwrap().reference = Some(reference);
                        }
                        job.progress.lock().unwrap().emit(
                            match stage {
                                Stage::Allocated => "Run allocated",
                                Stage::Executing => "Executing",
                                Stage::Executed => "Execution finished",
                            },
                            &job.cancellation,
                        );
                        if hold
                            && stage == Stage::Executed
                            && !release.exists()
                            && !job.cancellation.cancelled()
                        {
                            job.progress
                                .lock()
                                .unwrap()
                                .emit("Waiting for test release", &job.cancellation);
                            let mut heartbeat = Instant::now();
                            while !release.exists() && !job.cancellation.cancelled() {
                                thread::sleep(Duration::from_millis(50));
                                if heartbeat.elapsed() >= Duration::from_secs(1) {
                                    job.progress
                                        .lock()
                                        .unwrap()
                                        .emit("Waiting for test release", &job.cancellation);
                                    let queued: Vec<_> = registered
                                        .lock()
                                        .unwrap()
                                        .iter()
                                        .map(|entry| {
                                            (entry.progress.clone(), entry.cancellation.clone())
                                        })
                                        .collect();
                                    for (progress, cancellation) in queued {
                                        let mut progress = progress.lock().unwrap();
                                        if progress.reference.is_none() {
                                            progress.emit("Waiting for execution", &cancellation);
                                        }
                                    }
                                    heartbeat = Instant::now();
                                }
                            }
                        }
                    },
                );
                job.progress
                    .lock()
                    .unwrap()
                    .emit("Dispatch finished", &job.cancellation);
                // A response can be observed before its output callback returns.
                registered
                    .lock()
                    .unwrap()
                    .retain(|entry| entry.id != job.id);
                if !job.cancellation.cancelled() {
                    let message = match response {
                        Ok(Some(value)) => {
                            Some(json!({"jsonrpc":"2.0","id":job.id,"result":value}))
                        }
                        Err(error) => Some(json!({"jsonrpc":"2.0","id":job.id,"error":error})),
                        Ok(None) => None,
                    };
                    if let Some(message) = message {
                        responses(message);
                    }
                }
            }
        });
        Self {
            calls,
            store,
            sender: Some(sender),
            active,
            worker: Some(worker),
            output,
        }
    }
    pub fn submit(&self, id: Value, params: Value) -> Result<(), Value> {
        let invalid = |message: &str| json!({"code":-32602,"message":message});
        let mut active = self.active.lock().unwrap();
        if active.iter().any(|entry| entry.id == id) {
            return Err(invalid("Request ID is already active"));
        }
        if active.len() >= 32 {
            return Err(json!({"code":-32000,"message":"Test peer execution queue is full"}));
        }
        let sender = self
            .sender
            .as_ref()
            .ok_or_else(|| invalid("Test worker is closed"))?;
        let token = params["_meta"].get("progressToken").cloned();
        if token
            .as_ref()
            .is_some_and(|token| !token.is_string() && !token.is_i64() && !token.is_u64())
        {
            return Err(invalid("progressToken must be a string or integer"));
        }
        let pending = self.calls.accept(&self.store, &params)?;
        let cancellation = Arc::new(Cancellation::default());
        let progress = Arc::new(Mutex::new(Progress {
            token,
            reference: None,
            started: Instant::now(),
            last: -1.0,
            output: self.output.clone(),
        }));
        active.push(Active {
            id: id.clone(),
            cancellation: cancellation.clone(),
            progress: progress.clone(),
        });
        // The owned worker receives until shutdown; a lost receiver means an unrecoverable panic.
        sender
            .send(Job {
                id,
                pending,
                progress,
                cancellation,
            })
            .unwrap_or_else(|_| panic!("Test worker exited before shutdown"));
        Ok(())
    }
    pub fn cancel(&self, id: &Value) {
        if let Some(entry) = self
            .active
            .lock()
            .unwrap()
            .iter()
            .find(|entry| entry.id == *id)
        {
            entry.cancellation.cancel();
        }
    }
    pub fn shutdown(&mut self) {
        for entry in self.active.lock().unwrap().iter() {
            entry.cancellation.cancel();
        }
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            worker.join().expect("Test execution worker panicked");
        }
    }
}
impl Drop for Jobs {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::super::{calls::Calls, v04_store::Store};
    use super::*;
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde_json::{Value, json};
    use std::{
        sync::{Arc, mpsc},
        time::Duration,
    };
    fn setup(root: &std::path::Path, hold: bool) -> (Jobs, Arc<Calls>, mpsc::Receiver<Value>) {
        let calls = Arc::new(Calls::new(
            root,
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        ));
        let store = Arc::new(Store::new(root, "sample-lab", vec![]).unwrap());
        let (tx, rx) = mpsc::channel();
        (
            Jobs::new(root, calls.clone(), store, hold, move |value| {
                tx.send(value).is_ok()
            }),
            calls,
            rx,
        )
    }
    fn request(id: &str) -> Value {
        json!({"name":"make_token","arguments":{"source":{"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}}},"_meta":{"table-manners/client_run_id":id,"progressToken":id}})
    }
    fn receive(rx: &mpsc::Receiver<Value>) -> Value {
        rx.recv_timeout(Duration::from_secs(5))
            .expect("observable worker event")
    }
    fn until(rx: &mpsc::Receiver<Value>, mut predicate: impl FnMut(&Value) -> bool) -> Vec<Value> {
        let mut messages = vec![];
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let message = rx
                .recv_timeout(deadline.saturating_duration_since(Instant::now()))
                .expect("expected event before deadline");
            let done = predicate(&message);
            messages.push(message);
            if done {
                return messages;
            }
        }
    }
    fn resource(calls: &Calls, uri: &str) -> Value {
        let response = calls.resource(uri).unwrap().unwrap();
        serde_json::from_str(response["text"].as_str().unwrap()).unwrap()
    }
    #[test]
    fn held_work_reports_structured_heartbeats_while_queued_cancellation_keeps_its_claim() {
        let root = tempfile::tempdir().unwrap();
        let (mut jobs, calls, rx) = setup(root.path(), true);
        jobs.submit(json!(1), request("first")).unwrap();
        let mut events = until(&rx, |m| {
            m["params"]["message"] == "Waiting for test release"
        });
        assert_eq!(
            events
                .iter()
                .map(|m| m["params"]["message"].as_str().unwrap())
                .collect::<Vec<_>>(),
            [
                "Run allocated",
                "Executing",
                "Execution finished",
                "Waiting for test release"
            ]
        );
        let reference = events[0]["params"]["_meta"]["table-manners/run"].clone();
        assert!(reference.get("dir").is_none());
        let running = resource(&calls, "sample-lab://runs");
        assert_eq!(running["runs"][0]["status"], "running");
        assert!(
            calls
                .resource(reference["resource"].as_str().unwrap())
                .unwrap()
                .is_err()
        );
        let duplicate_request = jobs.submit(json!(1), request("queued")).unwrap_err();
        assert_eq!(duplicate_request["code"], -32602);
        assert!(duplicate_request.get("data").is_none());
        jobs.submit(json!(2), request("queued")).unwrap();
        let duplicate = jobs.submit(json!(3), request("queued")).unwrap_err();
        assert_eq!(duplicate["data"]["reason"], "client_run_id_reused");
        assert_eq!(duplicate["data"]["run"], Value::Null);
        events.extend(until(&rx, |m| m["params"]["progressToken"] == "queued"));
        jobs.cancel(&json!(2));
        std::fs::write(root.path().join("release-operation"), "release").unwrap();
        // A later completed request proves the cancelled queue entry was consumed before shutdown.
        jobs.submit(json!(4), json!({"name":"summarize","arguments":{},"_meta":{"table-manners/client_run_id":"drain"}})).unwrap();
        events.extend(until(&rx, |m| m["id"] == 4));
        jobs.shutdown();
        events.extend(rx.try_iter());
        assert!(!events.iter().any(|m| m["id"] == 2));
        let mut progress = std::collections::HashMap::<String, f64>::new();
        for message in events
            .iter()
            .filter(|m| m["method"] == "notifications/progress")
        {
            let token = message["params"]["progressToken"].as_str().unwrap();
            if token == "queued" {
                assert!(message["params"].get("_meta").is_none());
                assert_eq!(message["params"]["message"], "Waiting for execution");
            } else {
                assert_eq!(token, "first");
                assert_eq!(message["params"]["_meta"]["table-manners/run"], reference);
            }
            let previous = progress.entry(token.into()).or_insert(-1.0);
            let current = message["params"]["progress"].as_f64().unwrap();
            assert!(current > *previous);
            *previous = current;
            assert!(
                !message["params"]["message"]
                    .as_str()
                    .unwrap()
                    .contains("://")
            );
        }
        let listed = resource(&calls, "sample-lab://runs");
        assert_eq!(listed["runs"].as_array().unwrap().len(), 2);
        assert_eq!(listed["next_cursor"], Value::Null);
        assert!(
            listed["runs"]
                .as_array()
                .unwrap()
                .iter()
                .all(|run| run["client_run_id"] == "first" || run["client_run_id"] == "drain")
        );
        let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
        let refused = calls.accept(&store, &request("queued")).err().unwrap();
        assert_eq!(refused["data"]["reason"], "client_run_id_reused");
        assert_eq!(refused["data"]["run"], Value::Null);
        let run = resource(&calls, reference["resource"].as_str().unwrap());
        assert_eq!(run["status"], "completed");
        assert_eq!(run["artifacts"]["out"].as_array().unwrap().len(), 1);
    }
    #[test]
    fn shutdown_cancels_held_work_and_joins_before_saved_evidence_is_observed() {
        let root = tempfile::tempdir().unwrap();
        let (mut jobs, calls, rx) = setup(root.path(), true);
        jobs.submit(json!(1), request("active")).unwrap();
        let mut events = until(&rx, |m| {
            m["params"]["message"] == "Waiting for test release"
        });
        let reference = events[0]["params"]["_meta"]["table-manners/run"].clone();
        jobs.submit(json!(2), request("queued")).unwrap();
        jobs.shutdown();
        events.extend(rx.try_iter());
        assert!(events.iter().all(|m| m.get("id").is_none()));
        let run = resource(&calls, reference["resource"].as_str().unwrap());
        assert_eq!(run["status"], "cancelled");
        assert_eq!(run["artifacts"]["out"].as_array().unwrap().len(), 1);
        assert_eq!(
            resource(&calls, "sample-lab://runs")["runs"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(!root.path().join("release-operation").exists());
    }
    #[test]
    fn no_token_means_no_notifications_and_admission_refusals_do_not_claim_ids() {
        let root = tempfile::tempdir().unwrap();
        let (mut jobs, _, rx) = setup(root.path(), false);
        let mut params = request("retry");
        params["_meta"]["progressToken"] = json!({"invalid":true});
        assert_eq!(
            jobs.submit(json!(1), params.clone()).unwrap_err()["code"],
            -32602
        );
        params["_meta"]
            .as_object_mut()
            .unwrap()
            .remove("progressToken");
        jobs.submit(json!(1), params).unwrap();
        let response = receive(&rx);
        assert_eq!(response["id"], 1);
        assert_eq!(
            response["result"]["structuredContent"]["status"],
            "completed"
        );
        jobs.shutdown();
        assert!(rx.try_iter().next().is_none());
    }
    #[test]
    fn response_delivery_releases_the_request_id_before_the_client_can_reuse_it() {
        let root = tempfile::tempdir().unwrap();
        let calls = Arc::new(Calls::new(
            root.path(),
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        ));
        let store = Arc::new(Store::new(root.path(), "sample-lab", vec![]).unwrap());
        let (tx, rx) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let gate = Mutex::new(gate);
        let mut jobs = Jobs::new(root.path(), calls, store, false, move |value| {
            let first = value["result"]["structuredContent"]["run"]["client_run_id"] == "first";
            let sent = tx.send(value).is_ok();
            if first {
                gate.lock()
                    .unwrap()
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap();
            }
            sent
        });
        let params = |client| json!({"name":"summarize","arguments":{},"_meta":{"table-manners/client_run_id":client}});
        jobs.submit(json!(1), params("first")).unwrap();
        assert_eq!(receive(&rx)["id"], 1);
        // The output callback is still blocked, exactly as a slow stdout write might be.
        let reused = jobs.submit(json!(1), params("second"));
        release.send(()).unwrap();
        reused.expect("a delivered response has released its request ID");
        assert_eq!(
            receive(&rx)["result"]["structuredContent"]["run"]["client_run_id"],
            "second"
        );
        jobs.shutdown();
    }
    #[test]
    fn admission_is_bounded_and_overflow_does_not_claim_a_correlation_id() {
        let root = tempfile::tempdir().unwrap();
        let (mut jobs, calls, rx) = setup(root.path(), true);
        jobs.submit(json!(1), request("active")).unwrap();
        until(&rx, |m| {
            m["params"]["message"] == "Waiting for test release"
        });
        for id in 2..=32 {
            jobs.submit(json!(id), request(&format!("queued-{id}")))
                .unwrap();
        }
        let refused = jobs.submit(json!(33), request("overflow")).unwrap_err();
        assert_eq!(refused["code"], -32000);
        let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
        assert!(calls.accept(&store, &request("overflow")).is_ok());
        jobs.shutdown();
        assert_eq!(
            resource(&calls, "sample-lab://runs")["runs"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn failed_progress_delivery_cancels_before_work_without_waiting_for_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let calls = Arc::new(Calls::new(
            root.path(),
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        ));
        let store = Arc::new(Store::new(root.path(), "sample-lab", vec![]).unwrap());
        let (tx, rx) = mpsc::channel();
        let mut jobs = Jobs::new(root.path(), calls.clone(), store, true, move |value| {
            let delivered = value["method"] != "notifications/progress";
            tx.send(value).is_ok() && delivered
        });
        jobs.submit(json!(1), request("closed-output")).unwrap();
        jobs.submit(json!(2), json!({"name":"summarize","arguments":{},"_meta":{"table-manners/client_run_id":"drain"}})).unwrap();
        // Release only the later no-token call; the first must stop because delivery failed.
        let first = receive(&rx);
        assert_eq!(first["params"]["message"], "Run allocated");
        std::fs::write(root.path().join("release-operation"), "release").unwrap();
        let rest = until(&rx, |message| message["id"] == 2);
        assert!(
            rest.iter()
                .all(|message| message.get("method").is_none() && message["id"] != 1)
        );
        let run = resource(
            &calls,
            first["params"]["_meta"]["table-manners/run"]["resource"]
                .as_str()
                .unwrap(),
        );
        assert_eq!(run["status"], "cancelled");
        assert_eq!(run["artifacts"]["out"], json!([]));
        jobs.shutdown();
    }
}
