use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactDescriptor {
    pub id: uuid::Uuid,
    pub name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub source_machine: Option<String>,
    pub original_path: String,
    pub snapshot_path: String,
    pub created_at: DateTime<Utc>,
}
impl ArtifactDescriptor {
    pub fn attachment(&self) -> crate::ChatAttachment {
        crate::ChatAttachment {
            id: self.id.to_string(),
            name: self.name.clone(),
            mime_type: self.mime_type.clone(),
            size_bytes: self.size_bytes,
            sha256: Some(self.sha256.clone()),
            data_base64: None,
            preview_content: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferReceipt {
    pub id: uuid::Uuid,
    pub artifact_id: uuid::Uuid,
    pub source_machine: Option<String>,
    pub target_machine: Option<String>,
    pub route: String,
    pub status: String,
    pub bytes: u64,
    #[serde(default)]
    pub transferred_bytes: u64,
    pub sha256: String,
    pub path: Option<String>,
    pub fallback_reason: Option<String>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkNode {
    pub id: String,
    pub name: String,
    pub role: String,
    pub parent_id: Option<String>,
    pub endpoint: Option<String>,
    #[serde(default)]
    pub revoked: bool,
    pub online: bool,
    pub last_seen: DateTime<Utc>,
    pub cpu_cores: usize,
    pub ram_total_bytes: u64,
    pub ram_available_bytes: u64,
    pub active_agents: usize,
    #[serde(default)]
    pub children: Vec<NetworkNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatDispatch {
    pub id: uuid::Uuid,
    pub conversation_id: uuid::Uuid,
    pub message_id: uuid::Uuid,
    pub run_id: Option<uuid::Uuid>,
    pub source_id: String,
    pub source_name: String,
    pub target_id: String,
    pub target_name: String,
    pub target_machine: Option<String>,
    pub status: String,
    pub queued_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}
