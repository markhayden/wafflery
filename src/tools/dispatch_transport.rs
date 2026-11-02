//! The SDK assigns IDs when enqueuing requests. Persist at the write boundary, before
//! polling the underlying transport, so there is no sent-but-unrecorded request window.
use crate::library::attempts::AttemptStore;
use rmcp::{
    RoleClient,
    service::{RxJsonRpcMessage, TxJsonRpcMessage},
    transport::Transport,
};
use serde_json::Value;
use std::{
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

pub(crate) struct Permit {
    pub discovery: tokio::sync::OwnedMutexGuard<()>,
    pub id: String,
    pub store: AttemptStore,
    pub action: String,
    pub arguments: Value,
    pub generation: u64,
    pub written: tokio::sync::watch::Sender<bool>,
}
pub(crate) type Gate = Arc<Mutex<Option<Permit>>>;

/// Completion may race a replacement authorized after the old child was reaped.
pub(super) fn revoke(gate: &Gate, id: &str) {
    let mut gate = gate.lock().unwrap_or_else(|e| e.into_inner());
    if gate.as_ref().is_some_and(|permit| permit.id == id) {
        gate.take();
    }
}

pub(crate) struct RecordedTransport<T> {
    pub inner: T,
    pub gate: Gate,
    pub association: Arc<Mutex<super::association::ProgressAssociation>>,
    pub generation: Arc<AtomicU64>,
}
impl<T: Transport<RoleClient, Error = io::Error>> Transport<RoleClient> for RecordedTransport<T> {
    type Error = io::Error;
    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl Future<Output = io::Result<()>> + Send + 'static {
        let wire = serde_json::to_value(&item).map_err(io::Error::other);
        let permit = match &wire {
            Ok(value)
                if value["method"] == "tools/call"
                    && !matches!(
                        value["params"]["name"].as_str(),
                        Some("describe" | "estimate")
                    ) =>
            {
                Some(self.gate.lock().unwrap_or_else(|e| e.into_inner()).take())
            }
            _ => None,
        };
        let generation = self.generation.clone();
        let association = self.association.clone();
        let write = self.inner.send(item);
        async move {
            let wire = wire?;
            let mut written = None;
            let mut discovery = None;
            if let Some(permit) = permit {
                let permit = permit
                    .ok_or_else(|| io::Error::other("No persisted dispatch authorization"))?;
                if wire["params"]["name"] != permit.action
                    || wire["params"]["arguments"] != permit.arguments
                    || wire["params"]["_meta"]["table-manners/client_run_id"] != permit.id
                    || generation.load(Ordering::SeqCst) != permit.generation
                {
                    return Err(io::Error::other(
                        "Dispatch authorization is stale or does not match the request",
                    ));
                }
                discovery = Some(permit.discovery);
                // rmcp replaces progressToken when enqueuing; bind the actual wire token.
                association.lock().unwrap_or_else(|e| e.into_inner()).token =
                    wire["params"]["_meta"].get("progressToken").cloned();
                permit
                    .store
                    .record_wire(&permit.id, wire)
                    .await
                    .map_err(io::Error::other)?;
                if generation.load(Ordering::SeqCst) != permit.generation {
                    return Err(io::Error::other("Declaration changed before dispatch"));
                }
                written = Some(permit.written);
            }
            let outcome = write.await;
            drop(discovery);
            outcome?;
            if let Some(written) = written {
                written.send_replace(true);
            }
            Ok(())
        }
    }
    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        let message = self.inner.receive().await;
        if let Some(rmcp::model::JsonRpcMessage::Notification(notification)) = &message
            && let rmcp::model::ServerNotification::ProgressNotification(progress) =
                &notification.notification
            && let Ok(value) = serde_json::to_value(progress)
        {
            // rmcp moves _meta into NotificationContext and schedules handlers separately.
            // Associate in receive order before a later response can resolve the call.
            self.association
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .observe(&value["params"]);
        }
        message
    }
    async fn close(&mut self) -> io::Result<()> {
        self.inner.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::{assets::AssetStore, attempts::Submission};
    use serde_json::json;
    type WritePause = (Arc<tokio::sync::Notify>, Arc<tokio::sync::Notify>);
    #[derive(Default)]
    struct Wire(Arc<Mutex<Vec<Value>>>, Option<WritePause>);
    impl Transport<RoleClient> for Wire {
        type Error = io::Error;
        fn send(
            &mut self,
            item: TxJsonRpcMessage<RoleClient>,
        ) -> impl Future<Output = io::Result<()>> + Send + 'static {
            let values = self.0.clone();
            let pause = self.1.clone();
            async move {
                if let Some((entered, release)) = pause {
                    entered.notify_one();
                    release.notified().await;
                }
                values
                    .lock()
                    .unwrap()
                    .push(serde_json::to_value(item).unwrap());
                Ok(())
            }
        }
        async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
            None
        }
        async fn close(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    #[tokio::test]
    async fn progress_association_is_recorded_before_the_sdk_dispatches_the_notification() {
        struct Incoming(Option<RxJsonRpcMessage<RoleClient>>);
        impl Transport<RoleClient> for Incoming {
            type Error = io::Error;
            fn send(
                &mut self,
                _: TxJsonRpcMessage<RoleClient>,
            ) -> impl Future<Output = io::Result<()>> + Send + 'static {
                std::future::ready(Ok(()))
            }
            async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
                self.0.take()
            }
            async fn close(&mut self) -> io::Result<()> {
                Ok(())
            }
        }
        let reference = json!({"id":"run","resource":"sample-lab://run/run/run.json","session_id":"bb8f5520-a822-4cdc-a9eb-f1a72018e000","client_run_id":"request"});
        let wire = serde_json::from_value(json!({"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":7,"progress":1,"_meta":{"table-manners/run":reference}}})).unwrap();
        let association = Arc::new(Mutex::new(super::super::association::ProgressAssociation {
            request: "request".into(),
            token: Some(json!(7)),
            ..Default::default()
        }));
        let mut transport = RecordedTransport {
            inner: Incoming(Some(wire)),
            gate: Default::default(),
            association: association.clone(),
            generation: Arc::new(AtomicU64::new(0)),
        };
        assert!(transport.receive().await.is_some());
        assert_eq!(association.lock().unwrap().reference, Some(reference));
    }
    #[tokio::test]
    async fn database_failure_or_absent_authorization_sends_zero_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
        let store = AttemptStore::new(assets.database.clone());
        let request:Submission=serde_json::from_value(json!({"id":"id","tool":"fixture","action":"op","revision":"r","params":{},"images":{}})).unwrap();
        store
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
        let wire = Wire::default();
        let sent = wire.0.clone();
        let gate = Gate::default();
        let mut transport = RecordedTransport {
            inner: wire,
            association: Default::default(),
            gate: gate.clone(),
            generation: Arc::new(AtomicU64::new(0)),
        };
        let message = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"op","arguments":{},"_meta":{"progressToken":9,"table-manners/client_run_id":"id"}}});
        assert!(
            transport
                .send(serde_json::from_value(message.clone()).unwrap())
                .await
                .is_err()
        );
        let discovery = Arc::new(tokio::sync::Mutex::new(())).lock_owned().await;
        *gate.lock().unwrap() = Some(Permit {
            discovery,
            id: "id".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
            written: tokio::sync::watch::channel(false).0,
        });
        assets.database.shutdown().await.unwrap();
        assert!(
            transport
                .send(serde_json::from_value(message.clone()).unwrap())
                .await
                .is_err()
        );
        assert!(sent.lock().unwrap().is_empty());
        let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
        let store = AttemptStore::new(assets.database.clone());
        let discovery = Arc::new(tokio::sync::Mutex::new(())).lock_owned().await;
        *gate.lock().unwrap() = Some(Permit {
            discovery,
            id: "id".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
            written: tokio::sync::watch::channel(false).0,
        });
        transport
            .send(serde_json::from_value(message.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(store.get("id").await.unwrap().wire, Some(message.clone()));
        assert_eq!(*sent.lock().unwrap(), vec![message]);
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn discovery_guard_covers_the_wire_write_but_not_execution() {
        let dir = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
        let store = AttemptStore::new(assets.database.clone());
        let request: Submission = serde_json::from_value(json!({"id":"id","tool":"fixture","action":"op","revision":"r","params":{},"images":{}})).unwrap();
        store
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
        let discovery = Arc::new(tokio::sync::Mutex::new(()));
        let entered = Arc::new(tokio::sync::Notify::new());
        let release = Arc::new(tokio::sync::Notify::new());
        let gate = Gate::default();
        let guard = discovery.clone().lock_owned().await;
        *gate.lock().unwrap() = Some(Permit {
            discovery: guard,
            id: "id".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
            written: tokio::sync::watch::channel(false).0,
        });
        let mut transport = RecordedTransport {
            inner: Wire(Default::default(), Some((entered.clone(), release.clone()))),
            gate,
            association: Default::default(),
            generation: Arc::new(AtomicU64::new(0)),
        };
        let sending = tokio::spawn(transport.send(serde_json::from_value(json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"op","arguments":{},"_meta":{"table-manners/client_run_id":"id"}}})).unwrap()));
        entered.notified().await;
        let held_during_write = discovery.try_lock().is_err();
        release.notify_one();
        sending.await.unwrap().unwrap();
        let released_after_write = discovery.try_lock().is_ok();
        assert_eq!(store.get("id").await.unwrap().execution, "running");
        assets.database.shutdown().await.unwrap();
        assert!(
            held_during_write,
            "refresh could change authorization before the write completed"
        );
        assert!(
            released_after_write,
            "refresh remained blocked during execution"
        );
    }
    #[tokio::test]
    async fn old_completion_cannot_revoke_the_next_requests_authorization() {
        let dir = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
        let store = AttemptStore::new(assets.database.clone());
        let request: Submission = serde_json::from_value(json!({"id":"next","tool":"fixture","action":"op","revision":"r","params":{},"images":{}})).unwrap();
        store
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
        let gate = Gate::default();
        let discovery = Arc::new(tokio::sync::Mutex::new(())).lock_owned().await;
        *gate.lock().unwrap() = Some(Permit {
            discovery,
            id: "next".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
            written: tokio::sync::watch::channel(false).0,
        });
        // Model the exact interleaving: a replacement is authorized while old cleanup is paused.
        revoke(&gate, "previous");
        let wire = Wire::default();
        let sent = wire.0.clone();
        let mut transport = RecordedTransport {
            inner: wire,
            gate: gate.clone(),
            association: Default::default(),
            generation: Arc::new(AtomicU64::new(0)),
        };
        let message = json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"op","arguments":{},"_meta":{"table-manners/client_run_id":"next"}}});
        let result = transport
            .send(serde_json::from_value(message.clone()).unwrap())
            .await;
        let saved = store.get("next").await.unwrap();
        assets.database.shutdown().await.unwrap();
        assert!(
            result.is_ok(),
            "old cleanup removed the next call's authorization: {result:?}"
        );
        assert_eq!(saved.wire, Some(message.clone()));
        assert_eq!(*sent.lock().unwrap(), vec![message]);

        let discovery = Arc::new(tokio::sync::Mutex::new(())).lock_owned().await;
        *gate.lock().unwrap() = Some(Permit {
            discovery,
            id: "abandoned".into(),
            store,
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
            written: tokio::sync::watch::channel(false).0,
        });
        revoke(&gate, "abandoned");
        assert!(
            gate.lock().unwrap().is_none(),
            "own unused authorization must be revoked"
        );
    }
    #[tokio::test]
    async fn reserved_controls_leave_staged_dispatch_and_association_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let assets = AssetStore::open(&dir.path().join("Library")).unwrap();
        let store = AttemptStore::new(assets.database.clone());
        let request: Submission = serde_json::from_value(json!({"id":"run","tool":"fixture","action":"op","revision":"r","params":{},"images":{}})).unwrap();
        store
            .create(request, json!({}), json!({}), json!({}))
            .await
            .unwrap();
        let gate = Gate::default();
        let (written, observed) = tokio::sync::watch::channel(false);
        *gate.lock().unwrap() = Some(Permit {
            discovery: Arc::new(tokio::sync::Mutex::new(())).lock_owned().await,
            id: "run".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
            written,
        });
        let association = Arc::new(Mutex::new(super::super::association::ProgressAssociation {
            request: "run".into(),
            token: Some(json!(41)),
            ..Default::default()
        }));
        let wire = Wire::default();
        let sent = wire.0.clone();
        let mut transport = RecordedTransport {
            inner: wire,
            gate: gate.clone(),
            association: association.clone(),
            generation: Arc::new(AtomicU64::new(0)),
        };
        for (id, name) in [(1, "estimate"), (2, "describe")] {
            transport.send(serde_json::from_value(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":{},"_meta":{"progressToken":99}}})).unwrap()).await.unwrap();
            assert!(gate.lock().unwrap().is_some());
            assert!(!*observed.borrow());
            assert!(store.get("run").await.unwrap().wire.is_none());
            assert_eq!(association.lock().unwrap().token, Some(json!(41)));
        }
        let action = json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"op","arguments":{},"_meta":{"table-manners/client_run_id":"run","progressToken":42}}});
        transport
            .send(serde_json::from_value(action.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(store.get("run").await.unwrap().wire, Some(action));
        assert!(*observed.borrow());
        assert_eq!(association.lock().unwrap().token, Some(json!(42)));
        assert_eq!(sent.lock().unwrap().len(), 3);
        for name in ["estimate_more", "Describe", "other_action"] {
            assert!(transport.send(serde_json::from_value(json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":name,"arguments":{}}})).unwrap()).await.is_err());
        }
        assert_eq!(sent.lock().unwrap().len(), 3);
        assert_eq!(store.history(None, 50, None, None).await.unwrap().len(), 1);
        assets.database.shutdown().await.unwrap();
    }
    #[tokio::test]
    async fn reserved_estimate_control_reaches_independent_peer_without_a_dispatch_permit() {
        let dir = tempfile::tempdir().unwrap();
        use crate::tools::{client::ToolClient, registration::Descriptor};
        let repository = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            std::process::Command::new(env!("CARGO"))
                .args(["build", "--locked", "--example", "sample-lab"])
                .current_dir(repository)
                .status()
                .unwrap()
                .success()
        );
        let mut client = ToolClient::spawn(Descriptor {
            config: None,
            id: "fixture".into(),
            executable: repository.join("target/debug/examples/sample-lab"),
            args: vec![
                dir.path().join("runs").display().to_string(),
                "v03-ready".into(),
            ],
            cwd: dir.path().to_owned(),
            roots: vec![dir.path().to_owned()],
        })
        .unwrap();
        client.initialize().await.unwrap();
        client.describe().await.unwrap();
        let request = serde_json::from_value(json!({"method":"tools/call","params":{"name":"estimate","arguments":{"action":"price_test","arguments":{"label":"Transport control probe","max_spend_usd":0.5}}}})).unwrap();
        let response = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            client.peer().unwrap().send_request(request),
        )
        .await;
        let runs = client.read_resource("sample-lab://runs").await;
        assert!(client.close().await.unwrap());
        let response = serde_json::to_value(response.unwrap().unwrap()).unwrap();
        assert_eq!(response["structuredContent"]["allowed"], true);
        let runs: serde_json::Value =
            serde_json::from_str(runs.unwrap()["contents"][0]["text"].as_str().unwrap()).unwrap();
        assert!(runs["runs"].as_array().unwrap().is_empty());
        assert!(!dir.path().join("runs/calls.ndjson").exists());
    }
}
