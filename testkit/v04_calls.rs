//! Bounded, synchronous operation dispatch. Held workers and notifications are the next slice.
use super::{v04_fixtures, v04_store};
#[path = "v04_bindings.rs"]
mod v04_bindings;
#[path = "v04_results.rs"]
mod v04_results;
#[path = "v04_runs.rs"]
mod v04_runs;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{path::Path, time::Instant};
use v04_bindings::Failure;
use v04_store::Store;

pub struct Calls {
    tool: String,
    runs: v04_runs::Runs,
}
impl Calls {
    pub fn new(root: &Path, tool: &str, session: &str) -> Self {
        Self {
            tool: tool.into(),
            runs: v04_runs::Runs::new(root, tool, session),
        }
    }
    pub fn catalogue(&self) -> Vec<Value> {
        v04_fixtures::description(&self.tool)["actions"].as_array().unwrap().iter().map(|action| {
            json!({"name":action["name"],"description":action["purpose"],"inputSchema":v04_bindings::schemas::input_schema(action)})
        }).collect()
    }
    pub fn control_schema(name: &str) -> Value {
        let id = format!("urn:table-manners:operations:{name}");
        let text = table_manners_v04::SCHEMAS
            .iter()
            .find(|(uri, _)| *uri == id)
            .unwrap()
            .1;
        v04_bindings::schemas::standalone(&serde_json::from_str(text).unwrap())
    }
    pub fn call(&self, store: &Store, params: &Value) -> Result<Value, Value> {
        let client = params["_meta"].get("table-manners/client_run_id");
        if client.is_some_and(|value| !value.is_string()) {
            return Err(json!({"code":-32602,"message":"client_run_id must be a string"}));
        }
        let claim = self.runs.claim(client.and_then(Value::as_str))?;
        let action = params["name"].as_str().unwrap_or("");
        let mut arguments = params.get("arguments").cloned().unwrap_or(json!({}));
        if let Some(arguments) = arguments.as_object_mut() {
            arguments.entry("return_images").or_insert(json!("path"));
        }
        let prepared = match v04_bindings::prepare(store, &self.tool, action, arguments.clone()) {
            Err(Failure::Rpc(error)) => {
                self.runs.release(claim.as_ref());
                return Err(error);
            }
            other => other,
        };
        let elapsed = Instant::now();
        let info = match self.runs.allocate(action, claim.as_ref()) {
            Ok(info) => info,
            Err(error) => {
                self.runs.release(claim.as_ref());
                return Err(error);
            }
        };
        self.complete(store, &info, prepared, action, &arguments, elapsed)
    }
    fn complete(
        &self,
        store: &Store,
        info: &v04_results::RunInfo,
        prepared: Result<v04_bindings::Prepared, Failure>,
        action: &str,
        arguments: &Value,
        elapsed: Instant,
    ) -> Result<Value, Value> {
        let journal = self.runs.journal(info, "executing");
        let image_blocks = prepared
            .as_ref()
            .is_ok_and(|prepared| prepared.image_blocks);
        let mut run = match prepared {
            Ok(prepared) if journal.is_ok() => {
                v04_results::execute(store, &prepared, info, || false)
            }
            Ok(prepared) => v04_results::resolved(&prepared, info),
            Err(Failure::Invalid(reason)) => v04_results::invalid(info, action, arguments, &reason),
            Err(Failure::Rpc(_)) => unreachable!("RPC refusal returned before allocation"),
        };
        if let Err(error) = journal {
            Self::io_failure(&mut run, "Execution journal", &error);
        }
        self.respond(store, info, run, image_blocks, elapsed)
    }
    fn respond(
        &self,
        store: &Store,
        info: &v04_results::RunInfo,
        mut run: Value,
        image_blocks: bool,
        elapsed: Instant,
    ) -> Result<Value, Value> {
        let mut images = vec![];
        if image_blocks {
            for artifact in run["artifacts"]["out"].as_array().unwrap() {
                if !artifact["preview"].is_null() {
                    match store.preview(artifact["artifact"]["identity"].as_str().unwrap()) {
                        Ok(bytes) => images.push(json!({"type":"image","mimeType":"image/png","data":STANDARD.encode(bytes)})),
                        Err(error) => {
                            Self::io_failure(&mut run, "Image content", &error);
                            break;
                        }
                    }
                }
            }
        }
        v04_results::finish(&mut run, elapsed.elapsed());
        // Only failure to validate or publish final evidence escapes as a server error here.
        // Keep its claim: retrying execution would lose correlation with work already performed.
        let run = self.runs.finish(info, &run)?;
        let mut content = vec![json!({"type":"text","text":run.to_string()})];
        content.extend(images);
        Ok(
            json!({"isError":run["status"] != "completed","structuredContent":run,"content":content}),
        )
    }
    fn io_failure(run: &mut Value, context: &str, error: &Value) {
        let reason = format!(
            "{context}: {}",
            error["message"].as_str().unwrap_or("Storage failed")
        );
        if run["status"] == "completed" {
            v04_results::settle(
                run,
                v04_results::Outcome::Error { code: "io", reason },
                std::time::Duration::ZERO,
            );
        } else {
            // Delivery/recording trouble cannot replace an earlier input failure or cancellation.
            run["notes"].as_array_mut().unwrap().push(json!(reason));
        }
    }
    pub fn resource(&self, uri: &str) -> Option<Result<Value, Value>> {
        let runs = format!("{}://runs", self.tool);
        let value = if uri == runs {
            self.runs
                .list(None)
                .map(|page| ("application/json", page.to_string()))
        } else if let Some(cursor) = uri.strip_prefix(&format!("{runs}?cursor=")) {
            self.runs
                .list(Some(cursor))
                .map(|page| ("application/json", page.to_string()))
        } else {
            let path = uri.strip_prefix(&format!("{}://run/", self.tool))?;
            let (id, part) = path.split_once('/').unwrap_or((path, ""));
            self.runs.read(id, part).map(|text| {
                (
                    if part == "journal" {
                        "application/x-ndjson"
                    } else {
                        "application/json"
                    },
                    text,
                )
            })
        };
        Some(value.map(|(media, text)| json!({"uri":uri,"mimeType":media,"text":text})))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn saved(calls: &Calls, info: &v04_results::RunInfo) -> Value {
        serde_json::from_str(
            &calls
                .runs
                .read(info.reference["id"].as_str().unwrap(), "run.json")
                .unwrap(),
        )
        .unwrap()
    }
    #[test]
    fn missing_execution_journal_returns_the_saved_failure_without_starting_work() {
        let root = tempfile::tempdir().unwrap();
        let calls = Calls::new(
            root.path(),
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        );
        let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
        let info = calls.runs.allocate("summarize", None).unwrap();
        let args = json!({});
        let prepared = v04_bindings::prepare(&store, "sample-lab", "summarize", args.clone());
        std::fs::remove_file(Path::new(info.reference["dir"].as_str().unwrap()).join("journal"))
            .unwrap();
        let response = calls
            .complete(&store, &info, prepared, "summarize", &args, Instant::now())
            .expect("allocated failures are tool results");
        let run = &response["structuredContent"];
        assert_eq!(run["status"], "error");
        assert_eq!(run["error"]["code"], "io");
        assert_eq!(run["invocation"]["action"], "summarize");
        assert_eq!(run["artifacts"]["out"], json!([]));
        assert!(!root.path().join("artifacts").exists());
        assert_eq!(*run, saved(&calls, &info));
        assert_eq!(response["isError"], true);
        assert_eq!(
            serde_json::from_str::<Value>(response["content"][0]["text"].as_str().unwrap())
                .unwrap(),
            *run
        );
    }
    #[test]
    fn image_block_failure_is_saved_before_response_and_keeps_completed_outputs() {
        let root = tempfile::tempdir().unwrap();
        let calls = Calls::new(
            root.path(),
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        );
        let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
        let info = calls.runs.allocate("make_token", None).unwrap();
        let prepared = v04_bindings::prepare(&store, "sample-lab", "make_token", json!({"source":{"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}}})).unwrap();
        let run = v04_results::execute(&store, &prepared, &info, || false);
        let outputs = run["artifacts"]["out"].clone();
        std::fs::remove_dir_all(root.path().join("artifacts")).unwrap();
        let response = calls
            .respond(&store, &info, run, true, Instant::now())
            .expect("delivery failure is a tool result");
        let run = &response["structuredContent"];
        assert_eq!(run["status"], "error");
        assert_eq!(run["error"]["code"], "io");
        assert_eq!(run["artifacts"]["out"], outputs);
        assert_eq!(run["invocation"]["action"], "make_token");
        assert_eq!(*run, saved(&calls, &info));
        assert_eq!(response["isError"], true);
        assert_eq!(response["content"].as_array().unwrap().len(), 1);
    }
}
