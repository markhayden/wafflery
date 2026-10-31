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
    pub id: String,
    pub store: AttemptStore,
    pub action: String,
    pub arguments: Value,
    pub generation: u64,
}
pub(crate) type Gate = Arc<Mutex<Option<Permit>>>;
pub(crate) struct RecordedTransport<T> {
    pub inner: T,
    pub gate: Gate,
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
                if value["method"] == "tools/call" && value["params"]["name"] != "describe" =>
            {
                Some(self.gate.lock().unwrap_or_else(|e| e.into_inner()).take())
            }
            _ => None,
        };
        let generation = self.generation.clone();
        let write = self.inner.send(item);
        async move {
            let wire = wire?;
            if let Some(permit) = permit {
                let permit = permit
                    .ok_or_else(|| io::Error::other("No persisted dispatch authorization"))?;
                if wire["params"]["name"] != permit.action
                    || wire["params"]["arguments"] != permit.arguments
                    || generation.load(Ordering::SeqCst) != permit.generation
                {
                    return Err(io::Error::other(
                        "Dispatch authorization is stale or does not match the request",
                    ));
                }
                permit
                    .store
                    .record_wire(&permit.id, wire)
                    .await
                    .map_err(io::Error::other)?;
                if generation.load(Ordering::SeqCst) != permit.generation {
                    return Err(io::Error::other("Declaration changed before dispatch"));
                }
            }
            write.await
        }
    }
    async fn receive(&mut self) -> Option<RxJsonRpcMessage<RoleClient>> {
        self.inner.receive().await
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
    #[derive(Default)]
    struct Wire(Arc<Mutex<Vec<Value>>>);
    impl Transport<RoleClient> for Wire {
        type Error = io::Error;
        fn send(
            &mut self,
            item: TxJsonRpcMessage<RoleClient>,
        ) -> impl Future<Output = io::Result<()>> + Send + 'static {
            let values = self.0.clone();
            async move {
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
            gate: gate.clone(),
            generation: Arc::new(AtomicU64::new(0)),
        };
        let message = json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"op","arguments":{},"_meta":{"progressToken":9}}});
        assert!(
            transport
                .send(serde_json::from_value(message.clone()).unwrap())
                .await
                .is_err()
        );
        *gate.lock().unwrap() = Some(Permit {
            id: "id".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
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
        *gate.lock().unwrap() = Some(Permit {
            id: "id".into(),
            store: store.clone(),
            action: "op".into(),
            arguments: json!({}),
            generation: 0,
        });
        transport
            .send(serde_json::from_value(message.clone()).unwrap())
            .await
            .unwrap();
        assert_eq!(store.get("id").await.unwrap().wire, Some(message.clone()));
        assert_eq!(*sent.lock().unwrap(), vec![message]);
        assets.database.shutdown().await.unwrap();
    }
}
