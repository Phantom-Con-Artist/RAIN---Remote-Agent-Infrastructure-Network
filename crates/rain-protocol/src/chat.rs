use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SenderType {
    Human,
    Agent,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    Online,
    Thinking,
    Working,
    Responding,
    Waiting,
    Completed,
    Stopped,
    Disconnected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageDeliveryStatus {
    Sent,
    Delivered,
    Read,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatAttachment {
    pub id: String,
    pub name: String,
    pub mime_type: String, // e.g. "image/png", "text/plain", "application/json"
    pub size_bytes: u64,
    pub preview_content: Option<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    pub data_base64: Option<String>, // Base64 image or file data for rendering/downloading
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: uuid::Uuid,
    pub conversation_id: uuid::Uuid,
    pub sender_id: String,
    pub sender_name: String,
    pub sender_type: SenderType,
    pub text: String,
    pub status: MessageDeliveryStatus,
    pub attachments: Vec<ChatAttachment>,
    pub job_delegation: Option<crate::remote::JobDelegation>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: uuid::Uuid,
    pub title: String,
    pub is_group: bool,
    pub is_active: bool,
    pub participant_ids: Vec<String>,
    pub last_message: Option<String>,
    pub last_message_time: Option<DateTime<Utc>>,
    pub unread_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub id: String,
    pub name: String,
    pub provider: String, // "codex", "claude_code", "custom"
    pub machine_name: String,
    pub state: SessionState,
    pub cwd: String,
    pub subagents: Vec<String>,
    #[serde(default)]
    pub interactive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    pub text: String,
    pub target_session_id: Option<String>, // Direct to specific agent or whole group
    pub attachments: Option<Vec<ChatAttachment>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateConversationRequest {
    pub title: String,
    pub is_group: bool,
    pub participant_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectChatRequest {
    pub agent_id: String,
}
