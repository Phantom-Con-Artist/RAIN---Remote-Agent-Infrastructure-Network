use rain_core::{AgentSupervisor, CapabilityRegistry, ChatStore, InMemoryStore, RemoteManager};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) history_path: std::path::PathBuf,
    pub(crate) registry: Arc<CapabilityRegistry>,
    pub(crate) supervisor: Arc<AgentSupervisor>,
    pub(crate) store: Arc<InMemoryStore>,
    pub(crate) chat: Arc<ChatStore>,
    pub(crate) remote: Arc<RemoteManager>,
}
