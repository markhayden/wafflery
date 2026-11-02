//! Independent operation dispatch with acceptance, allocation and cancellation boundaries.
use super::{v04_fixtures, v04_store};
#[path = "v04_bindings.rs"]
mod v04_bindings;
#[path = "v04_cancellation.rs"]
mod v04_cancellation;
#[path = "v04_results.rs"]
mod v04_results;
#[path = "v04_runs.rs"]
mod v04_runs;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{path::Path, time::Instant};
use v04_bindings::Failure;
pub use v04_cancellation::Cancellation;
pub use v04_results::RunInfo;
use v04_store::Store;

pub struct Pending {
    action: String,
    arguments: Value,
    prepared: Result<v04_bindings::Prepared, Failure>,
    claim: Option<v04_runs::Claim>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    Allocated,
    Executed,
}

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
        let pending = self.accept(store, params)?;
        self.execute(store, pending, &Cancellation::default(), |_, _| {})
            .map(|reply| reply.expect("synchronous calls are never cancelled"))
    }
    pub fn accept(&self, store: &Store, params: &Value) -> Result<Pending, Value> {
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
        Ok(Pending {
            action: action.into(),
            arguments,
            prepared,
            claim,
        })
    }
    pub fn execute(
        &self,
        store: &Store,
        pending: Pending,
        cancellation: &Cancellation,
        mut observe: impl FnMut(&RunInfo, Stage),
    ) -> Result<Option<Value>, Value> {
        let elapsed = Instant::now();
        let info = match cancellation
            .allocate(|| self.runs.allocate(&pending.action, pending.claim.as_ref()))
        {
            Ok(Some(info)) => info,
            Ok(None) => return Ok(None), // Cancellation before allocation retains the claimed ID.
            Err(error) => {
                self.runs.release(pending.claim.as_ref());
                return Err(error);
            }
        };
        observe(&info, Stage::Allocated);
        self.complete(store, &info, pending, cancellation, &mut observe, elapsed)
    }
    fn complete(
        &self,
        store: &Store,
        info: &RunInfo,
        pending: Pending,
        cancellation: &Cancellation,
        observe: &mut impl FnMut(&RunInfo, Stage),
        elapsed: Instant,
    ) -> Result<Option<Value>, Value> {
        let Pending {
            action,
            arguments,
            prepared,
            ..
        } = pending;
        let journal = self.runs.journal(info, "executing");
        let image_blocks = prepared
            .as_ref()
            .is_ok_and(|prepared| prepared.image_blocks);
        let mut run = match prepared {
            Ok(prepared) if journal.is_ok() => {
                v04_results::execute(store, &prepared, info, || cancellation.cancelled())
            }
            Ok(prepared) => v04_results::resolved(&prepared, info),
            Err(Failure::Invalid(reason)) => {
                v04_results::invalid(info, &action, &arguments, &reason)
            }
            Err(Failure::Rpc(_)) => unreachable!("RPC refusal returned before allocation"),
        };
        if let Err(error) = journal {
            Self::io_failure(&mut run, "Execution journal", &error);
        }
        observe(info, Stage::Executed);
        self.respond(store, info, run, image_blocks, cancellation, elapsed)
    }
    fn respond(
        &self,
        store: &Store,
        info: &v04_results::RunInfo,
        mut run: Value,
        image_blocks: bool,
        cancellation: &Cancellation,
        elapsed: Instant,
    ) -> Result<Option<Value>, Value> {
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
        cancellation.finish(|cancelled| {
            if cancelled {
                // Publication is the terminal boundary: before it, cancellation wins (4.2).
                // Preserve any earlier failure details even though no MCP response is sent.
                if let Some(reason) = run["error"]["reason"].as_str().map(str::to_owned) {
                    run["notes"].as_array_mut().unwrap().push(json!(reason));
                }
                v04_results::settle(&mut run, v04_results::Outcome::Cancelled, elapsed.elapsed());
            } else {
                v04_results::finish(&mut run, elapsed.elapsed());
            }
            // Only failure to validate or publish final evidence escapes as a server error here.
            // Cancellation::finish suppresses even storage errors for a cancelled request.
            let run = self.runs.finish(info, &run)?;
            let mut content = vec![json!({"type":"text","text":run.to_string()})];
            content.extend(images);
            Ok(json!({"isError":run["status"] != "completed","structuredContent":run,"content":content}))
        })
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
            .complete(
                &store,
                &info,
                Pending {
                    action: "summarize".into(),
                    arguments: args,
                    prepared,
                    claim: None,
                },
                &Cancellation::default(),
                &mut |_, _| {},
                Instant::now(),
            )
            .expect("allocated failures are tool results")
            .unwrap();
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
            .respond(
                &store,
                &info,
                run,
                true,
                &Cancellation::default(),
                Instant::now(),
            )
            .expect("delivery failure is a tool result")
            .unwrap();
        let run = &response["structuredContent"];
        assert_eq!(run["status"], "error");
        assert_eq!(run["error"]["code"], "io");
        assert_eq!(run["artifacts"]["out"], outputs);
        assert_eq!(run["invocation"]["action"], "make_token");
        assert_eq!(*run, saved(&calls, &info));
        assert_eq!(response["isError"], true);
        assert_eq!(response["content"].as_array().unwrap().len(), 1);
    }
    fn image() -> Value {
        json!({"inline":{"media_type":"image/png","data":STANDARD.encode(include_bytes!("../ui/tests/fixtures/sample.png"))}})
    }
    #[test]
    fn cancelled_queued_calls_keep_their_id_without_allocating_a_run() {
        let root = tempfile::tempdir().unwrap();
        let calls = Calls::new(
            root.path(),
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        );
        let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
        let pending = calls.accept(&store, &json!({"name":"summarize","arguments":{},"_meta":{"table-manners/client_run_id":"queued"}})).unwrap();
        let cancellation = Cancellation::default();
        cancellation.cancel();
        assert!(
            calls
                .execute(&store, pending, &cancellation, |_, _| panic!(
                    "no allocation"
                ))
                .unwrap()
                .is_none()
        );
        assert_eq!(calls.runs.list(None).unwrap()["runs"], json!([]));
        let refused = calls.runs.claim(Some("queued")).unwrap_err();
        assert_eq!(refused["data"]["reason"], "client_run_id_reused");
        assert_eq!(refused["data"]["run"], Value::Null);
        assert!(!root.path().join("artifacts").exists());
    }
    #[test]
    fn allocated_cancellation_persists_invocation_and_only_completed_outputs_without_a_reply() {
        for stop_at in [Stage::Allocated, Stage::Executed] {
            let root = tempfile::tempdir().unwrap();
            let calls = Calls::new(
                root.path(),
                "sample-lab",
                "11111111-1111-4111-8111-111111111111",
            );
            let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
            let pending = calls.accept(&store, &json!({"name":"make_token","arguments":{"source":image()},"_meta":{"table-manners/client_run_id":"running"}})).unwrap();
            let cancellation = Cancellation::default();
            let mut reference = None;
            let response = calls
                .execute(&store, pending, &cancellation, |info, stage| {
                    reference = Some(info.reference.clone());
                    if stage == stop_at {
                        cancellation.cancel();
                    }
                })
                .unwrap();
            assert!(response.is_none());
            let reference = reference.unwrap();
            let run: Value = serde_json::from_str(
                &calls
                    .runs
                    .read(reference["id"].as_str().unwrap(), "run.json")
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(run["status"], "cancelled");
            assert_eq!(run["error"], Value::Null);
            assert_eq!(run["invocation"]["action"], "make_token");
            assert_eq!(run["spend"]["usd"], 0);
            assert_eq!(
                run["artifacts"]["out"].as_array().unwrap().len(),
                usize::from(stop_at == Stage::Executed)
            );
            assert_eq!(
                calls.runs.claim(Some("running")).unwrap_err()["data"]["run"]["id"],
                reference["id"]
            );
        }
    }
    #[test]
    fn finalization_wins_over_a_late_cancel_and_response_stays_equal_to_saved_evidence() {
        let root = tempfile::tempdir().unwrap();
        let calls = Calls::new(
            root.path(),
            "sample-lab",
            "11111111-1111-4111-8111-111111111111",
        );
        let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
        let pending = calls
            .accept(&store, &json!({"name":"summarize","arguments":{}}))
            .unwrap();
        let cancellation = Cancellation::default();
        let response = calls
            .execute(&store, pending, &cancellation, |_, _| {})
            .unwrap()
            .unwrap();
        cancellation.cancel();
        assert!(!cancellation.cancelled());
        let run = &response["structuredContent"];
        assert_eq!(run["status"], "completed");
        let saved: Value = serde_json::from_str(
            &calls
                .runs
                .read(run["run"]["id"].as_str().unwrap(), "run.json")
                .unwrap(),
        )
        .unwrap();
        assert_eq!(*run, saved);
    }
    #[test]
    fn cancelled_requests_never_reply_even_if_final_evidence_cannot_be_saved() {
        for cancelled in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let calls = Calls::new(
                root.path(),
                "sample-lab",
                "11111111-1111-4111-8111-111111111111",
            );
            let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
            let pending = calls.accept(&store, &json!({"name":"summarize","arguments":{},"_meta":{"table-manners/client_run_id":"unsaved"}})).unwrap();
            let cancellation = Cancellation::default();
            let response = calls.execute(&store, pending, &cancellation, |info, stage| {
                if stage == Stage::Executed {
                    std::fs::create_dir(
                        Path::new(info.reference["dir"].as_str().unwrap()).join("run.json"),
                    )
                    .unwrap();
                    if cancelled {
                        cancellation.cancel();
                    }
                }
            });
            if cancelled {
                assert!(response.unwrap().is_none());
            } else {
                assert_eq!(response.unwrap_err()["code"], -32603);
            }
            let refusal = calls.runs.claim(Some("unsaved")).unwrap_err();
            assert!(refusal["data"]["run"]["id"].is_string());
        }
    }
    #[test]
    fn cancellation_before_publication_retains_prior_error_details_in_notes() {
        for invalid in [false, true] {
            let root = tempfile::tempdir().unwrap();
            let calls = Calls::new(
                root.path(),
                "sample-lab",
                "11111111-1111-4111-8111-111111111111",
            );
            let store = Store::new(root.path(), "sample-lab", vec![]).unwrap();
            let source = if invalid {
                json!({"inline":{"media_type":"image/png","data":"e30="}})
            } else {
                image()
            };
            if !invalid {
                std::fs::write(root.path().join("previews"), "blocked").unwrap();
            }
            let pending = calls
                .accept(
                    &store,
                    &json!({"name":"make_token","arguments":{"source":source}}),
                )
                .unwrap();
            let cancellation = Cancellation::default();
            let response = calls
                .execute(&store, pending, &cancellation, |_, stage| {
                    if stage == Stage::Executed {
                        cancellation.cancel();
                    }
                })
                .unwrap();
            assert!(response.is_none());
            let listed = calls.runs.list(None).unwrap();
            assert_eq!(listed["runs"][0]["status"], "cancelled");
            let run: Value = serde_json::from_str(
                &calls
                    .runs
                    .read(listed["runs"][0]["id"].as_str().unwrap(), "run.json")
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(run["status"], "cancelled");
            assert_eq!(run["error"], Value::Null);
            assert!(!run["notes"][0].as_str().unwrap().is_empty());
            assert_eq!(run["invocation"].is_null(), invalid);
            assert_eq!(
                run["artifacts"]["out"].as_array().unwrap().len(),
                usize::from(!invalid)
            );
        }
    }
}
