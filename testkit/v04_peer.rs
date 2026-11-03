//! Independent v0.4 peer. No host types, MCP SDK or providers.
use super::{
    v04_fixtures,
    v04_store::{self, Store},
};
#[path = "v04_calls.rs"]
mod calls;
#[path = "v04_jobs.rs"]
mod jobs;
use base64::{Engine, engine::general_purpose::STANDARD};
use calls::Calls;
use jobs::Jobs;
use serde_json::{Value, json};
use std::{
    io::{self, BufRead, Read, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

struct Configuration {
    tool: String,
    allowed: Vec<PathBuf>,
    hold_operation: bool,
    priced_operations: bool,
    export_fault: Option<String>,
    hold_control: bool,
    tiny_inline: bool,
}
impl Configuration {
    fn parse(arguments: Vec<String>) -> Result<Self, String> {
        let mut result = Self {
            tool: "sample-lab".into(),
            allowed: vec![],
            hold_operation: false,
            priced_operations: false,
            export_fault: None,
            hold_control: false,
            tiny_inline: false,
        };
        let mut arguments = arguments.into_iter();
        while let Some(flag) = arguments.next() {
            if flag == "--hold-control" {
                result.hold_control = true;
                continue;
            }
            if flag == "--tiny-inline" {
                result.tiny_inline = true;
                continue;
            }
            if flag == "--priced-operations" {
                result.priced_operations = true;
                continue;
            }
            if flag == "--hold-operation" {
                result.hold_operation = true;
                continue;
            }
            let value = arguments
                .next()
                .ok_or_else(|| format!("Missing value for {flag}"))?;
            match flag.as_str() {
                "--export-fault" => result.export_fault = Some(value),
                "--allow-dir" => {
                    let path = PathBuf::from(value);
                    if !path.is_absolute() {
                        return Err("--allow-dir requires an absolute directory".into());
                    }
                    let path = path
                        .canonicalize()
                        .map_err(|_| "Cannot open --allow-dir directory")?;
                    if !path.is_dir() {
                        return Err("--allow-dir is not a directory".into());
                    }
                    if !result.allowed.contains(&path) {
                        result.allowed.push(path);
                    }
                }
                "--tool-name" => {
                    if !value.starts_with(|c: char| c.is_ascii_lowercase())
                        || !value
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
                    {
                        return Err("Invalid test tool name".into());
                    }
                    result.tool = value;
                }
                _ => return Err(format!("Unknown test peer flag {flag}")),
            }
        }
        Ok(result)
    }
}

fn error(code: i64, message: &str) -> Value {
    json!({"error":{"code":code,"message":message}})
}
fn result(value: Value) -> Value {
    json!({"result":value})
}
fn structured(value: Value) -> Value {
    result(
        json!({"content":[{"type":"text","text":value.to_string()}],"structuredContent":value,"isError":false}),
    )
}

pub fn serve(root: &Path, arguments: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let configuration = match Configuration::parse(arguments) {
        Ok(value) => value,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(64);
        }
    };
    let workspace = root.canonicalize()?;
    let root = workspace.as_path();
    let tool = &configuration.tool;
    let store =
        Arc::new(Store::new(root, tool, configuration.allowed.clone()).map_err(|e| e.to_string())?);
    let documents = v04_fixtures::documents(tool);
    let mut description = documents
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:core:description")
        .unwrap()
        .1
        .clone();
    if configuration.priced_operations {
        for action in description["actions"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .filter(|action| action["kind"] == "operation")
        {
            action["cost"] = json!({"may_spend":true,"basis":"estimated"});
        }
    }
    if configuration.tiny_inline {
        description["limits"]["inline_bytes"] = json!(1);
    }
    let mut quotes = std::collections::HashMap::<String, std::time::Instant>::new();
    let mut session = documents
        .iter()
        .find(|(id, _)| *id == "urn:table-manners:core:session")
        .unwrap()
        .1
        .clone();
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)?;
    bytes[6] = (bytes[6] & 15) | 64;
    bytes[8] = (bytes[8] & 63) | 128;
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    session["id"] = json!(format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    ));
    session["started"] = json!(
        time::OffsetDateTime::now_utc().format(&time::format_description::well_known::Rfc3339)?
    );
    session["allowed_dirs"] = json!(configuration.allowed);
    let calls = Arc::new(Calls::new(root, tool, session["id"].as_str().unwrap()));
    let output = Arc::new(Mutex::new(io::stdout()));
    let worker_output = output.clone();
    let mut jobs = Jobs::new(
        root,
        calls.clone(),
        store.clone(),
        configuration.hold_operation,
        move |message| write_message(&worker_output, &message).is_ok(),
    );
    let mut input = io::stdin().lock();
    let mut initialized = false;
    let control_done = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut control_workers = Vec::new();
    loop {
        let mut line = Vec::new();
        if (&mut input).take(2097153).read_until(b'\n', &mut line)? == 0 {
            break;
        }
        if line.len() > 2097152 {
            return Err("Request exceeds the v0.4 probe limit".into());
        }
        let message: Value = serde_json::from_slice(&line)?;
        let Some(id) = message.get("id") else {
            if initialized && message["method"] == "notifications/cancelled" {
                jobs.cancel(&message["params"]["requestId"]);
            }
            continue;
        };
        let params = &message["params"];
        let mut response = match message["method"].as_str().unwrap_or("") {
            "initialize" if !initialized => {
                if params["protocolVersion"] != "2025-11-25" {
                    error(-32602, "Only protocol 2025-11-25 is supported")
                } else {
                    initialized = true;
                    result(
                        json!({"protocolVersion":"2025-11-25","capabilities":{"tools":{},"resources":{}},"serverInfo":{"name":tool,"version":"0.1.0"}}),
                    )
                }
            }
            _ if !initialized => error(-32000, "Initialize the test peer first"),
            "ping" => result(json!({})),
            "tools/list" => {
                let mut tools = vec![
                    json!({"name":"describe","description":"Describe the v0.4 test actions.","inputSchema":{"type":"object","properties":{},"additionalProperties":false}}),
                    json!({"name":"export","description":"Export held synthetic artifacts.","inputSchema":Calls::control_schema("export-request")}),
                    json!({"name":"import","description":"Import complete portable synthetic records.","inputSchema":Calls::control_schema("import-request")}),
                ];
                if configuration.priced_operations {
                    tools.push(json!({"name":"estimate","description":"Price one synthetic operation.","inputSchema":{"type":"object","required":["action","arguments"],"properties":{"action":{"type":"string"},"arguments":{"type":"object"}},"additionalProperties":false}}));
                }
                tools.extend(calls.catalogue());
                result(json!({"tools":tools}))
            }
            "tools/call" if params["name"] == "describe" => {
                if params["arguments"]
                    .as_object()
                    .is_some_and(|args| args.is_empty())
                {
                    structured(description.clone())
                } else {
                    error(-32602, "describe takes an empty object")
                }
            }
            "tools/call" if params["name"] == "estimate" && configuration.priced_operations => {
                match calls.estimate(&store, &params["arguments"], &session) {
                    Ok(quote) => {
                        quotes.insert(
                            quote["binding"].as_str().unwrap().into(),
                            std::time::Instant::now(),
                        );
                        structured(quote)
                    }
                    Err(error) => json!({"error":error}),
                }
            }
            "tools/call" if params["name"] == "export" || params["name"] == "import" => {
                let response = if params["name"] == "export" {
                    store.export(params["arguments"].clone())
                } else {
                    store.import(params["arguments"].clone())
                };
                match response {
                    Ok(mut value) => {
                        if params["name"] == "export" {
                            match configuration.export_fault.as_deref() {
                                Some("hash") => value["file"]["sha256"] = json!("0".repeat(64)),
                                Some("path") => value["file"]["path"] = json!("/etc/hosts"),
                                Some("media") => value["file"]["media_type"] = json!("image/png"),
                                _ => (),
                            }
                        }
                        structured(value)
                    }
                    Err(value) => json!({"error":value}),
                }
            }
            "resources/list" => result(json!({"resources":[
                {"uri":format!("{tool}://session"),"name":"Session","mimeType":"application/json"},
                {"uri":format!("{tool}://runs"),"name":"Runs","mimeType":"application/json"}
            ]})),
            "tools/call" => {
                let priced = configuration.priced_operations
                    && description["actions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|action| {
                            action["name"] == params["name"] && action["cost"]["may_spend"] == true
                        });
                let authorization = if priced {
                    calls.estimate(&store,&json!({"action":params["name"],"arguments":params["arguments"]}),&session).and_then(|quote| {
                        let binding=quote["binding"].as_str().unwrap();
                        if params["_meta"]["table-manners/estimate"]==binding && quotes.get(binding).is_some_and(|created|created.elapsed()<std::time::Duration::from_secs(60)) {Ok(())}
                        else {Err(json!({"code":-32602,"message":"Estimate no longer matches","data":{"reason":"stale_estimate","run":null}}))}
                    })
                } else {
                    Ok(())
                };
                match authorization.and_then(|()| jobs.submit(id.clone(), params.clone())) {
                    Ok(()) => continue,
                    Err(error) => json!({"error":error}),
                }
            }
            "resources/read" => {
                let uri = params["uri"].as_str().unwrap_or("");
                if let Some(content) = calls.resource(uri) {
                    match content {
                        Ok(content) => result(json!({"contents":[content]})),
                        Err(error) => json!({"error":error}),
                    }
                } else if let Some(identity) = uri.strip_prefix(&format!("{tool}://artifact/")) {
                    let response = if let Some(identity) = identity.strip_suffix("/preview") {
                        store.preview(identity).map(|bytes| json!({"uri":uri,"mimeType":"image/png","blob":STANDARD.encode(bytes)}))
                    } else {
                        store.read(identity).map(|value| json!({"uri":uri,"mimeType":"application/json","text":value.to_string()}))
                    };
                    match response {
                        Ok(content) => result(json!({"contents":[content]})),
                        Err(value) => json!({"error":value}),
                    }
                } else {
                    let value = if uri == format!("{tool}://session") {
                        Some(session.clone())
                    } else {
                        None
                    };
                    match value {
                        Some(value) => result(
                            json!({"contents":[{"uri":uri,"mimeType":"application/json","text":value.to_string()}]}),
                        ),
                        None => error(-32002, "Unknown probe resource"),
                    }
                }
            }
            _ => error(-32601, "Unknown probe method"),
        };
        response["jsonrpc"] = json!("2.0");
        response["id"] = id.clone();
        if configuration.hold_control
            && message["method"] == "tools/call"
            && matches!(params["name"].as_str(), Some("export" | "import"))
            && !root.join("release-control").exists()
        {
            let release = root.join("release-control");
            let marker = root.join(format!("holding-control-{}", control_workers.len()));
            std::fs::write(&marker, b"ready")?;
            let output = output.clone();
            let done = control_done.clone();
            control_workers.push(std::thread::spawn(move || {
                while !release.exists() && !done.load(std::sync::atomic::Ordering::Acquire) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                if !done.load(std::sync::atomic::Ordering::Acquire) {
                    let _ = write_message(&output, &response);
                }
                let _ = std::fs::remove_file(marker);
            }));
            continue;
        }
        write_message(&output, &response)?;
    }
    control_done.store(true, std::sync::atomic::Ordering::Release);
    for worker in control_workers {
        let _ = worker.join();
    }
    jobs.shutdown();
    Ok(())
}

fn write_message(output: &Mutex<io::Stdout>, message: &Value) -> io::Result<()> {
    let mut output = output.lock().unwrap();
    writeln!(output, "{message}")?;
    output.flush()
}
