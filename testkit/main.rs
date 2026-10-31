//! Independent, line-delimited MCP peer. Deliberately does not import rmcp or wafflehouse.
mod fixtures;
mod scenarios;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Read, Write},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

#[derive(Default)]
pub struct PeerState {
    pub roots: Vec<PathBuf>,
    pub runs: Vec<Value>,
    pub active: Vec<(Value, Arc<AtomicBool>)>,
}

pub fn send(output: &Arc<Mutex<io::Stdout>>, message: Value) {
    let mut output = output.lock().expect("stdout lock");
    let _ = writeln!(output, "{message}");
    let _ = output.flush();
}

fn reply(output: &Arc<Mutex<io::Stdout>>, id: &Value, result: Value) {
    send(output, json!({"jsonrpc":"2.0", "id":id, "result":result}));
}

fn reject(output: &Arc<Mutex<io::Stdout>>, id: &Value, code: i64, message: &str) {
    send(
        output,
        json!({"jsonrpc":"2.0", "id":id, "error":{"code":code, "message":message}}),
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let root = PathBuf::from(
        arguments
            .next()
            .ok_or("expected a test workspace directory")?,
    );
    std::fs::create_dir_all(&root)?;
    let root = root.canonicalize()?;
    let scenario = arguments.next().unwrap_or_else(|| "normal".into());
    if scenario == "malformed" {
        println!("not JSON");
        return Ok(());
    }
    let output = Arc::new(Mutex::new(io::stdout()));
    let state = Arc::new(Mutex::new(PeerState::default()));
    let mut input = io::stdin().lock();
    let mut initialized = false;
    let mut workers = Vec::new();
    loop {
        let mut line = Vec::new();
        if (&mut input)
            .take(32 * 1024 * 1024 + 1)
            .read_until(b'\n', &mut line)?
            == 0
        {
            break;
        }
        if line.len() > 32 * 1024 * 1024 {
            return Err("request exceeds test peer limit".into());
        }
        let message: Value = serde_json::from_slice(&line)?;
        let id = &message["id"];
        let params = &message["params"];
        if id == "root-query" && message.get("result").is_some() {
            state.lock().unwrap().roots = message["result"]["roots"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|root| {
                    url::Url::parse(root["uri"].as_str()?)
                        .ok()?
                        .to_file_path()
                        .ok()?
                        .canonicalize()
                        .ok()
                })
                .collect();
            continue;
        }
        match message["method"].as_str().unwrap_or("") {
            "initialize" => {
                if params["protocolVersion"] != "2025-11-25" {
                    reject(&output, id, -32602, "only protocol 2025-11-25 is supported");
                    continue;
                }
                initialized = true;
                reply(
                    &output,
                    id,
                    json!({"protocolVersion": if scenario == "wrong-protocol" {"2026-07-28"} else {"2025-11-25"}, "capabilities":{"tools":{},"resources":{}}, "serverInfo":{"name":fixtures::TOOL,"version":"0.1.0"}}),
                );
            }
            "notifications/initialized" if initialized => send(
                &output,
                json!({"jsonrpc":"2.0","id":"root-query","method":"roots/list","params":{}}),
            ),
            "notifications/cancelled" => {
                for (request, cancelled) in &state.lock().unwrap().active {
                    if request == &params["requestId"] {
                        cancelled.store(true, Ordering::Release);
                    }
                }
            }
            "ping" if initialized => reply(&output, id, json!({})),
            "tools/list" if initialized => {
                let description = scenarios::description(&scenario);
                if params.get("cursor").is_none() {
                    reply(
                        &output,
                        id,
                        json!({"tools":[{"name":"describe","description":"Shared contract description", "inputSchema":{"type":"object","additionalProperties":false}}], "nextCursor":"actions"}),
                    );
                } else if params["cursor"] == "actions" {
                    reply(
                        &output,
                        id,
                        json!({"tools":[{"name":"mark_band","description":"Add an illustrative test band", "inputSchema":scenarios::input_schema(&description["actions"][0])}]}),
                    );
                } else {
                    reject(&output, id, -32602, "invalid cursor");
                }
            }
            "tools/call" if initialized && params["name"] == "describe" => {
                let description = scenarios::description(&scenario);
                reply(
                    &output,
                    id,
                    json!({"content":[{"type":"text","text":description.to_string()}],"structuredContent":description,"isError":false}),
                );
            }
            "tools/call" if initialized && params["name"] == "mark_band" => {
                if !state.lock().unwrap().active.is_empty() {
                    reject(&output, id, -32000, "test tool is busy");
                    continue;
                }
                if let Err(error) = scenarios::validate(
                    &scenarios::input_schema(&scenarios::description(&scenario)["actions"][0]),
                    &params["arguments"],
                ) {
                    reject(&output, id, -32602, &error);
                    continue;
                }
                let cancelled = Arc::new(AtomicBool::new(false));
                state
                    .lock()
                    .unwrap()
                    .active
                    .push((id.clone(), cancelled.clone()));
                let (output, state, root, scenario, message) = (
                    output.clone(),
                    state.clone(),
                    root.clone(),
                    scenario.clone(),
                    message.clone(),
                );
                workers.push(std::thread::spawn(move || {
                    scenarios::execute(output, state, root, scenario, message, cancelled)
                }));
            }
            "resources/list" if initialized => reply(
                &output,
                id,
                json!({"resources":[{"uri":format!("{}://runs",fixtures::TOOL),"name":"Recent runs","mimeType":"application/json"}]}),
            ),
            "resources/read" if initialized => {
                match scenarios::resource(
                    &state.lock().unwrap(),
                    &root,
                    params["uri"].as_str().unwrap_or(""),
                ) {
                    Some(content) => reply(&output, id, json!({"contents":[content]})),
                    None => reject(&output, id, -32002, "resource not found"),
                }
            }
            method if method.starts_with("notifications/") => (),
            _ => reject(&output, id, -32601, "unknown method or uninitialized peer"),
        }
    }
    for (_, cancelled) in &state.lock().unwrap().active {
        cancelled.store(true, Ordering::Release);
    }
    for worker in workers {
        let _ = worker.join();
    }
    Ok(())
}
