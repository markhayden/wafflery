//! The `checks` scenario: an illustrative action that answers with a verdict (table-manners 0.5
//! draft, 2.4), with an optional credential (3.1) and labelled image outs for each try (10.2).
//! Nothing is judged: the caller's `answer` and `tries` script what the sample check says, so
//! browser tests can show a pass, a fail with places and a three-try run without a real tool.
use super::fixtures;
use serde_json::{Value, json};
use std::{fs, path::Path};

pub const ACTION: &str = "checked_band";
pub const KEY: &str = "EXAMPLE_CHECK_KEY";

/// The key's state, from `<root>/check-key-state` (`present` unless the test says otherwise).
fn key_state(root: &Path) -> String {
    let state = fs::read_to_string(root.join("check-key-state")).unwrap_or_else(|_| "present".into());
    match state.trim() {
        "present" | "valid" | "invalid" | "missing" | "unknown" => state.trim().into(),
        _ => "unknown".into(),
    }
}

fn keyed(root: &Path) -> bool {
    matches!(key_state(root).as_str(), "present" | "valid")
}

/// The checking action, added beside `mark_band` in the `checks` scenario.
pub fn action(root: &Path, mark_band: &Value) -> Value {
    let mut action = mark_band.clone();
    action["name"] = json!(ACTION);
    action["title"] = json!("Add a checked band");
    action["purpose"] = json!("Adds the test band, then asks a scripted sample check whether the band still shows. Nothing is judged.");
    action["answers"] = json!("verdict");
    action["params_schema"] = json!({"type":"object","additionalProperties":false,"properties":{
        "label": {"type":"string","title":"Label","description":"A note to identify this experiment.","minLength":1,"maxLength":120},
        "check": {"type":"boolean","title":"Check the result","description":"Ask the check whether the band still shows, and try again when it does.","default":true},
        "answer": {"type":"string","title":"Sample check answers","description":"What the scripted check says about the last try. Earlier tries are answered fail.","enum":["pass","fail","unsure"],"default":"pass"},
        "tries": {"type":"integer","title":"Tries","description":"How many tries the sample makes when it checks.","minimum":1,"maximum":3,"default":1}
    },"required":["label"]});
    action["outputs"] = json!({"images":["result","preview","mask"],"locations":[]});
    action["readiness"] = json!({"state":"ready","requirements":[{
        "kind":"credential","name":KEY,"state":key_state(root),"optional":true,"for":"checking the result",
        "setup":"Set EXAMPLE_CHECK_KEY where the sample tool starts, then refresh. Without it the band is added unchecked."
    }]});
    action
}

/// One try's images: the result (the band at a strength that grows with the try), a preview
/// (the band in amber) and a mask (the band's rows, wider each try).
fn try_images(
    source: &Value,
    roots: &[std::path::PathBuf],
    directory: &Path,
    k: u64,
) -> Result<[(String, Value, Vec<u8>); 3], Box<dyn std::error::Error>> {
    let strength = 0.35 + 0.2 * k as f64;
    let (_, result, result_bytes) =
        super::scenarios::mark(&json!({"source":source,"strength":strength,"tone":"blue"}), roots)?;
    let (_, preview, preview_bytes) =
        super::scenarios::mark(&json!({"source":source,"strength":0.8,"tone":"amber"}), roots)?;
    let (width, height) = (result["width"].as_u64().unwrap() as u32, result["height"].as_u64().unwrap() as u32);
    let half = (height / 12).max(1) * k as u32;
    let middle = height / 2;
    let mask = image::GrayImage::from_fn(width, height, |_, y| {
        image::Luma([if y + half >= middle && y <= middle + half { 255 } else { 0 }])
    });
    let mut bytes = std::io::Cursor::new(Vec::new());
    mask.write_to(&mut bytes, image::ImageFormat::Png)?;
    let mask_bytes = bytes.into_inner();
    let mask = fixtures::reference(&mask_bytes, width, height);
    let mut outs = Vec::new();
    for (name, reference, bytes) in [
        ("result", result, result_bytes),
        ("preview", preview, preview_bytes),
        ("mask", mask, mask_bytes),
    ] {
        let file = format!("try-{k}-{name}.png");
        fs::write(directory.join(&file), &bytes)?;
        outs.push((file, reference, bytes));
    }
    Ok(outs.try_into().unwrap())
}

/// Fills a completed run of the checking action: its image outs, its verdict and notes.
pub fn complete(
    run: &mut Value,
    params: &Value,
    root: &Path,
    directory: &Path,
    roots: &[std::path::PathBuf],
    id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let checked = params["check"] != false && keyed(root);
    let tries = if checked { params["tries"].as_u64().unwrap_or(1).clamp(1, 3) } else { 1 };
    let answer = params["answer"].as_str().unwrap_or("pass");
    let locator = params["return_images"].as_str().unwrap_or("path").to_owned();
    let out = |role: String, label: Option<String>, file: &str, reference: &Value, bytes: &[u8]| {
        let mut image = json!({"role":role,"image":reference});
        if let Some(label) = label {
            image["label"] = json!(label);
        }
        match locator.as_str() {
            "inline" => {
                use base64::Engine;
                image["inline"] = json!({"media_type":"image/png","data":base64::engine::general_purpose::STANDARD.encode(bytes)})
            }
            "resource" => image["resource"] = json!(format!("{}://run/{id}/{file}", fixtures::TOOL)),
            _ => image["path"] = json!(directory.join(file)),
        }
        image
    };
    let mut images = Vec::new();
    for k in 1..=tries {
        images.push(try_images(&params["source"], roots, directory, k)?);
    }
    let label = |k: u64, name: &str| (tries > 1).then(|| format!("Try {k} · {name}"));
    let mut outs = Vec::new();
    // The kept try (the last) leads under the plain roles, then every other try in order.
    for (name, (file, reference, bytes)) in ["result", "preview", "mask"].iter().zip(&images[tries as usize - 1]) {
        outs.push(out((*name).into(), label(tries, name), file, reference, bytes));
    }
    for k in 1..tries {
        for (name, (file, reference, bytes)) in ["result", "preview", "mask"].iter().zip(&images[k as usize - 1]) {
            outs.push(out(format!("{name}_try_{k}"), label(k, name), file, reference, bytes));
        }
    }
    run["images"]["out"] = json!(outs);
    run["action"] = json!(ACTION);
    let kept = &images[tries as usize - 1][0].1;
    let mut notes = vec![];
    if !checked {
        notes.push(if params["check"] == false {
            "Not checked: the check was turned off for this call.".to_owned()
        } else {
            format!("Not checked: the sample check needs {KEY}.")
        });
    } else {
        let places: Vec<Value> = if answer == "pass" {
            vec![]
        } else {
            vec![json!({"image":kept,"kind":"bbox","parts":[{"bbox":[0.3,0.38,0.62,0.62]}],
                "straddles_wrap":false,"label":"the band"})]
        };
        notes.push(format!("Checked: up to {tries} tries within the spend limits."));
        let mut summary = format!("Kept try {tries} of {tries}.");
        if answer != "pass" {
            summary.push_str(&format!(" The check still sees the band in 1 place ({answer})."));
        }
        notes.push(summary);
        run["verdict"] = json!({"value":answer,"where":places});
        run["invocation"]["requirements"] = json!([KEY]);
        run["body"]["selected_try"] = json!(tries);
    }
    notes.push("Illustrative offline test output; not an image repair or quality evaluation.".into());
    run["notes"] = json!(notes);
    Ok(())
}
