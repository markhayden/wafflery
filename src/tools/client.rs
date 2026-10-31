// Roots are part of our pinned 2025-11-25 protocol; upgrading requires a new file boundary.
#![allow(deprecated)]
use super::dispatch_transport::{Gate, RecordedTransport};
use super::{Error, Result, registration::Descriptor, validate_contract, validator};
use rmcp::{
    ClientHandler, RoleClient,
    model::*,
    service::{NotificationContext, Peer, RequestContext, RunningService, serve_client},
    transport::async_rw::AsyncRwTransport,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    process::{Child, ChildStdin, ChildStdout, Command},
    sync::{Mutex, watch},
};

const CONTROL_TIMEOUT: Duration = Duration::from_secs(5);
#[derive(Clone)]
struct Handler {
    roots: Vec<Root>,
    generation: Arc<AtomicU64>,
    progress: watch::Sender<Option<Value>>,
}
impl ClientHandler for Handler {
    fn get_info(&self) -> ClientConfig {
        let mut capabilities = ClientCapabilities::default();
        capabilities.roots = Some(Default::default());
        ClientConfig::new(
            capabilities,
            Implementation::new("wafflehouse", crate::VERSION),
        )
        .with_protocol_version(ProtocolVersion::V_2025_11_25)
    }
    async fn list_roots(
        &self,
        _: RequestContext<RoleClient>,
    ) -> std::result::Result<ListRootsResult, rmcp::ErrorData> {
        Ok(ListRootsResult::new(self.roots.clone()))
    }
    async fn on_tool_list_changed(&self, _: NotificationContext<RoleClient>) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
    async fn on_progress(
        &self,
        params: ProgressNotificationParam,
        _: NotificationContext<RoleClient>,
    ) {
        // Display-only data: never scrape the message for a run identity.
        if let Ok(mut value) = serde_json::to_value(params) {
            if let Some(message) = value["message"].as_str() {
                value["message"] = json!(message.chars().take(2048).collect::<String>());
            }
            self.progress.send_replace(Some(value));
        }
    }
}
#[derive(Clone, serde::Serialize)]
pub struct Declaration {
    pub document: Value,
    pub revision: String,
    pub schemas: BTreeMap<String, Value>,
    pub generation: u64,
}
/// Keeps ownership even when initialization fails; close never force-kills a child.
pub struct ToolClient {
    pub descriptor: Descriptor,
    pub session: String,
    pub(crate) gate: Gate,
    child: Mutex<Child>,
    pipes: std::sync::Mutex<Option<(ChildStdout, ChildStdin)>>,
    service: Mutex<Option<RunningService<RoleClient, Handler>>>,
    peer: Option<Peer<RoleClient>>,
    generation: Arc<AtomicU64>,
    progress: watch::Sender<Option<Value>>,
}
impl ToolClient {
    pub fn spawn(descriptor: Descriptor) -> Result<Self> {
        let descriptor = descriptor.validate()?;
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|e| Error::Invalid(e.to_string()))?;
        let session = random.iter().map(|b| format!("{b:02x}")).collect();
        let mut child = Command::new(&descriptor.executable)
            .args(&descriptor.args)
            .current_dir(&descriptor.cwd)
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::Invalid("Missing stdout pipe".into()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::Invalid("Missing stdin pipe".into()))?;
        let (progress, _) = watch::channel(None);
        Ok(Self {
            descriptor,
            session,
            gate: Gate::default(),
            child: Mutex::new(child),
            pipes: std::sync::Mutex::new(Some((stdout, stdin))),
            service: Mutex::new(None),
            peer: None,
            generation: Arc::new(AtomicU64::new(0)),
            progress,
        })
    }
    pub async fn initialize(&mut self) -> Result<()> {
        let roots = self
            .descriptor
            .roots
            .iter()
            .map(|path| {
                let uri = url::Url::from_directory_path(path)
                    .map_err(|_| Error::Invalid("Invalid root URI".into()))?;
                Ok(Root::new(uri.to_string()))
            })
            .collect::<Result<Vec<_>>>()?;
        let (stdout, stdin) = self
            .pipes
            .get_mut()
            .unwrap_or_else(|e| e.into_inner())
            .take()
            .ok_or_else(|| Error::Invalid("Tool already initialized".into()))?;
        let handler = Handler {
            roots,
            generation: self.generation.clone(),
            progress: self.progress.clone(),
        };
        let transport =
            AsyncRwTransport::new_client(stdout, stdin).with_max_line_length(32 * 1024 * 1024);
        let service = tokio::time::timeout(
            CONTROL_TIMEOUT,
            serve_client(
                handler,
                RecordedTransport {
                    inner: transport,
                    gate: self.gate.clone(),
                    generation: self.generation.clone(),
                },
            ),
        )
        .await?
        .map_err(|e| Error::Invalid(format!("MCP initialize failed: {e}")))?;
        let compatible = service.peer_info().is_some_and(|info| {
            info.protocol_version == ProtocolVersion::V_2025_11_25
                && info.capabilities.tools.is_some()
                && info.capabilities.resources.is_some()
        });
        if compatible {
            self.peer = Some(service.peer().clone());
        }
        *self.service.get_mut() = Some(service);
        if !compatible {
            return Err(Error::Invalid(
                "Tool must support MCP 2025-11-25, tools and resources".into(),
            ));
        }
        Ok(())
    }
    pub(crate) fn peer(&self) -> Result<&Peer<RoleClient>> {
        self.peer
            .as_ref()
            .filter(|p| !p.is_transport_closed())
            .ok_or_else(|| Error::Invalid("Tool is disconnected".into()))
    }
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::SeqCst)
    }
    pub fn progress(&self) -> watch::Receiver<Option<Value>> {
        self.progress.subscribe()
    }
    async fn control(&self, method: &str, params: Value) -> Result<Value> {
        let request: ClientRequest =
            serde_json::from_value(json!({"method":method,"params":params}))?;
        let result =
            tokio::time::timeout(CONTROL_TIMEOUT, self.peer()?.send_request(request)).await??;
        Ok(serde_json::to_value(result)?)
    }
    pub async fn describe(&self) -> Result<Declaration> {
        let generation = self.generation();
        let mut cursor = Value::Null;
        let mut seen = HashSet::new();
        let mut schemas = BTreeMap::new();
        for page in 0..64 {
            let params = if cursor.is_null() {
                json!({})
            } else {
                json!({"cursor":cursor})
            };
            let result = self.control("tools/list", params).await?;
            for tool in result["tools"]
                .as_array()
                .ok_or_else(|| Error::Invalid("Missing tools list".into()))?
            {
                let name = tool["name"]
                    .as_str()
                    .ok_or_else(|| Error::Invalid("Missing tool name".into()))?;
                validator(&tool["inputSchema"])?;
                if schemas
                    .insert(name.to_string(), tool["inputSchema"].clone())
                    .is_some()
                    || schemas.len() > 1024
                {
                    return Err(Error::Invalid(
                        "Duplicate or excessive tool declarations".into(),
                    ));
                }
            }
            cursor = result["nextCursor"].clone();
            if cursor.is_null() {
                break;
            }
            if page == 63 || !seen.insert(cursor.to_string()) {
                return Err(Error::Invalid("Tool pagination did not terminate".into()));
            }
        }
        if !schemas.contains_key("describe") {
            return Err(Error::Invalid("Tool must expose describe".into()));
        }
        let response = self
            .control("tools/call", json!({"name":"describe","arguments":{}}))
            .await?;
        if response["isError"] == true {
            return Err(Error::Invalid("Tool description failed".into()));
        }
        let document = structured_result(&response)?;
        validate_contract("urn:table-manners:core:description", &document)?;
        for action in document["actions"]
            .as_array()
            .ok_or_else(|| Error::Invalid("Missing actions".into()))?
        {
            validator(&action["params_schema"])?;
            if !action["name"]
                .as_str()
                .is_some_and(|name| schemas.contains_key(name))
            {
                return Err(Error::Invalid("Declared action has no MCP tool".into()));
            }
        }
        if generation != self.generation() {
            return Err(Error::Invalid(
                "Tool declarations changed while reading".into(),
            ));
        }
        let revision = Sha256::digest(serde_json::to_vec(&json!([document, schemas, generation]))?)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        Ok(Declaration {
            document,
            revision,
            schemas,
            generation,
        })
    }
    pub async fn read_resource(&self, uri: &str) -> Result<Value> {
        self.control("resources/read", json!({"uri":uri})).await
    }
    /// Transport loss alone is not proof that the owned process stopped.
    pub async fn has_exited(&self) -> Result<bool> {
        Ok(self.child.lock().await.try_wait()?.is_some())
    }
    pub async fn confirm_exit(&self) -> Result<bool> {
        let mut child = self.child.lock().await;
        if child.try_wait()?.is_some() {
            return Ok(true);
        }
        if self
            .peer
            .as_ref()
            .is_some_and(|peer| peer.is_transport_closed())
        {
            return match tokio::time::timeout(Duration::from_millis(250), child.wait()).await {
                Ok(status) => {
                    status?;
                    Ok(true)
                }
                Err(_) => Ok(false),
            };
        }
        Ok(false)
    }
    /// Returns false when EOF did not stop the process. The owner retains the handle for explicit force quit.
    pub async fn close(&self) -> Result<bool> {
        self.pipes.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(service) = self.service.lock().await.as_mut() {
            let _ = service.close_with_timeout(Duration::from_secs(2)).await;
        }
        match tokio::time::timeout(Duration::from_secs(2), self.child.lock().await.wait()).await {
            Ok(status) => {
                status?;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }
}
pub fn structured_result(response: &Value) -> Result<Value> {
    if let Some(value) = response.get("structuredContent") {
        return Ok(value.clone());
    }
    let content = response["content"]
        .as_array()
        .ok_or_else(|| Error::Invalid("Missing result content".into()))?;
    let mut text = content.iter().filter(|v| v["type"] == "text");
    let value = text
        .next()
        .and_then(|v| v["text"].as_str())
        .ok_or_else(|| Error::Invalid("Missing JSON result".into()))?;
    if text.next().is_some() {
        return Err(Error::Invalid("Ambiguous text result".into()));
    }
    Ok(serde_json::from_str(value)?)
}
