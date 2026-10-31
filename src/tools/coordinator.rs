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
}
pub struct Coordinator {
    pub assets: AssetStore,
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
        let tools:Vec<_>=self.tools.iter().map(|(id,entry)|json!({"id":id,"declaration":entry.declaration,"error":entry.error,"busy":entry.reservation.available_permits()==0})).collect();
        Ok(json!({"tools":tools,"attempts":self.attempts.list(None,50).await?}))
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
                    let reference=json!({"file_sha256":asset.sha256,"width":asset.width,"height":asset.height,"projection":"flat","topology":"bounded"});
                    arguments.insert(name.into(),json!({"path":path,"expect":reference}));
                    inputs.insert(name.into(),json!([{ "asset":asset,"image":reference,"role":slot["role"] }]));
                }
            }
            arguments.insert("return_images".into(),json!("path"));arguments.insert("image_blocks".into(),json!(false));
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
                serde_json::to_value(&current)?,
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
        *client.gate.lock().unwrap_or_else(|e| e.into_inner()) = Some(Permit {
            id: request.id.clone(),
            store: self.attempts.clone(),
            action: request.action.clone(),
            arguments: arguments.clone(),
            generation: current.generation,
        });
        self.active
            .lock()
            .await
            .insert(request.id.clone(), Active { _permit: permit });
        let coordinator = self.clone();
        let worker = tokio::spawn(async move {
            coordinator.execute(client, request, arguments).await;
        });
        self.workers.lock().await.push(worker);
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
                let saved = self
                    .attempts
                    .update(&request.id, move |attempt| {
                        attempt.execution =
                            result["status"].as_str().unwrap_or("interrupted").into();
                        attempt.result = Some(result);
                        attempt.preservation["contract"] = json!("incomplete");
                        attempt
                            .notices
                            .push("Output and journal capture has not completed.".into());
                        Ok(())
                    })
                    .await;
                if saved.is_ok() {
                    self.active.lock().await.remove(&request.id);
                }
            }
            Err(error) => {
                let message = error.to_string();
                let _ = self
                    .attempts
                    .update(&request.id, move |attempt| {
                        attempt.execution = "interrupted".into();
                        attempt.preservation["contract"] = json!("incomplete");
                        attempt.notices.push(message);
                        Ok(())
                    })
                    .await;
            }
        }
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
            for task in self.workers.lock().await.drain(..) {
                let _ = task.await;
            }
            self.active.lock().await.clear();
        }
        stopped
    }
}
