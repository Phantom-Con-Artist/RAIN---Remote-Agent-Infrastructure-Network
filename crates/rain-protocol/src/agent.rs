use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Starting,
    Running,
    Completed,
    Failed,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcess {
    pub id: uuid::Uuid,
    pub name: String,
    pub provider: String,
    #[serde(default)]
    pub machine: Option<String>,
    #[serde(default)]
    pub interactive: bool,
    pub pid: Option<u32>,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: String,
    pub status: AgentStatus,
    pub exit_code: Option<i32>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub finished_at: Option<chrono::DateTime<chrono::Utc>>,
    pub recent_logs: Vec<String>,
    #[serde(default)]
    pub output_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLaunchRequest {
    pub name: String,
    pub provider: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    #[serde(default)]
    pub interactive: bool,
    #[serde(default)]
    pub machine: Option<String>,
    pub env_vars: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInputRequest {
    pub input: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStopRequest {
    pub force: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    Output {
        agent_id: uuid::Uuid,
        stream: String, // "stdout" or "stderr"
        line: String,
    },
    StatusChanged {
        agent_id: uuid::Uuid,
        status: AgentStatus,
        exit_code: Option<i32>,
    },
}
