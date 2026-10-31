use super::{PeerState, fixtures, reply, send};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::{self, Cursor, Read, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

pub fn description(scenario: &str) -> Value {
    let mut document = fixtures::description(if scenario == "unsupported-contract" {
        "0.2.0"
    } else {
        "0.1.0"
    });
    if scenario == "paid" {
        document["actions"][0]["cost"]["may_spend"] = json!(true);
        document["actions"][0]["cost"]["basis"] = json!("estimated");
    }
    document
}

pub fn input_schema(action: &Value) -> Value {
    let mut schema = action["params_schema"].clone();
    schema["properties"]["source"] = json!({"$ref":"urn:table-manners:images:image-in"});
    schema["properties"]["return_images"] = json!({"enum":["path","inline","resource"]});
    schema["properties"]["image_blocks"] = json!({"type":"boolean"});
    schema["required"]
        .as_array_mut()
        .unwrap()
        .push(json!("source"));
    schema
}

struct Embedded;
impl jsonschema::Retrieve for Embedded {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let (_, text) = table_manners::SCHEMAS
            .iter()
            .find(|(id, _)| *id == uri.as_str())
            .ok_or("unknown schema")?;
        Ok(serde_json::from_str(text)?)
    }
}

pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    let validator = jsonschema::options()
        .with_retriever(Embedded)
        .build(schema)
        .map_err(|error| error.to_string())?;
    validator.validate(value).map_err(|error| error.to_string())
}

fn timestamp() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

pub fn execute(
    output: Arc<Mutex<io::Stdout>>,
    state: Arc<Mutex<PeerState>>,
    root: PathBuf,
    scenario: String,
    message: Value,
    cancelled: Arc<AtomicBool>,
) {
    if cancelled.load(Ordering::Acquire) {
        state
            .lock()
            .unwrap()
            .active
            .retain(|(id, _)| id != &message["id"]);
        return;
    }
    let params = &message["params"]["arguments"];
    let started = Instant::now();
    let mut run = fixtures::result("completed");
    let id = format!("run-{:04}", state.lock().unwrap().runs.len() + 1);
    let directory = root.join(&id);
    let resource = format!("{}://run/{id}/run.json", fixtures::TOOL);
    run["run"] = json!({"id":id,"dir":directory,"resource":resource});
    run["started"] = json!(timestamp());
    run["status"] = json!("running");
    run["images"]["in"] = json!([]);
    run["notes"] =
        json!(["Illustrative offline test output; not an image repair or quality evaluation."]);
    let identity: String = Sha256::digest(params.to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    run["input_identity"] = json!(format!("sha256:{identity}"));
    run["body"] = json!({"illustrative":true,"parameters":params});
    state.lock().unwrap().runs.push(run.clone());
    let mut progress = 0;
    let mut notify = |text: &str| {
        progress += 1;
        if let Some(token) = message["params"]["_meta"].get("progressToken") {
            send(
                &output,
                json!({"jsonrpc":"2.0","method":"notifications/progress","params":{"progressToken":token,"progress":progress,"message":text}}),
            );
        }
        if let Ok(mut journal) = OpenOptions::new()
            .create(true)
            .append(true)
            .open(directory.join("journal.ndjson"))
        {
            let _ = writeln!(journal, "{}", json!({"step":progress,"message":text}));
        }
    };
    let operation = (|| -> Result<(), Box<dyn std::error::Error>> {
        fs::create_dir_all(&directory)?;
        writeln!(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(root.join("calls.ndjson"))?,
            "{}",
            json!({"name":"mark_band","arguments":params})
        )?;
        notify(&format!("{resource} started"));
        let delay = if scenario == "slow" { 10_000 } else { 100 };
        for tick in 0..(delay / 50) {
            if cancelled.load(Ordering::Acquire) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(50));
            if tick % 10 == 9 {
                notify("Waiting in the offline test tool");
            }
        }
        let roots = state.lock().unwrap().roots.clone();
        let (input_reference, output_reference, output_bytes) = mark(params, &roots)?;
        run["images"]["in"] = json!([{"slot":"source","role":"source","image":input_reference}]);
        let path = directory.join("candidate.png");
        fs::write(&path, &output_bytes)?;
        let mut image = json!({"role":"candidate","image":output_reference});
        match params["return_images"].as_str().unwrap_or("path") {
            "inline" => {
                image["inline"] =
                    json!({"media_type":"image/png","data":STANDARD.encode(&output_bytes)})
            }
            "resource" => {
                image["resource"] = json!(format!("{}://run/{id}/candidate.png", fixtures::TOOL))
            }
            _ => image["path"] = json!(path),
        }
        run["images"]["out"] = json!([image]);
        notify("Illustrative test band saved");
        Ok(())
    })();
    let is_cancelled = cancelled.load(Ordering::Acquire);
    run["status"] = json!(if is_cancelled {
        "cancelled"
    } else if operation.is_err() {
        "error"
    } else {
        "completed"
    });
    if !is_cancelled && let Err(error) = operation {
        let io_failure = error.downcast_ref::<io::Error>().is_some();
        run["status"] = json!(if io_failure { "error" } else { "invalid" });
        run["error"] = json!({"code":if io_failure {"io"} else {"input"},"retryable":false,"reason":error.to_string()});
    }
    run["finished"] = json!(timestamp());
    run["duration_ms"] = json!(started.elapsed().as_millis() as u64);
    if let Err(error) = fs::write(directory.join("run.json"), run.to_string()) {
        run["status"] = json!("error");
        run["error"] = json!({"code":"io","retryable":false,"reason":error.to_string()});
    }
    {
        let mut state = state.lock().unwrap();
        if let Some(saved) = state.runs.iter_mut().find(|saved| saved["run"]["id"] == id) {
            *saved = run.clone();
        }
        state.active.retain(|(id, _)| id != &message["id"]);
    }
    if !is_cancelled {
        let mut content = vec![json!({"type":"text","text":run.to_string()})];
        if params["image_blocks"] == true
            && let Ok(bytes) = fs::read(directory.join("candidate.png"))
        {
            content
                .push(json!({"type":"image","data":STANDARD.encode(bytes),"mimeType":"image/png"}));
        }
        reply(
            &output,
            &message["id"],
            json!({"structuredContent":run,"content":content,"isError":run["status"] != "completed"}),
        );
    }
}

