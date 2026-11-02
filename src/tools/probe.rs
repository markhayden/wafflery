//! Probe configuration without credentials, close it, then start the only user-visible session.
use super::{
    client::{Declaration, ToolClient},
    environment,
    registration::Descriptor,
};
use std::sync::Arc;

pub(super) struct Started {
    pub client: Option<Arc<ToolClient>>,
    pub declaration: Option<Declaration>,
    pub error: Option<String>,
}
impl Started {
    fn failed(client: Option<ToolClient>, error: String) -> Self {
        Self {
            client: client.map(Arc::new),
            declaration: None,
            error: Some(error),
        }
    }
}

pub(super) async fn start(descriptor: Descriptor) -> Started {
    let mut probe = match ToolClient::spawn(descriptor.clone()) {
        Ok(client) => client,
        Err(error) => return Started::failed(None, error.to_string()),
    };
    let discovered = match probe.initialize().await {
        Ok(()) => probe.describe().await,
        Err(error) => Err(error),
    };
    match probe.close().await {
        Ok(true) => (),
        Ok(false) => return Started::failed(Some(probe), "Configuration probe did not stop. No serving session was started; stop the owned probe before trying again.".into()),
        Err(error) => return Started::failed(Some(probe), format!("Configuration probe cleanup failed; no serving session was started: {error}")),
    }
    let discovered = match discovered {
        Ok(declaration) => declaration,
        Err(error) => return Started::failed(None, format!("Configuration probe failed: {error}")),
    };
    let requirements = environment::requirements(&discovered.document);
    let mut client = match ToolClient::spawn_with_requirements(descriptor, Some(&requirements)) {
        Ok(client) => client,
        Err(error) => return Started::failed(None, error.to_string()),
    };
    let described = match client.initialize().await {
        Ok(()) => client.describe().await,
        Err(error) => Err(error),
    };
    match described {
        Ok(declaration) => {
            let error = client.setup_notice(&declaration);
            Started {
                client: Some(Arc::new(client)),
                declaration: Some(declaration),
                error,
            }
        }
        Err(error) => Started::failed(Some(client), error.to_string()),
    }
}
