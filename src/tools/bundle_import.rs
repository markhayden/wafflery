//! Explicit archive retention. Upload review does not call this publication boundary.
use super::{Error, Result, bundle_reader::ValidatedBundle};
use crate::library::{
    assets::AssetStore,
    bundle_archive::{Bundle, Member},
    bundle_store::{BundleStore, Record},
};
use std::io::Write;

pub async fn retain(assets: &AssetStore, bundle: &ValidatedBundle) -> Result<Record> {
    let root = assets.root().join("staging");
    let validated = bundle.clone();
    let built = tokio::task::spawn_blocking(move || -> Result<Bundle> {
        let mut file = tempfile::NamedTempFile::new_in(root)?;
        file.write_all(validated.archive())?;
        Ok(Bundle {
            file,
            sha256: validated.sha256().into(),
            size: validated.archive().len() as u64,
            manifest: validated.manifest().clone(),
            members: validated
                .members()
                .iter()
                .map(|(name, member)| Member {
                    name: name.clone(),
                    sha256: member.sha256().into(),
                    bytes: member.bytes(),
                })
                .collect(),
            workspace_dependent: validated.manifest()["steps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|step| {
                    !step["reference_only"].as_array().unwrap().is_empty()
                        || !step["unportable"].as_array().unwrap().is_empty()
                }),
            input_assets: vec![],
            native_links: vec![],
        })
    })
    .await
    .map_err(|_| Error::Invalid("Bundle retention worker stopped".into()))??;
    Ok(BundleStore(assets.clone()).save_imported(built).await?)
}

use crate::library::attempts::InputBinding;
use serde::Serialize;
use serde_json::Value;
use std::sync::RwLock;
#[derive(Clone, Debug, Serialize)]
pub struct ImportedInput {
    pub name: String,
    pub index: usize,
    pub status: String,
    pub binding: Option<InputBinding>,
    pub reason: Option<String>,
    pub source: Option<crate::library::attempts::BindingSource>,
    pub changed: bool,
}
#[derive(Clone, Debug, Serialize)]
pub struct Progress {
    pub replay: Option<crate::library::attempts::ReplayContext>,
    pub phase: String,
    pub bundle: String,
    pub step: usize,
    pub tool: String,
    pub action: String,
    pub params: Value,
    pub inputs: Vec<ImportedInput>,
    pub error: Option<String>,
}
pub struct ImportJob(pub(crate) RwLock<Progress>);
impl ImportJob {
    pub fn snapshot(&self) -> Progress {
        self.0.read().unwrap_or_else(|e| e.into_inner()).clone()
    }
    pub(crate) fn update(&self, update: impl FnOnce(&mut Progress)) {
        update(&mut self.0.write().unwrap_or_else(|e| e.into_inner()));
    }
}