fn mark(
    params: &Value,
    roots: &[PathBuf],
) -> Result<(Value, Value, Vec<u8>), Box<dyn std::error::Error>> {
    let source = &params["source"];
    let bytes = if let Some(path) = source["path"].as_str() {
        if !Path::new(path).is_absolute() {
            return Err("image path must be absolute".into());
        }
        let path = Path::new(path).canonicalize()?;
        if !roots.iter().any(|root| path.starts_with(root)) {
            return Err("image is outside MCP roots".into());
        }
        let mut bytes = Vec::new();
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(nix::libc::O_NONBLOCK)
            .open(path)?;
        if !file.metadata()?.is_file() {
            return Err("source must be a regular file".into());
        }
        file.take(128 * 1024 * 1024 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > 128 * 1024 * 1024 {
            return Err("image exceeds byte limit".into());
        }
        bytes
    } else {
        let bytes = STANDARD.decode(
            source["inline"]["data"]
                .as_str()
                .ok_or("missing inline image")?,
        )?;
        if bytes.len() > 16 * 1024 * 1024 {
            return Err("inline image exceeds limit".into());
        }
        bytes
    };
    let mut image = image::ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()?
        .decode()?
        .into_rgba8();
    let reference = fixtures::reference(&bytes, image.width(), image.height());
    if let Some(expected) = source.get("expect") {
        for field in ["file_sha256", "width", "height", "projection", "topology"] {
            if expected[field] != reference[field] {
                return Err("image fingerprint or frame differs from expect".into());
            }
        }
    }
    let strength = params["strength"].as_f64().unwrap_or(0.5);
    let color = if params["tone"] == "amber" {
        [245.0, 180.0, 65.0]
    } else {
        [90.0, 135.0, 245.0]
    };
    let middle = image.height() / 2;
    let half = (image.height() / 12).max(1);
    for (_, y, pixel) in image.enumerate_pixels_mut() {
        if y >= middle.saturating_sub(half) && y <= middle.saturating_add(half) {
            for channel in 0..3 {
                pixel[channel] = (f64::from(pixel[channel]) * (1.0 - strength)
                    + color[channel] * strength)
                    .round() as u8;
            }
        }
    }
    let mut output = Cursor::new(Vec::new());
    image.write_to(&mut output, image::ImageFormat::Png)?;
    let output = output.into_inner();
    Ok((
        reference,
        fixtures::reference(&output, image.width(), image.height()),
        output,
    ))
}

pub fn resource(state: &PeerState, root: &Path, uri: &str) -> Option<Value> {
    if uri == format!("{}://runs", fixtures::TOOL) {
        let runs: Vec<_> = state.runs.iter().rev().map(|run| json!({"id":run["run"]["id"],"action":run["action"],"status":run["status"],"started":run["started"],"resource":run["run"]["resource"]})).collect();
        return Some(
            json!({"uri":uri,"mimeType":"application/json","text":json!({"contract":"0.1.0","tool":{"name":fixtures::TOOL,"version":"0.1.0"},"runs":runs}).to_string()}),
        );
    }
    for run in &state.runs {
        let id = run["run"]["id"].as_str()?;
        let base = format!("{}://run/{id}/", fixtures::TOOL);
        let Some(name) = uri.strip_prefix(&base) else {
            continue;
        };
        let file = match name {
            "run.json" => "run.json",
            "journal" => "journal.ndjson",
            "candidate.png" => "candidate.png",
            _ => continue,
        };
        let bytes = fs::read(root.join(id).join(file)).ok()?;
        return Some(if file.ends_with(".png") {
            json!({"uri":uri,"mimeType":"image/png","blob":STANDARD.encode(bytes)})
        } else {
            json!({"uri":uri,"mimeType":if file.ends_with(".ndjson") {"application/x-ndjson"} else {"application/json"},"text":String::from_utf8(bytes).ok()?})
        });
    }
    None
}
