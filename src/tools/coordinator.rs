use super::{
    Error, Result,
    client::{Declaration, ToolClient, structured_result},
    dispatch_transport::Permit,
    registration::Descriptor,
    validate_contract, validator,
};
use crate::library::{
    assets::AssetStore,
    attempts::{Attempt, AttemptStore, Submission},
};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::BTreeMap, os::unix::fs::DirBuilderExt, sync::Arc};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

struct Entry {
    client: Option<Arc<ToolClient>>,
    declaration: Option<Declaration>,
    error: Option<String>,
    reservation: Arc<Semaphore>,
}
struct Active {
    _permit: OwnedSemaphorePermit,
    written: tokio::sync::watch::Receiver<bool>,
    cancel_requested: bool,
}
pub struct Coordinator {
    pub assets: AssetStore,
    pub events: crate::events::Events,
    pub attempts: AttemptStore,
    tools: BTreeMap<String, Entry>,
    submission: Mutex<()>,
    active: Mutex<BTreeMap<String, Active>>,
    workers: Mutex<Vec<tokio::task::JoinHandle<()>>>,
    stopping: AtomicBool,
}
impl Coordinator {
    pub async fn open(assets: AssetStore, descriptors: Vec<Descriptor>) -> Result<Self> {
        let attempts = AttemptStore::new(assets.database.clone());
        attempts.recover().await?;
        let execution = assets.root().join("execution");
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(&execution)?;
        let mut names = std::collections::HashSet::new();
        if descriptors.len() > 32 || descriptors.iter().any(|d| !names.insert(d.id.clone())) {
            return Err(Error::Invalid(
                "Duplicate or excessive registered tools".into(),
            ));
        }
        let mut tools = BTreeMap::new();
        for mut descriptor in descriptors {
            if tools.contains_key(&descriptor.id) || tools.len() >= 32 {
                return Err(Error::Invalid(
                    "Duplicate or excessive registered tools".into(),
                ));
            }
            descriptor.roots.push(execution.clone());
            let id = descriptor.id.clone();
            let (client, declaration, error) = match ToolClient::spawn(descriptor) {
                Ok(mut client) => {
                    let initialized = client.initialize().await;
                    let description = match initialized {
                        Ok(()) => client.describe().await,
                        Err(e) => Err(e),
                    };
                    let (declaration, error) = match description {
                        Ok(d) => (Some(d), None),
                        Err(e) => (None, Some(e.to_string())),
                    };
                    (Some(Arc::new(client)), declaration, error)
                }
                Err(e) => (None, None, Some(e.to_string())),
            };
            tools.insert(
                id,
                Entry {
                    client,
                    declaration,
                    error,
                    reservation: Arc::new(Semaphore::new(1)),
                },
            );
        }
        Ok(Self {
            assets,
            events: crate::events::Events::new()?,
            attempts,
            tools,
            submission: Mutex::new(()),
            active: Mutex::new(BTreeMap::new()),
            workers: Mutex::new(vec![]),
            stopping: AtomicBool::new(false),
        })
    }
    pub async fn description(&self, id: &str) -> Result<Declaration> {
        self.tools
            .get(id)
            .ok_or(Error::Missing("Tool not registered"))?
            .declaration
            .clone()
            .ok_or_else(|| Error::Invalid("This tool has no supported declaration".into()))
    }
    pub async fn snapshot(&self) -> Result<Value> {
        let tools:Vec<_>=self.tools.iter().map(|(id,entry)|json!({"id":id,"declaration":entry.declaration,"error":entry.error,"busy":entry.reservation.available_permits()==0,"progress":entry.client.as_ref().and_then(|client|client.progress().borrow().clone())})).collect();
        Ok(
            json!({"tools":tools,"attempts":self.attempts.list(None,50).await?,"event_session":self.events.session()}),
        )
    }
    pub async fn submit(self: &Arc<Self>, request: Submission) -> Result<Attempt> {
        if request.id.is_empty()
            || request.id.len() > 128
            || !request
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
        {
            return Err(Error::Invalid("Invalid submission ID".into()));
        }
        let _submission = self.submission.lock().await;
        if self.stopping.load(Ordering::Acquire) {
            return Err(Error::Conflict("The host is shutting down"));
        }
        if let Some(attempt) = self.attempts.find(&request).await? {
            return Ok(attempt);
        }
        let entry = self
            .tools
            .get(&request.tool)
            .ok_or(Error::Missing("Tool not registered"))?;
        let permit = entry.reservation.clone().try_acquire_owned().map_err(|_| {
            Error::Conflict("This tool is busy. Wait for its current attempt to finish.")
        })?;
        let client = entry
            .client
            .clone()
            .ok_or(Error::Missing("Tool is unavailable"))?;
        // Re-read the live declaration under the reservation. Never authorize using UI metadata.
        let current = client.describe().await?;
        if current.revision != request.revision {
            return Err(Error::Conflict(
                "Tool description changed. Reload its inputs before running.",
            ));
        }
        let action = current.document["actions"]
            .as_array()
            .and_then(|actions| actions.iter().find(|a| a["name"] == request.action))
            .ok_or(Error::Missing("Action not declared"))?;
        if action["cost"]["may_spend"].as_bool() != Some(false) {
            return Err(Error::Conflict(
                "This foundation only runs actions declared free and offline.",
            ));
        }
        validator(&action["params_schema"])?
            .validate(&request.params)
            .map_err(|e| Error::Invalid(format!("Invalid parameters: {e}")))?;
        let mut arguments = request
            .params
            .as_object()
            .cloned()
            .ok_or_else(|| Error::Invalid("Parameters must be an object".into()))?;
        if action["locations"]
            .as_array()
            .is_some_and(|slots| slots.iter().any(|s| s["required"] == true))
        {
            return Err(Error::Invalid(
                "Required location bindings need the operations interface".into(),
            ));
        }
        let slots = action["images"].as_array().cloned().unwrap_or_default();
        if request
            .images
            .keys()
            .any(|name| !slots.iter().any(|s| s["slot"] == *name))
        {
            return Err(Error::Invalid("Undeclared image slot".into()));
        }
        if request.frames.iter().any(|(slot, frames)| {
            request
                .images
                .get(slot)
                .is_none_or(|images| images.len() != frames.len())
        }) {
            return Err(Error::Invalid(
                "Image coordinate metadata must match its image bindings".into(),
            ));
        }
        let directory = self.assets.root().join("execution").join(&request.id);
        std::fs::DirBuilder::new().mode(0o700).create(&directory)?;
        let materialized=async {
            let mut inputs=serde_json::Map::new();
            for (index,slot) in slots.iter().enumerate() {
                let name=slot["slot"].as_str().ok_or_else(||Error::Invalid("Invalid image slot".into()))?;
                if arguments.contains_key(name) {return Err(Error::Invalid("Image slots cannot be supplied as raw parameters".into()));}
                let ids=request.images.get(name).cloned().unwrap_or_default();
                if ids.len()>1 || (slot["required"]==true && ids.len()!=1) {return Err(Error::Invalid(format!("Image slot {name} requires {} image",if slot["required"]==true {"one"}else{"at most one"})));}
                if let Some(id)=ids.first() {
                    let (asset,bytes)=self.assets.read(id).await?;
                    let path=directory.join(format!("input-{index}.{}",if asset.media_type=="image/png"{"png"}else{"jpg"}));
                    tokio::fs::write(&path,bytes).await?;
                    let frame=request.frames.get(name).and_then(|frames|frames.first()).cloned().unwrap_or_default();
                    let reference=json!({"file_sha256":asset.sha256,"width":asset.width,"height":asset.height,"projection":frame.projection,"topology":frame.topology});
                    validator(&json!({"$ref":"urn:table-manners:images:image-reference"}))?.validate(&reference).map_err(|e|Error::Invalid(format!("Invalid image coordinates: {e}")))?;
                    arguments.insert(name.into(),json!({"path":path,"expect":reference}));
                    inputs.insert(name.into(),json!([{ "asset":asset,"image":reference,"role":slot["role"] }]));
                }
            }
            if current.document["profiles"].as_array().is_some_and(|profiles|profiles.iter().any(|profile|profile=="images")) {
                arguments.insert("return_images".into(),json!("path"));arguments.insert("image_blocks".into(),json!(false));
            }
            let arguments=Value::Object(arguments);
            let schema=current.schemas.get(&request.action).ok_or(Error::Missing("No MCP action schema"))?;
            validator(schema)?.validate(&arguments).map_err(|e|Error::Invalid(format!("Invalid action inputs: {e}")))?;
            Ok::<_,Error>((arguments,Value::Object(inputs)))
        }.await;
        let (arguments, inputs) = match materialized {
            Ok(value) => value,
            Err(error) => {
                let _ = tokio::fs::remove_dir_all(&directory).await;
                return Err(error);
            }
        };
        let attempt = self
            .attempts
            .create(
                request.clone(),
                arguments.clone(),
                {
                    let mut declaration = serde_json::to_value(&current)?;
                    declaration["session"] = json!(client.session);
                    declaration
                },
                inputs,
            )
            .await;
        let attempt = match attempt {
            Ok(attempt) => attempt,
            Err(error) => {
                let _ = tokio::fs::remove_dir_all(&directory).await;
                return Err(error.into());
            }
        };
        let (written, dispatched) = tokio::sync::watch::channel(false);
        *client.gate.lock().unwrap_or_else(|e| e.into_inner()) = Some(Permit {
            id: request.id.clone(),
            store: self.attempts.clone(),
            action: request.action.clone(),
            arguments: arguments.clone(),
            generation: current.generation,
            written,
        });
        self.active.lock().await.insert(
            request.id.clone(),
            Active {
                _permit: permit,
                written: dispatched,
                cancel_requested: false,
            },
        );
        let coordinator = self.clone();
        let worker = tokio::spawn(async move {
            coordinator.execute(client, request, arguments).await;
        });
        self.workers.lock().await.push(worker);
        self.events.changed();
        Ok(attempt)
    }
    async fn execute(&self, client: Arc<ToolClient>, request: Submission, arguments: Value) {
        let result=async {
            let message=serde_json::from_value(json!({"method":"tools/call","params":{"name":request.action,"arguments":arguments}}))?;
            let result=client.peer()?.send_request(message).await?;
            let raw=serde_json::to_value(result)?;
            let result=structured_result(&raw)?;
            validate_contract("urn:table-manners:core:result",&result)?;
            if result["action"]!=request.action {return Err(Error::Invalid("Result action does not match the submitted action".into()));}
            Ok::<_,Error>(result)
        }.await;
        match result {
            Ok(result) => {
                let saved = async {
                    let attempt = self.attempts.get(&request.id).await?;
                    super::evidence::check_association(&attempt, &result)?;
                    let attempt = self.attempts.bind_result(&request.id, result).await?;
                    self.events.changed();
                    let (outputs, journal, notices) =
                        super::evidence::capture(&self.assets, &client, &attempt).await;
                    self.attempts
                        .update(&request.id, move |attempt| {
                            attempt.preservation["contract"] = json!(if notices.is_empty() {
                                "complete"
                            } else {
                                "incomplete"
                            });
                            attempt.outputs = outputs;
                            attempt.journal = journal;
                            attempt.notices.extend(notices);
                            Ok(())
                        })
                        .await?;
                    Ok::<_, Error>(())
                }
                .await;
                match saved {
                    Ok(()) => {
                        self.active.lock().await.remove(&request.id);
                    }
                    Err(error) => {
                        let message = error.to_string();
                        let _ = self
                            .attempts
                            .update(&request.id, move |attempt| {
                                if attempt.result.is_none() {
                                    attempt.execution = "interrupted".into();
                                }
                                attempt.preservation["contract"] = json!("incomplete");
                                attempt.notices.push(message);
                                Ok(())
                            })
                            .await;
                    }
                }
            }
            Err(error) => {
                let locally_cancelled =
                    matches!(error, Error::Mcp(rmcp::ServiceError::Cancelled { .. }));
                let message = error.to_string();
                let _ = self
                    .attempts
                    .update(&request.id, move |attempt| {
                        if attempt.execution == "rejected"
                            || (locally_cancelled && attempt.execution == "unconfirmed")
                        {
                            return Ok(());
                        }
                        attempt.execution = "interrupted".into();
                        attempt.preservation["contract"] = json!("incomplete");
                        attempt.notices.push(message);
                        Ok(())
                    })
                    .await;
            }
        }
        client.gate.lock().unwrap_or_else(|e| e.into_inner()).take();
        if self
            .attempts
            .get(&request.id)
            .await
            .is_ok_and(|a| a.execution == "rejected")
        {
            self.active.lock().await.remove(&request.id);
        }
        self.events.changed();
    }
    pub async fn cancel(self: &Arc<Self>, id: &str) -> Result<Attempt> {
        let mut active = self.active.lock().await;
        let Some(running) = active.get_mut(id) else {
            return Ok(self.attempts.get(id).await?);
        };
        if running.cancel_requested {
            return Ok(self.attempts.get(id).await?);
        }
        let attempt = self.attempts.request_cancel(id).await?;
        running.cancel_requested = true;
        let mut written = running.written.clone();
        if attempt.execution == "unconfirmed" {
            let client = self
                .tools
                .get(&attempt.request.tool)
                .and_then(|e| e.client.clone())
                .ok_or(Error::Missing("Tool is unavailable"))?;
            let request_id = attempt
                .wire
                .as_ref()
                .and_then(|wire| wire.get("id"))
                .cloned()
                .ok_or_else(|| Error::Invalid("Request identity missing".into()))?;
            let coordinator = self.clone();
            let id = id.to_owned();
            self.workers.lock().await.push(tokio::spawn(async move {
                // A cancellation must not overtake the call while its DB write is pending.
                if written.wait_for(|sent|*sent).await.is_ok() {
                    let sent=async {let params=serde_json::from_value(serde_json::json!({"requestId":request_id,"reason":"Cancelled by the workspace user"}))?;client.peer()?.notify_cancelled(params).await?;Ok::<_,Error>(())}.await;
                    if let Err(error)=sent {let message=format!("Cancellation could not be delivered: {error}");let _=coordinator.attempts.update(&id,move|a|{a.notices.push(message);Ok(())}).await;}
                    coordinator.events.changed();
                }
            }));
        }
        drop(active);
        self.events.changed();
        Ok(attempt)
    }

    pub async fn try_quit(&self) -> bool {
        let _submission = self.submission.lock().await;
        if !self.active.lock().await.is_empty() {
            return false;
        }
        self.stopping.store(true, Ordering::Release);
        true
    }
    pub async fn close(&self) -> bool {
        let mut stopped = true;
        for entry in self.tools.values() {
            if let Some(client) = &entry.client {
                stopped &= client.close().await.unwrap_or(false);
            }
        }
        if stopped {
            let tasks = self.workers.lock().await.drain(..).collect::<Vec<_>>();
            for task in tasks {
                let _ = task.await;
            }
            let ids = self.active.lock().await.keys().cloned().collect::<Vec<_>>();
            for id in ids {
                let _ = self
                    .attempts
                    .update(&id, |attempt| {
                        if attempt.result.is_none() && attempt.execution != "rejected" {
                            attempt.execution = "interrupted".into();
                            attempt.notices.push(
                                "Tool session ended without a confirmed final result.".into(),
                            );
                        }
                        Ok(())
                    })
                    .await;
            }
            self.active.lock().await.clear();
        }
        stopped
    }
}
