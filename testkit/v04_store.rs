//! Synthetic portable records owned by the independent peer; no host or real tool formats.
use super::v04_fixtures as fixtures;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use table_manners::operations::{ExportRequest, ImportRequest};

const INLINE_LIMIT: usize = 1048576;
const FILE_LIMIT: u64 = 4 * 1048576;
type Result<T> = std::result::Result<T, Value>;

pub struct Store {
    root: PathBuf,
    tool: String,
    allowed: Vec<PathBuf>,
}
fn invalid(message: &str) -> Value {
    json!({"code":-32602,"message":message,"data":{"reason":"invalid_arguments","run":null,"error":{"code":"input","retryable":false,"reason":message}}})
}
fn internal(error: impl std::fmt::Display) -> Value {
    json!({"code":-32603,"message":format!("Test artifact store: {error}")})
}
fn missing(port: Option<&str>) -> Value {
    json!({"code":-32602,"message":"Artifact is not held by this peer","data":fixtures::refusal("unresolved",port,None)})
}
fn hash(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn file_name(identity: &str) -> Option<&str> {
    identity.strip_prefix("sha256:").filter(|s| {
        s.len() == 64
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}
fn media(tool: &str, kind: &str) -> String {
    if kind == "location" {
        "application/json".into()
    } else {
        format!("application/x-{tool}-token")
    }
}

// Open every canonical component without following links again. A replacement symlink after
// the grant check cannot turn a permitted file into a read outside the permitted directory.
fn read_regular(path: &Path) -> std::io::Result<Vec<u8>> {
    use nix::{
        fcntl::{OFlag, openat},
        sys::stat::Mode,
    };
    if !path.is_absolute() {
        return Err(std::io::Error::other("absolute path required"));
    }
    let parts: Vec<_> = path
        .components()
        .filter(|part| !matches!(part, Component::RootDir))
        .collect();
    let mut handle = File::open("/")?;
    for (index, part) in parts.iter().enumerate() {
        let Component::Normal(name) = part else {
            return Err(std::io::Error::other("canonical path required"));
        };
        let mut flags = OFlag::O_RDONLY | OFlag::O_NOFOLLOW | OFlag::O_CLOEXEC | OFlag::O_NONBLOCK;
        if index + 1 < parts.len() {
            flags |= OFlag::O_DIRECTORY;
        }
        handle = File::from(openat(&handle, Path::new(name), flags, Mode::empty())?);
    }
    if !handle.metadata()?.is_file() {
        return Err(std::io::Error::other("regular file required"));
    }
    let mut bytes = Vec::new();
    handle.take(FILE_LIMIT + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > FILE_LIMIT {
        return Err(std::io::Error::other(
            "portable file exceeds 4 MiB test limit",
        ));
    }
    Ok(bytes)
}

impl Store {
    pub fn new(root: &Path, tool: &str, allowed: Vec<PathBuf>) -> Result<Self> {
        let root = root.canonicalize().map_err(internal)?;
        if !root.is_dir() {
            return Err(invalid("Workspace must be a directory"));
        }
        let mut grants = Vec::new();
        for path in allowed {
            if !path.is_absolute() {
                return Err(invalid("Grant must be an absolute directory"));
            }
            let path = path
                .canonicalize()
                .map_err(|_| invalid("Grant must be an absolute directory"))?;
            if !path.is_dir() {
                return Err(invalid("Grant must be an absolute directory"));
            }
            if !grants.contains(&path) {
                grants.push(path);
            }
        }
        Ok(Self {
            root,
            tool: tool.into(),
            allowed: grants,
        })
    }
    // This test format deliberately accepts only its own complete synthetic records. The
    // host must treat these bytes as opaque; the fake verifies its native semantics here.
    fn validate(&self, document: &Value, portable: bool) -> Result<Value> {
        let owner = document["tool"]["name"]
            .as_str()
            .ok_or_else(|| invalid("Missing tool identity"))?;
        let kind = document["artifact"]["type"]
            .as_str()
            .ok_or_else(|| invalid("Missing artifact type"))?;
        let token = format!("{}:Token", self.tool);
        let memo = format!("{}:Memo", self.tool);
        if kind != "location"
            && (owner != self.tool
                || (kind != token && (portable || (kind != memo && kind != "image"))))
        {
            return Err(invalid("Unsupported portable type or tool owner"));
        }
        let amount = if kind == "location" || kind == "image" {
            0.0
        } else {
            document["record"]["amount"]
                .as_f64()
                .filter(|n| (0.0..=1.0).contains(n))
                .ok_or_else(|| invalid("Invalid synthetic amount"))?
        };
        let expected = fixtures::artifact_variant(owner, kind, amount);
        if *document != expected {
            return Err(invalid(
                "Portable record, reference or preview failed verification",
            ));
        }
        Ok(fixtures::artifact_variant(&self.tool, kind, amount))
    }
    pub fn read(&self, identity: &str) -> Result<Value> {
        let name = file_name(identity).ok_or_else(|| missing(None))?;
        let bytes =
            read_regular(&self.root.join("artifacts").join(name)).map_err(|_| missing(None))?;
        let document: Value = serde_json::from_slice(&bytes).map_err(|_| missing(None))?;
        let value = self.validate(&document, false).map_err(|_| missing(None))?;
        if value["artifact"]["identity"] != identity {
            return Err(missing(None));
        }
        Ok(value)
    }
    pub fn preview(&self, identity: &str) -> Result<Vec<u8>> {
        let document = self.read(identity)?;
        if document["preview"].is_null() {
            return Err(invalid("This artifact has no preview"));
        }
        let kind = document["artifact"]["type"].as_str().unwrap();
        if kind == "image" {
            return Ok(include_bytes!("../ui/tests/fixtures/sample.png").to_vec());
        }
        Ok(fixtures::preview_png(
            kind,
            document["record"]["amount"].as_f64().unwrap_or(0.0),
        ))
    }
    pub fn remember(&self, document: &Value) -> Result<bool> {
        let document = self.validate(document, false)?;
        let identity = document["artifact"]["identity"].as_str().unwrap();
        let name = file_name(identity).ok_or_else(|| invalid("Invalid identity"))?;
        let directory = self.root.join("artifacts");
        let destination = directory.join(name);
        let held = || {
            let existing = self.read(identity).map_err(|_| {
                internal("Stored artifact is corrupt; repair the test workspace before retrying")
            })?;
            if existing != document {
                return Err(internal("Stored identity conflicts with imported record"));
            }
            Ok(true)
        };
        if destination.try_exists().map_err(internal)? {
            return held();
        }
        std::fs::create_dir_all(&directory).map_err(internal)?;
        let mut staged = tempfile::NamedTempFile::new_in(directory).map_err(internal)?;
        staged
            .write_all(&serde_json::to_vec(&document).map_err(internal)?)
            .map_err(internal)?;
        match staged.persist_noclobber(destination) {
            Ok(_) => Ok(false),
            Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => held(),
            Err(error) => Err(internal(error)),
        }
    }
    pub fn read_path(&self, path: &str) -> Result<Vec<u8>> {
        let refusal = || invalid("Input path is not available under the current grants");
        let path = Path::new(path);
        if !path.is_absolute() {
            return Err(refusal());
        }
        let path = path.canonicalize().map_err(|_| refusal())?;
        if !self.allowed.iter().any(|grant| path.starts_with(grant)) {
            return Err(refusal());
        }
        read_regular(&path).map_err(|_| refusal())
    }
    pub fn import(&self, request: Value) -> Result<Value> {
        let request: ImportRequest =
            serde_json::from_value(request).map_err(|_| invalid("Invalid import request"))?;
        let (bytes, expected_hash, declared_media) = match request {
            ImportRequest::File(request) => {
                (self.read_path(&request.file.path)?, request.sha256, None)
            }
            ImportRequest::Inline(request) => (
                inline_bytes(&request.inline.data)?,
                request.sha256,
                Some(request.inline.media_type),
            ),
        };
        if expected_hash.is_some_and(|expected| expected != hash(&bytes)) {
            return Err(invalid("Import digest mismatch"));
        }
        let document: Value = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("Not a complete portable record"))?;
        let document = self.validate(&document, true)?;
        let kind = document["artifact"]["type"].as_str().unwrap();
        if declared_media.is_some_and(|declared| declared != media(&self.tool, kind)) {
            return Err(invalid("Import media type does not match portable record"));
        }
        let already_present = self.remember(&document)?;
        Ok(
            json!({"contract":"0.4.0","tool":{"name":self.tool,"version":"0.1.0"},"artifact":document["artifact"],"already_present":already_present}),
        )
    }
    pub fn export(&self, request: Value) -> Result<Value> {
        let request: ExportRequest =
            serde_json::from_value(request).map_err(|_| invalid("Invalid export request"))?;
        let reference = serde_json::to_value(request.artifact).map_err(internal)?;
        let identity = reference["identity"].as_str().unwrap();
        let document = self.read(identity).map_err(|_| missing(Some("artifact")))?;
        if reference != document["artifact"] {
            return Err(invalid("Reference does not match held artifact"));
        }
        let document = self.validate(&document, true)?;
        let kind = document["artifact"]["type"].as_str().unwrap();
        let bytes = serde_json::to_vec(&document).map_err(internal)?;
        let directory = self.root.join("exports");
        std::fs::create_dir_all(&directory).map_err(internal)?;
        let extension = if kind == "location" { "json" } else { "bundle" };
        let destination = directory.join(format!("{}.{extension}", file_name(identity).unwrap()));
        let mut staged = tempfile::NamedTempFile::new_in(&directory).map_err(internal)?;
        staged.write_all(&bytes).map_err(internal)?;
        staged.persist(&destination).map_err(internal)?;
        Ok(
            json!({"contract":"0.4.0","tool":{"name":self.tool,"version":"0.1.0"},"artifact":document["artifact"],
            "file":{"path":destination,"bytes":bytes.len(),"sha256":hash(&bytes),"media_type":media(&self.tool,kind)}}),
        )
    }
}

pub fn inline_bytes(encoded: &str) -> Result<Vec<u8>> {
    if encoded.len() > INLINE_LIMIT.div_ceil(3) * 4 {
        return Err(invalid("Inline import exceeds 1 MiB"));
    }
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| invalid("Invalid base64"))?;
    if bytes.len() > INLINE_LIMIT {
        return Err(invalid("Inline import exceeds 1 MiB"));
    }
    Ok(bytes)
}
