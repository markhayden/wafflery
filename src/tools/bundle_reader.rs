//! Validate a whole bounded archive in memory before publishing files or contacting tools.
use super::{Error, Result, validator};
use crate::library::bundle_archive::{MAX_MANIFEST, MAX_MEMBER, MAX_MEMBERS, MAX_TOTAL};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
    sync::Arc,
};
#[derive(Clone, Debug, serde::Serialize)]
pub struct InputClaim {
    pub name: String,
    pub index: usize,
    pub input: Input,
}
#[derive(Clone, Debug, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Input {
    Image {
        member: String,
        image: Value,
    },
    /// Host-only prepared replacement. The archive reader never constructs this variant.
    Fresh {
        binding: crate::library::attempts::InputBinding,
        source: crate::library::attempts::BindingSource,
        changed: bool,
    },
    Artifact {
        member: String,
        reference: Value,
    },
    Location {
        value: Value,
    },
    Unresolved {
        reason: String,
    },
}
#[derive(Clone)]
pub struct Member {
    sha256: String,
    bytes: u64,
    range: Range<usize>,
}
#[derive(Clone)]
pub struct ValidatedBundle {
    manifest: Value,
    members: BTreeMap<String, Member>,
    sha256: String,
    bytes: Arc<Vec<u8>>,
}
impl Member {
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
    pub fn bytes(&self) -> u64 {
        self.bytes
    }
}
impl ValidatedBundle {
    pub fn input_claims(&self, index: usize) -> Result<Vec<InputClaim>> {
        let step = self.manifest["steps"]
            .get(index)
            .ok_or(Error::Missing("Bundle step is unavailable"))?;
        claims(step)
    }
    pub fn manifest(&self) -> &Value {
        &self.manifest
    }
    pub fn members(&self) -> &BTreeMap<String, Member> {
        &self.members
    }
    pub fn sha256(&self) -> &str {
        &self.sha256
    }

