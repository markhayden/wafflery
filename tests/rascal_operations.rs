//! Opt-in M5.21 acceptance against the coordinator-approved Rascal executable.
#[allow(dead_code)]
#[path = "support/real_peers.rs"]
mod peer;
use serde_json::{Value, json};
use std::{collections::BTreeMap, path::Path, sync::Arc, time::Duration};
use wafflery::{
    library::{
        assets::AssetStore,
        attempts::{Attempt, Submission},
        blobs::BlobStore,
    },
    tools::{bundle_reader, bundles, coordinator::Coordinator, registration::Descriptor},
};
fn descriptor(binary: &Path, tool: &Path) -> Descriptor {
    let exports =
        std::path::PathBuf::from(std::env::var_os("XDG_DATA_HOME").unwrap()).join("rascal/exports");
    std::fs::create_dir_all(&exports).unwrap();
    Descriptor {
        id: "rascal".into(),
        executable: binary.into(),
        args: vec![
            "serve".into(),
            "--workspace".into(),
            tool.join("workspace").display().to_string(),
            "--runs".into(),
            tool.join("runs").display().to_string(),
        ],
        cwd: tool.into(),
        allow_dirs: vec![],
        evidence_dirs: vec![tool.into(), exports],
        config: None,
    }
}
async fn saved(c: &Arc<Coordinator>, id: &str) -> Attempt {
    let result = peer::settled(c, id).await;
    assert_eq!(result.execution, "completed", "{:?}", result.notices);
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let a = c.attempts.get(id).await.unwrap();
            if a.preservation["native"] != "preserving" {
                assert_eq!(a.preservation["contract"], "complete", "{:?}", a.notices);
                return a;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
async fn call(
    c: &Arc<Coordinator>,
    id: &str,
    action: &str,
    params: Value,
    bindings: Value,
    lineage: Value,
) -> Attempt {
    let description = c.description("rascal").await.unwrap();
    let request:Submission=serde_json::from_value(json!({"id":id,"tool":"rascal","action":action,"revision":description.revision,"params":params,"images":{},"bindings":bindings,"lineage":lineage})).unwrap();
    c.submit(request).await.unwrap();
    saved(c, id).await
}
fn output(a: &Attempt) -> Value {
    a.result.as_ref().unwrap()["artifacts"]["out"][0]["artifact"].clone()
}
fn reference(v: &Value) -> Value {
    json!({"kind":"artifact","reference":v})
}
async fn imported(
    c: &Arc<Coordinator>,
    bundle: Arc<bundle_reader::ValidatedBundle>,
    step: usize,
) -> wafflery::tools::bundle_import::Progress {
    let job = c
        .start_bundle_import(bundle, step, "rascal".into())
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let p = job.snapshot();
            if p.phase != "importing" {
                assert_eq!(p.phase, "completed", "{:?}", p.error);
                assert!(
                    p.inputs.iter().all(|i| i.status == "completed"),
                    "{:?}",
                    p.inputs
                );
                return p;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap()
}
#[tokio::test]
#[ignore = "requires approved WAFFLERY_RASCAL_BIN and isolated XDG directories"]
async fn real_operations_keep_native_identity_across_restarts_moves_exports_and_fresh_chain_replay()
{
    let binary = peer::binary("WAFFLERY_RASCAL_BIN");
    let root = tempfile::tempdir().unwrap();
    let tool = root.path().join("source");
    std::fs::create_dir(&tool).unwrap();
    let library = root.path().join("Library");
    let assets = AssetStore::open(&library).unwrap();
    let image_path = root.path().join("image.png");
    std::fs::write(
        &image_path,
        include_bytes!("../ui/tests/fixtures/sample.png"),
    )
    .unwrap();
    let image = assets.import_path(image_path).await.unwrap();
    let c = Arc::new(
        Coordinator::open(assets, vec![descriptor(&binary, &tool)])
            .await
            .unwrap(),
    );
    let (originals,bundle,location,mask)=peer::run(&c,async {
        let d=c.description("rascal").await.unwrap();assert_eq!(d.document["contract"],"0.5.0");assert!(d.document["profiles"].as_array().unwrap().contains(&json!("operations")));assert!(c.session("rascal").await.unwrap()["allowed_dirs"].as_array().unwrap().iter().any(|p|p.as_str().unwrap().ends_with("/execution")));
        let line=call(&c,"line","line",json!({"column":0.5,"width_px":24}),json!({"image":[{"kind":"image","asset":image.id}]}),json!({})).await;
        let location=output(&line);assert_eq!(location["type"],"location");let bytes=c.export_artifact(&line.id,"out",0).await.unwrap();assert_eq!(bytes.media_type,"application/json");let record:Value=serde_json::from_slice(&BlobStore(c.assets.clone()).read(&bytes.sha256).await.unwrap()).unwrap();let mut expected=line.result.as_ref().unwrap()["locations"]["found"][0].clone();expected["image"].as_object_mut().unwrap().remove("pixel_sha256");expected.as_object_mut().unwrap().remove("source");assert_eq!(record,expected,"portable Location JSON retains geometry and file identity; decoded-pixel/source hints are optional");
        let raster=call(&c,"raster","rasterize",json!({}),json!({"image":[{"kind":"image","asset":image.id}],"location":[reference(&location)]}),json!({"location":[{"attempt":line.id,"port":"location","occurrence":0}]})).await;
        let mask=output(&raster);assert_eq!(mask["type"],"rascal:Mask");
        let feather=call(&c,"feather","feather",json!({"sigma_px":6}),json!({"mask":[reference(&mask)]}),json!({"mask":[{"attempt":raster.id,"port":"mask","occurrence":0}]})).await;
        let edited=call(&c,"edited-feather","feather",json!({"sigma_px":3}),json!({"mask":[reference(&mask)]}),json!({"mask":[{"attempt":raster.id,"port":"mask","occurrence":0}]})).await;
        assert_ne!(output(&edited)["identity"],output(&feather)["identity"]);assert_eq!(edited.result.as_ref().unwrap()["invocation"]["arguments"]["mask"],mask);
        let literal=line.result.as_ref().unwrap()["locations"]["found"][0].clone();
        let by_value=call(&c,"by-value","rasterize",json!({}),json!({"image":[{"kind":"image","asset":image.id}],"location":[{"kind":"location","value":literal}]}),json!({})).await;
        assert_eq!(output(&by_value)["identity"],mask["identity"]);
        assert_eq!(c.attempts.history(None,100,None,None).await.unwrap().len(),5,"parameter edits and literal inputs cannot rerun producers");
        let image_reference=line.result.as_ref().unwrap()["artifacts"]["in"][0]["artifact"].clone();
        let mut missing=image_reference.clone();let identity=missing["identity"].as_str().unwrap().to_owned();let absent=format!("sha256:{}","0".repeat(64));missing["identity"]=json!(absent);missing["resource"]=json!(missing["resource"].as_str().unwrap().replace(identity.strip_prefix("sha256:").unwrap(),&"0".repeat(64)));
        let d=c.description("rascal").await.unwrap();let bad=serde_json::from_value(json!({"id":"bad-plane","tool":"rascal","action":"select","revision":d.revision,"params":{},"images":{},"bindings":{"plane":[reference(&image_reference),reference(&missing)]}})).unwrap();
        c.submit(bad).await.unwrap();let refused=peer::settled(&c,"bad-plane").await;assert_eq!(refused.execution,"rejected");let data=&refused.observations["refusal"];assert_eq!(data["reason"],"binding");assert_eq!(data["bindings"][0]["port"],"plane");assert_eq!(data["bindings"][0]["index"],1);assert_eq!(data["bindings"][0]["problem"],"unresolved");assert!(data["run"].is_null());assert!(refused.result.is_none());
        assert_eq!(std::fs::read_dir(tool.join("runs")).unwrap().filter(|entry|entry.as_ref().unwrap().path().join("run.json").is_file()).count(),5,"the tool records only the five explicit actions, with no hidden upstream or refused run");
        c.restart("rascal",true).await.unwrap();assert_eq!(c.inspect_artifact(&raster.id,"out",0).await.unwrap()["artifact"]["identity"],mask["identity"]);
        let originals=vec![line,raster,feather];let built=bundles::build_chain(&c.assets,&originals).await.unwrap();let bytes=std::fs::read(built.file.path()).unwrap();let bundle=Arc::new(bundle_reader::read(bytes).unwrap());assert!(!bundle.manifest()["steps"][1]["artifacts"].as_array().unwrap().is_empty(),"Location JSON travels with the conservative bundle");
        (originals,bundle,location,mask)
    }).await;
    let moved = root.path().join("moved-source");
    std::fs::rename(&tool, &moved).unwrap();
    let c = Arc::new(
        Coordinator::open(
            AssetStore::open(&library).unwrap(),
            vec![descriptor(&binary, &moved)],
        )
        .await
        .unwrap(),
    );
    peer::run(&c, async {
        let got = c.inspect_artifact("raster", "out", 0).await.unwrap();
        assert_eq!(got["artifact"]["identity"], mask["identity"]);
    })
    .await;
    std::fs::remove_dir_all(&moved).unwrap();
    let destination = root.path().join("destination");
    std::fs::create_dir(&destination).unwrap();
    let c = Arc::new(
        Coordinator::open(
            AssetStore::open(&root.path().join("FreshLibrary")).unwrap(),
            vec![descriptor(&binary, &destination)],
        )
        .await
        .unwrap(),
    );
    peer::run(&c,async {
        let single=imported(&c,bundle.clone(),2).await;let imported_mask=&single.inputs[0].binding;assert!(matches!(imported_mask,Some(wafflery::library::attempts::InputBinding::Artifact{reference}) if reference["identity"]==mask["identity"]));assert!(c.attempts.history(None,100,None,None).await.unwrap().is_empty());
        let review=c.start_chain_replay(bundle.clone(),vec!["rascal".into();3]).await.unwrap();
        for (step,original) in originals.iter().enumerate(){c.stage_chain_replay(&review.id,false).await.unwrap();let state=tokio::time::timeout(Duration::from_secs(15),async {loop{let s=c.chain_replay(&review.id).await.unwrap();assert_ne!(s.phase,"blocked","{:?}",s.error);if s.phase=="awaiting_run"{break s;}tokio::time::sleep(Duration::from_millis(10)).await;}}).await.unwrap();let p=state.import.as_ref().unwrap();let mut bindings=BTreeMap::<String,Vec<_>>::new();let mut lineage=BTreeMap::<String,Vec<_>>::new();for input in &p.inputs{bindings.entry(input.name.clone()).or_default().push(input.binding.clone().unwrap());lineage.entry(input.name.clone()).or_default().push(input.source.clone());}
            let id=format!("replay-{step}");let d=c.description("rascal").await.unwrap();let request=serde_json::from_value(json!({"id":id,"tool":"rascal","action":p.action,"revision":d.revision,"params":p.params,"images":{},"bindings":bindings,"lineage":lineage,"source_bundle":{"id":state.bundle,"step":step,"replay":{"id":state.id,"generation":state.generation}}})).unwrap();c.submit(request).await.unwrap();let a=saved(&c,&id).await;assert_eq!(output(&a)["identity"],output(original)["identity"]);assert_eq!(a.result.as_ref().unwrap()["invocation"],original.result.as_ref().unwrap()["invocation"]);
            if step>0 {let port=if step==1{"location"}else{"mask"};assert_eq!(serde_json::to_value(&a.request.lineage[port][0]).unwrap()["attempt"],format!("replay-{}",step-1));}
        }
        assert_eq!(c.chain_replay(&review.id).await.unwrap().phase,"completed");assert_eq!(c.attempts.history(None,100,None,None).await.unwrap().len(),3);
        assert_eq!(c.inspect_artifact("replay-0","out",0).await.unwrap()["artifact"]["identity"],location["identity"]);
    }).await;
    let cli = root.path().join("cli");
    std::fs::create_dir(&cli).unwrap();
    let archive = cli.join("chain.tar");
    std::fs::write(&archive, bundle.archive()).unwrap();
    let result = peer::output(
        tokio::process::Command::new(&binary)
            .args(["call", "--bundle"])
            .arg(&archive)
            .arg("--allow-dir")
            .arg(&cli)
            .arg("--workspace")
            .arg(cli.join("workspace"))
            .arg("--runs")
            .arg(cli.join("runs"))
            .current_dir(&cli),
    )
    .await;
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let rows = String::from_utf8(result.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 3);
    for (row, original) in rows.iter().zip(originals) {
        assert_eq!(row["status"], "completed");
        assert_eq!(
            row["invocation"],
            original.result.as_ref().unwrap()["invocation"]
        );
        assert_eq!(
            row["artifacts"]["out"][0]["artifact"]["identity"],
            output(&original)["identity"]
        );
    }
}
