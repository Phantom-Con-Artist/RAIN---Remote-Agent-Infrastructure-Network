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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coworker: Option<CoworkerEnvelope>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    #[serde(default)]
    pub native_settings: NativeChatSettings,
    #[serde(default)]
    pub native_terminal_id: Option<uuid::Uuid>,
    pub id: uuid::Uuid,
    pub title: String,
    pub is_group: bool,
    pub is_active: bool,
    pub participant_ids: Vec<String>,
    pub last_message: Option<String>,
    pub last_message_time: Option<DateTime<Utc>>,
    pub unread_count: usize,
    #[serde(default)]
    pub collaboration: Option<CollaborationState>,
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

/// RAIN Coworker Protocol v1. Only complete, scoped chat replies are routable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoworkerKind {
    Discuss,
    Plan,
    Handoff,
    HumanInput,
    Complete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoworkerEnvelope {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flair: Option<MessageFlair>,
    pub rain_coworker: u8,
    pub run_id: uuid::Uuid,
    pub kind: CoworkerKind,
    pub text: String,
    #[serde(default)]
    pub to: Vec<String>,
    /// Relative paths in the sending agent's own workspace, never the hub's.
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollaborationState {
    #[serde(default)]
    pub mode: ConversationMode,
    #[serde(default)]
    pub interrupted_ids: Vec<String>,
    #[serde(default)]
    pub addressed_ids: Vec<String>,
    pub enabled: bool,
    pub max_turns: u32,
    pub remaining_turns: u32,
    pub run_id: Option<uuid::Uuid>,
    pub waiting_for_human: bool,
    pub human_request: Option<String>,
    #[serde(default)]
    pub human_goal: Option<String>,
    #[serde(default)]
    pub human_direction: Option<String>,
}
impl Default for CollaborationState {
    fn default() -> Self {
        Self {
            mode: ConversationMode::Casual,
            interrupted_ids: vec![],
            addressed_ids: vec![],
            enabled: true,
            max_turns: 0,
            remaining_turns: 0,
            run_id: None,
            waiting_for_human: false,
            human_request: None,
            human_goal: None,
            human_direction: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CollaborationRequest {
    pub enabled: bool,
    pub max_turns: u32,
}

/// Membership-scoped group history. A deleted group is remembered but never routable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMemory {
    pub conversation: Conversation,
    pub messages: Vec<ChatMessage>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NativeChatSettings {
    pub model: Option<String>,
    pub effort: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageFlair {
    Question,
    Idea,
    Review,
    Decision,
    Blocker,
    Casual,
    Update,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationMode {
    #[default]
    Casual,
    Goal,
    Evidence,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModeRequest {
    pub mode: ConversationMode,
    pub context: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentTrace {
    pub id: uuid::Uuid,
    pub dispatch_id: uuid::Uuid,
    pub conversation_id: uuid::Uuid,
    pub agent_id: uuid::Uuid,
    pub step: String,
    pub timestamp: DateTime<Utc>,
}
