use super::*;
use wafflery::tools::bundle_reader;
fn tar(entries: Vec<(String, Vec<u8>)>) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (name, bytes) in entries {
        let mut header = tar::Header::new_ustar();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_uid(0);
        header.set_gid(0);
        header.set_mtime(0);
        header.set_username("").unwrap();
        header.set_groupname("").unwrap();
        tar.append_data(&mut header, name, bytes.as_slice())
            .unwrap();
    }
    tar.into_inner().unwrap()
}
pub(super) async fn exported() -> (tempfile::TempDir, AssetStore, Vec<u8>) {
    let (root, assets, attempt, _) = fixture().await;
    let built = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let bytes = std::fs::read(built.file.path()).unwrap();
    (root, assets, bytes)
}
fn rewrite_manifest(bytes: &[u8], change: impl FnOnce(&mut Value)) -> Vec<u8> {
    let mut archive = tar::Archive::new(bytes);
    let mut entries = vec![];
    for entry in archive.entries().unwrap() {
        let mut entry = entry.unwrap();
        let name = entry.path().unwrap().to_str().unwrap().to_owned();
        let mut data = vec![];
        entry.read_to_end(&mut data).unwrap();
        entries.push((name, data));
    }
    let mut manifest: Value = serde_json::from_slice(&entries[0].1).unwrap();
    change(&mut manifest);
    entries[0].1 = serde_json::to_vec(&manifest).unwrap();
    tar(entries)
}
fn header(bytes: &mut [u8], offset: usize, change: impl FnOnce(&mut [u8])) {
    let h = &mut bytes[offset..offset + 512];
    change(h);
    h[148..156].fill(b' ');
    let sum: u32 = h.iter().map(|v| u32::from(*v)).sum();
    let checksum = format!("{sum:06o}\0 ");
    h[148..156].copy_from_slice(checksum.as_bytes());
}
#[tokio::test]
async fn canonical_bundles_are_verified_without_materializing_or_importing_anything() {
    let (_root, assets, bytes) = exported().await;
    let parsed = bundle_reader::read(bytes.clone()).unwrap();
    assert_eq!(parsed.sha256(), hash(&bytes));
    assert_eq!(parsed.manifest()["bundle_version"], 1);
    assert_eq!(parsed.members().len(), 2);
    for item in parsed.manifest()["steps"][0]["artifacts"]
        .as_array()
        .unwrap()
    {
        assert_eq!(
            hash(parsed.member(item["member"].as_str().unwrap()).unwrap()),
            item["sha256"]
        );
    }
    assert_eq!(
        AttemptStore::new(assets.database.clone())
            .history(None, 100, None, None)
            .await
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        std::fs::read_dir(assets.root().join("staging"))
            .unwrap()
            .count(),
        0
    );
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn traversal_links_extensions_duplicates_truncation_and_trailing_payload_are_refused() {
    let (_root, assets, bytes) = exported().await;
    for kind in *b"125xLg7" {
        let mut changed = bytes.clone();
        header(&mut changed, 0, |h| h[156] = kind);
        assert!(bundle_reader::read(changed).is_err(), "entry kind {kind}");
    }
    for name in ["../escape", "/absolute", "inputs/a/b", "INVOCATION.JSON"] {
        let mut changed = bytes.clone();
        header(&mut changed, 0, |h| {
            h[..100].fill(0);
            h[..name.len()].copy_from_slice(name.as_bytes());
        });
        assert!(bundle_reader::read(changed).is_err(), "{name}");
    }
    let mut changed = bytes.clone();
    changed.extend_from_slice(b"hidden payload");
    assert!(bundle_reader::read(changed).is_err());
    for length in [0, 511, bytes.len() - 1] {
        assert!(bundle_reader::read(bytes[..length].to_vec()).is_err());
    }
    let repeated = tar(vec![
        ("invocation.json".into(), b"{}".to_vec()),
        ("invocation.json".into(), b"{}".to_vec()),
    ]);
    assert!(bundle_reader::read(repeated).is_err());
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn schema_identity_member_and_argument_disagreements_fail_before_any_effect() {
    let (_root, assets, bytes) = exported().await;
    for mutation in [
        "version",
        "contract",
        "invocation",
        "result-tool",
        "missing",
        "member-name",
        "artifact-hash",
        "artifact-identity",
        "extra",
        "image-frame",
    ] {
        let changed = rewrite_manifest(&bytes, |manifest| {
            let step = &mut manifest["steps"][0];
            match mutation {
                "version" => manifest["bundle_version"] = json!(2),
                "contract" => step["contract"] = json!("0.3.0"),
                "invocation" => step["invocation"]["arguments"]["amount"] = json!(0.5),
                "result-tool" => step["result"]["tool"]["name"] = json!("other"),
                "missing" => {
                    step["inputs"][0]["member"] = json!(format!("inputs/{}.png", "e".repeat(64)))
                }
                "member-name" => {
                    step["artifacts"][0]["member"] =
                        json!(format!("artifacts/sha256-{}.json", "e".repeat(64)))
                }
                "artifact-hash" => step["artifacts"][0]["sha256"] = json!("e".repeat(64)),
                "artifact-identity" => {
                    step["artifacts"][0]["artifact"]["identity"] =
                        json!(format!("sha256:{}", "e".repeat(64)));
                }
                "extra" => {
                    let extra = step["inputs"][0].clone();
                    step["inputs"].as_array_mut().unwrap().push(extra);
                }
                _ => step["inputs"][0]["image"]["width"] = json!(999),
            }
        });
        assert!(bundle_reader::read(changed).is_err(), "{mutation}");
    }
    let changed = rewrite_manifest(&bytes, |m| {
        m["steps"][0]["artifacts"] = json!([]);
    });
    assert!(
        bundle_reader::read(changed).is_err(),
        "unclaimed native member"
    );
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn deleting_a_recorded_input_and_its_file_cannot_turn_it_into_a_complete_bundle() {
    let (_root, assets, bytes) = exported().await;
    for (field, prefix) in [("inputs", "inputs/"), ("artifacts", "artifacts/")] {
        let changed = rewrite_manifest(&bytes, |m| m["steps"][0][field] = json!([]));
        let mut archive = tar::Archive::new(changed.as_slice());
        let mut entries = vec![];
        for entry in archive.entries().unwrap() {
            let mut entry = entry.unwrap();
            let name = entry.path().unwrap().to_str().unwrap().to_owned();
            let mut data = vec![];
            entry.read_to_end(&mut data).unwrap();
            if !name.starts_with(prefix) {
                entries.push((name, data));
            }
        }
        assert!(
            bundle_reader::read(tar(entries)).is_err(),
            "unaccounted {field}"
        );
    }
    assets.database.shutdown().await.unwrap();
}
#[tokio::test]
async fn raw_member_paths_fixed_headers_padding_and_fingerprints_are_checked() {
    let (_root, assets, bytes) = exported().await;
    let manifest = tar::Header::from_byte_slice(&bytes[..512]).size().unwrap() as usize;
    let next = 512 + manifest.div_ceil(512) * 512;
    for name in [
        "../escape",
        "/absolute",
        "inputs/a/b",
        "inputs/back\\slash",
        "artifacts/.hidden",
    ] {
        let mut changed = bytes.clone();
        header(&mut changed, next, |h| {
            h[..100].fill(0);
            h[..name.len()].copy_from_slice(name.as_bytes());
        });
        assert!(bundle_reader::read(changed).is_err(), "{name}");
    }
    let mut changed = bytes.clone();
    header(&mut changed, next, |h| h[265] = b'x');
    assert!(bundle_reader::read(changed).is_err());
    let mut changed = bytes.clone();
    header(&mut changed, next, |h| {
        h[345] = b'x';
    });
    assert!(bundle_reader::read(changed).is_err());
    let mut changed = bytes.clone();
    header(&mut changed, next, |h| {
        h[100..108].copy_from_slice(b"0000600\0");
    });
    assert!(bundle_reader::read(changed).is_err());
    let mut changed = bytes.clone();
    changed[next + 512] ^= 1;
    assert!(bundle_reader::read(changed).is_err(), "native export bytes");
    let mut changed = bytes.clone();
    changed[512 + manifest] = 1;
    assert!(bundle_reader::read(changed).is_err(), "nonzero padding");
    assets.database.shutdown().await.unwrap();
}
#[test]
fn duplicate_json_keys_and_oversized_headers_are_refused_before_processing_content() {
    let duplicated = tar(vec![(
        "invocation.json".into(),
        br#"{"bundle_version":2,"bundle_version":1,"steps":[]}"#.to_vec(),
    )]);
    let error = bundle_reader::read(duplicated).err().unwrap().to_string();
    assert!(error.contains("Duplicate JSON key"), "{error}");
    for size in [8 * 1024 * 1024 + 1, 64 * 1024 * 1024 + 1] {
        let mut changed = tar(vec![
            ("invocation.json".into(), b"{}".to_vec()),
            ("inputs/a.png".into(), vec![]),
        ]);
        let offset = if size < 64 * 1024 * 1024 { 0 } else { 1024 };
        header(&mut changed, offset, |h| {
            h[124..136].copy_from_slice(format!("{size:011o}\0").as_bytes())
        });
        assert!(
            bundle_reader::read(changed)
                .err()
                .unwrap()
                .to_string()
                .contains("exceeds")
        );
    }
}

#[tokio::test]
async fn safe_ustar_prefix_encoding_is_accepted_but_binary_numeric_headers_are_not() {
    let (_root, assets, bytes) = exported().await;
    let manifest = tar::Header::from_byte_slice(&bytes[..512]).size().unwrap() as usize;
    let next = 512 + manifest.div_ceil(512) * 512;
    let old = String::from_utf8(
        tar::Header::from_byte_slice(&bytes[next..next + 512])
            .path_bytes()
            .into_owned(),
    )
    .unwrap();
    let (directory, base) = old.split_once('/').unwrap();
    let mut prefixed = bytes.clone();
    header(&mut prefixed, next, |h| {
        h[..100].fill(0);
        h[..base.len()].copy_from_slice(base.as_bytes());
        h[345..500].fill(0);
        h[345..345 + directory.len()].copy_from_slice(directory.as_bytes());
    });
    assert!(bundle_reader::read(prefixed).is_ok());
    let mut binary = bytes.clone();
    header(&mut binary, 0, |h| {
        h[108..116].fill(0);
        h[108] = 0x80;
    });
    assert!(
        bundle_reader::read(binary).is_err(),
        "base-256 uid is not a ustar octal header"
    );
    assets.database.shutdown().await.unwrap();
}

#[tokio::test]
async fn explicit_omissions_and_literal_locations_remain_inspectable_without_private_argument_inference()
 {
    let (_root, assets, mut attempt, marker) = fixture().await;
    let partial = bundles::build(
        &assets,
        &attempt,
        &Options {
            omit: BTreeMap::from([("pictures".into(), "Chosen omission".into())]),
        },
    )
    .await
    .unwrap();
    let parsed = bundle_reader::read(std::fs::read(partial.file.path()).unwrap()).unwrap();
    assert_eq!(
        parsed.manifest()["steps"][0]["reference_only"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["markers"] =
        json!([marker["record"]]);
    attempt.result.as_mut().unwrap()["artifacts"]["in"] =
        json!([{"port":"markers","artifact":marker["artifact"],"preview":null}]);
    // A private parameter resembling an input is not a declared contract input occurrence.
    let resembles_image =
        attempt.result.as_ref().unwrap()["invocation"]["arguments"]["pictures"][0].clone();
    attempt.result.as_mut().unwrap()["invocation"]["arguments"]["opaque_parameter"] =
        resembles_image;
    let full = bundles::build(&assets, &attempt, &Options::default())
        .await
        .unwrap();
    let parsed = bundle_reader::read(std::fs::read(full.file.path()).unwrap()).unwrap();
    assert!(
        parsed.manifest()["steps"][0]["artifacts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assets.database.shutdown().await.unwrap();
}
#[test]
fn member_count_is_bounded_before_manifest_interpretation() {
    let mut entries = vec![("invocation.json".into(), b"{}".to_vec())];
    for index in 0..64 {
        entries.push((format!("inputs/{index:064x}.png"), vec![]));
    }
    assert!(
        bundle_reader::read(tar(entries))
            .err()
            .unwrap()
            .to_string()
            .contains("64 members")
    );
}