    pub fn member(&self, name: &str) -> Option<&[u8]> {
        self.members.get(name).map(|m| &self.bytes[m.range.clone()])
    }
    pub fn archive(&self) -> &[u8] {
        &self.bytes
    }
}
fn invalid(message: impl Into<String>) -> Error {
    Error::Invalid(message.into())
}
fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
fn schema(kind: &str, value: &Value) -> Result<()> {
    validator(&json!({"$ref":kind}))?
        .validate(value)
        .map_err(|e| {
            invalid(format!(
                "Invalid invocation bundle at {}",
                e.instance_path()
            ))
        })
}
pub fn read(bytes: Vec<u8>) -> Result<ValidatedBundle> {
    if bytes.len() as u64 > MAX_TOTAL || bytes.len() < 1536 || !bytes.len().is_multiple_of(512) {
        return Err(invalid(
            "Invocation bundle must be a bounded, uncompressed ustar",
        ));
    }
    let mut archive = tar::Archive::new(bytes.as_slice());
    let mut members = BTreeMap::new();
    let mut manifest = None;
    let mut previous = String::new();
    let mut end = 0;
    for (index, item) in archive.entries()?.raw(true).enumerate() {
        if index >= MAX_MEMBERS {
            return Err(invalid("Invocation bundle exceeds 64 members"));
        }
        let item = item?;
        let h = item.header();
        let raw = h.as_bytes();
        if h.as_ustar().is_none()
            || !matches!(h.entry_type().as_byte(), 0 | b'0')
            || raw[157..257].iter().any(|b| *b != 0)
            || raw[265..329].iter().any(|b| *b != 0)
            || raw[100..156].iter().any(|b| *b >= 128)
            || h.mode()? != 0o644
            || h.uid()? != 0
            || h.gid()? != 0
            || h.mtime()? != 0
        {
            return Err(invalid(
                "Invocation bundles require regular ustar files with fixed headers; links and extensions are refused",
            ));
        }
        let name = std::str::from_utf8(&h.path_bytes())
            .map_err(|_| invalid("Bundle member names must be UTF-8"))?
            .to_owned();
        if index == 0 {
            if name != "invocation.json" {
                return Err(invalid("invocation.json must be the first bundle member"));
            }
        } else if !crate::library::bundle_archive::safe_name(&name)
            || (index > 1 && name <= previous)
        {
            return Err(invalid(
                "Bundle members must have unique, sorted, safe lowercase names",
            ));
        }
        let size = item.size();
        let max = if index == 0 { MAX_MANIFEST } else { MAX_MEMBER };
        if size > max {
            return Err(invalid(if index == 0 {
                "Invocation manifest exceeds 8 MiB"
            } else {
                "Invocation bundle member exceeds 64 MiB"
            }));
        }
        let start = usize::try_from(item.raw_file_position())
            .map_err(|_| invalid("Bundle offset overflow"))?;
        let finish = start
            .checked_add(size as usize)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(|| invalid("Truncated invocation bundle member"))?;
        end = finish
            .checked_add((512 - finish % 512) % 512)
            .filter(|n| *n <= bytes.len())
            .ok_or_else(|| invalid("Truncated invocation bundle padding"))?;
        if bytes[finish..end].iter().any(|b| *b != 0) {
            return Err(invalid("Invocation bundle padding must be zero"));
        }
        let data = &bytes[start..finish];
        if index == 0 {
            manifest = Some(serde_json::from_slice::<unique_json::Unique>(data)?.0);
        } else {
            members.insert(
                name.clone(),
                Member {
                    sha256: digest(data),
                    bytes: size,
                    range: start..finish,
                },
            );
        }
        previous = name;
    }
    if bytes.len() - end < 1024 || bytes[end..].iter().any(|b| *b != 0) {
        return Err(invalid(
            "Invocation bundle must end with zero blocks and no trailing payload",
        ));
    }
    let manifest = manifest.ok_or_else(|| invalid("Invocation bundle has no manifest"))?;
    schema("urn:table-manners:core:invocation-bundle", &manifest)?;
    let mut named = BTreeSet::new();
    for step in manifest["steps"].as_array().unwrap() {
        validate_step(step, &members, &bytes, &mut named)?;
    }
    if named.len() != members.len() {
        return Err(invalid("Invocation bundle contains an unclaimed member"));
    }
    let sha256 = digest(&bytes);
    Ok(ValidatedBundle {
        manifest,
        members,
        sha256,
        bytes: Arc::new(bytes),
    })
}
fn reference(reference: &Value) -> Result<()> {
    if reference["resource"]
        != format!(
            "{}://artifact/{}",
            reference["tool"].as_str().unwrap_or(""),
            reference["identity"].as_str().unwrap_or("")
        )
    {
        return Err(invalid(
            "Artifact resource conflicts with its tool or identity",
        ));
    }
    Ok(())
}
fn validate_step<'a>(
    step: &Value,
    members: &'a BTreeMap<String, Member>,
    bytes: &[u8],
    named: &mut BTreeSet<&'a str>,
) -> Result<()> {
    if step["contract"] != "0.4.0"
        || step["result"]["contract"] != step["contract"]
        || step["tool"] != step["result"]["tool"]
        || step["invocation"] != step["result"]["invocation"]
        || step["invocation"]["action"] != step["result"]["action"]
    {
        return Err(invalid(
            "Bundle step conflicts with its original v0.4 result or invocation",
        ));
    }
    for input in step["inputs"].as_array().unwrap() {
        let name = input["member"].as_str().unwrap();
        let (key, member) = members
            .get_key_value(name)
            .ok_or_else(|| invalid("Bundle image member is missing"))?;
        let sha = input["image"]["file_sha256"].as_str().unwrap();
        if member.sha256 != sha || !name.starts_with(&format!("inputs/{sha}.")) {
            return Err(invalid(
                "Bundle image fingerprint or digest filename is inconsistent",
            ));
        }
        let image = &bytes[member.range.clone()];
        let reader = image::ImageReader::new(std::io::Cursor::new(image)).with_guessed_format()?;
        let suffix = match reader.format() {
            Some(image::ImageFormat::Png) => "png",
            Some(image::ImageFormat::Jpeg) => "jpg",
            _ => return Err(invalid("Bundle images must be PNG or JPEG")),
        };
        if !name.ends_with(&format!(".{suffix}")) && !(suffix == "jpg" && name.ends_with(".jpeg")) {
            return Err(invalid(
                "Bundle image extension disagrees with its encoded format",
            ));
        }
        let (width, height) = reader
            .into_dimensions()
            .map_err(|_| invalid("Bundle image header is invalid"))?;
        if input["image"]["width"] != width || input["image"]["height"] != height {
            return Err(invalid("Bundle image dimensions disagree with its header"));
        }
        named.insert(key.as_str());
    }
    for artifact in step["artifacts"].as_array().unwrap() {
        let name = artifact["member"].as_str().unwrap();
        let (key, member) = members
            .get_key_value(name)
            .ok_or_else(|| invalid("Bundle native member is missing"))?;
        reference(&artifact["artifact"])?;
        if artifact["artifact"]["type"] == "image" {
            return Err(invalid(
                "Images travel as original files, not native exports",
            ));
        }
        let identity = artifact["artifact"]["identity"]
            .as_str()
            .unwrap()
            .replace(':', "-");
        if artifact["sha256"] != member.sha256
            || !name.starts_with(&format!("artifacts/{identity}."))
        {
            return Err(invalid(
                "Bundle native export fingerprint or identity filename is inconsistent",
            ));
        }
        named.insert(key.as_str());
    }
    claims(step).map(|_| ())
}
fn name(item: &Value) -> &str {
    item["port"]
        .as_str()
        .or_else(|| item["slot"].as_str())
        .unwrap()
}
fn is_image_argument(value: &Value) -> bool {
    value.get("expect").is_some() || value["type"] == "image" && value.get("resource").is_some()
}
fn image_matches(value: &Value, image: &Value) -> bool {
    if let Some(expect) = value.get("expect") {
        value.as_object().is_some_and(|v| v.len() == 1) && expect == image
    } else {
        value["type"] == "image"
            && value["identity"]
                == format!("sha256:{}", image["file_sha256"].as_str().unwrap_or(""))
    }
}
fn claims(step: &Value) -> Result<Vec<InputClaim>> {
    let mut claims = Vec::new();
    let mut names = BTreeMap::new();
    for field in ["inputs", "artifacts", "unportable", "reference_only"] {
        for item in step[field].as_array().unwrap() {
            let input = name(item);
            let kind = if item.get("slot").is_some() {
                "slot"
            } else {
                "port"
            };
            if names
                .insert(input, kind)
                .is_some_and(|previous| previous != kind)
            {
                return Err(invalid(
                    "Bundle input uses conflicting slot and port declarations",
                ));
            }
        }
    }
    // Input evidence is public contract data, not private argument inference. Removing a
    // file and its manifest row must not erase an input still recorded by the original run.
    for (container, field) in [("artifacts", "port"), ("images", "slot")] {
        for item in step["result"][container]["in"]
            .as_array()
            .into_iter()
            .flatten()
        {
            if let Some(input) = item[field].as_str() {
                names.entry(input).or_insert("recorded");
            }
        }
    }
    let arguments = step["invocation"]["arguments"].as_object().unwrap();
    for (input, _) in names {
        let value = arguments
            .get(input)
            .ok_or_else(|| invalid("Bundle names an input absent from its invocation"))?;
        let values: Vec<_> = if let Some(values) = value.as_array() {
            values.iter().collect()
        } else {
            vec![value]
        };
        let omitted = step["reference_only"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|item| name(item) == input)
            .count();
        if omitted > 0 {
            if omitted != 1
                || values.is_empty()
                || ["inputs", "artifacts", "unportable"].iter().any(|field| {
                    step[*field]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| name(item) == input)
                })
            {
                return Err(invalid(
                    "Omitted bundle input has conflicting included occurrences",
                ));
            }
            let reason = step["reference_only"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| name(item) == input)
                .unwrap()["why"]
                .as_str()
                .unwrap()
                .to_owned();
            for index in 0..values.len() {
                claims.push(InputClaim {
                    name: input.into(),
                    index,
                    input: Input::Unresolved {
                        reason: reason.clone(),
                    },
                });
            }
            continue;
        }
        let mut used = vec![false; values.len()];
        for field in ["unportable", "artifacts", "inputs"] {
            let mut after = 0;
            for item in step[field]
                .as_array()
                .unwrap()
                .iter()
                .filter(|item| name(item) == input)
            {
                let found = values
                    .iter()
                    .enumerate()
                    .skip(after)
                    .find(|(index, value)| {
                        !used[*index]
                            && if field == "inputs" {
                                image_matches(value, &item["image"])
                            } else {
                                ***value == item["artifact"]
                            }
                    });
                let Some((index, value)) = found else {
                    return Err(invalid(
                        "Bundle input occurrence, order or count conflicts with its invocation",
                    ));
                };
                if field != "inputs" || value.get("resource").is_some() {
                    schema("urn:table-manners:operations:artifact-ref", value)?;
                    reference(value)?;
                }
                let source = match field {
                    "inputs" => Input::Image {
                        member: item["member"].as_str().unwrap().into(),
                        image: item["image"].clone(),
                    },
                    "artifacts" => Input::Artifact {
                        member: item["member"].as_str().unwrap().into(),
                        reference: item["artifact"].clone(),
                    },
                    _ => Input::Unresolved {
                        reason: item["reason"].as_str().unwrap().into(),
                    },
                };
                claims.push(InputClaim {
                    name: input.into(),
                    index,
                    input: source,
                });
                used[index] = true;
                after = index + 1;
            }
        }
        for (index, value) in values.iter().enumerate() {
            if !used[index] {
                if value.get("resource").is_some() || is_image_argument(value) {
                    return Err(invalid(
                        "Bundle invocation input has no file or explicit dependency record",
                    ));
                }
                // Literal shared locations stay in the frozen invocation and require no native export.
                schema("urn:table-manners:images:location", value)?;
                claims.push(InputClaim {
                    name: input.into(),
                    index,
                    input: Input::Location {
                        value: (*value).clone(),
                    },
                });
            }
        }
    }
    claims.sort_by(|a, b| (&a.name, a.index).cmp(&(&b.name, b.index)));
    Ok(claims)
}
mod unique_json {
    use serde::{
        Deserialize, Deserializer,
        de::{self, MapAccess, SeqAccess, Visitor},
    };
    use serde_json::{Map, Number, Value};
    use std::fmt;
    pub struct Unique(pub Value);
    impl<'de> Deserialize<'de> for Unique {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            deserializer.deserialize_any(UniqueVisitor)
        }
    }
    struct UniqueVisitor;
    impl<'de> Visitor<'de> for UniqueVisitor {
        type Value = Unique;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("JSON without duplicate keys")
        }
        fn visit_bool<E: de::Error>(self, v: bool) -> Result<Unique, E> {
            Ok(Unique(Value::Bool(v)))
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Unique, E> {
            Ok(Unique(v.into()))
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Unique, E> {
            Ok(Unique(v.into()))
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Unique, E> {
            Ok(Unique(Value::Number(
                Number::from_f64(v).ok_or_else(|| E::custom("Invalid JSON number"))?,
            )))
        }
        fn visit_str<E: de::Error>(self, v: &str) -> Result<Unique, E> {
            Ok(Unique(v.into()))
        }
        fn visit_string<E: de::Error>(self, v: String) -> Result<Unique, E> {
            Ok(Unique(v.into()))
        }
        fn visit_unit<E: de::Error>(self) -> Result<Unique, E> {
            Ok(Unique(Value::Null))
        }
        fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Unique, A::Error> {
            let mut v = vec![];
            while let Some(Unique(value)) = seq.next_element()? {
                v.push(value);
            }
            Ok(Unique(Value::Array(v)))
        }
        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Unique, A::Error> {
            let mut v = Map::new();
            while let Some(key) = map.next_key::<String>()? {
                if v.contains_key(&key) {
                    return Err(de::Error::custom(
                        "Duplicate JSON key in invocation manifest",
                    ));
                }
                let Unique(value) = map.next_value()?;
                v.insert(key, value);
            }
            Ok(Unique(Value::Object(v)))
        }
    }
}
