use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use crate::PlatformKind;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size_bytes: u64,
    pub modified_time: Option<DateTime<Utc>>,
    pub is_editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileResponse {
    pub path: String,
    pub content: String,
    pub size_bytes: u64,
    pub is_binary: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteFileRequest {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledApp {
    pub id: String,
    pub name: String,
    pub executable: String,
    pub icon_name: String,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchAppRequest {
    pub app_id: String,
    pub custom_args: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExecuteRequest {
    pub command: String,
    pub is_sudo: bool,
    pub sudo_password: Option<String>,
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminalExecuteResponse {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryStatus {
    pub percent: u8,
    pub is_charging: bool,
    pub is_ac_connected: bool,
    pub power_source: String, // "Battery" | "AC Power"
    pub time_remaining_minutes: Option<u32>,
    pub is_critical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineOverview {
    pub id: String,
    pub name: String,
    pub platform: PlatformKind,
    pub is_online: bool,
    pub ip_address: String,
    pub cpu_usage: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub active_agents_count: usize,
    pub uptime_seconds: u64,
    pub battery: Option<BatteryStatus>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    DirectUsb,
    LanMdns,
    WireguardP2p,
    EncryptedRelay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMeshStatus {
    pub current_transport: TransportKind,
    pub node_id: String,
    pub latency_ms: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub is_e2ee: bool,
    pub available_transports: Vec<TransportKind>,
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Proposed,
    Accepted,
    Transferring,
    Running,
    RequiresApproval,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDelegation {
    pub id: uuid::Uuid,
    pub source_agent_id: String,
    pub source_machine: String,
    pub target_agent_id: String,
    pub target_machine: String,
    pub task_instruction: String,
    pub required_resources: Vec<String>,
    pub status: JobStatus,
    pub progress_percent: u8,
    pub shared_artifacts: Vec<crate::ChatAttachment>,
    pub requires_sudo: bool,
    pub sudo_command: Option<String>,
    pub is_approved: bool,
    pub result_summary: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineResourceCatalog {
    pub machine_id: String,
    pub machine_name: String,
    pub platform: crate::PlatformKind,
    pub cpu_cores: usize,
    pub ram_total_gb: f32,
    pub ram_free_gb: f32,
    pub gpu_model: Option<String>,
    pub network_speed_mbps: u32,
    pub tags: Vec<String>,
    pub battery: Option<BatteryStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactTransferRequest {
    pub transfer_id: uuid::Uuid,
    pub source_machine: String,
    pub target_machine: String,
    pub file_name: String,
    pub target_path: String,
    pub size_bytes: u64,
    pub content_base64: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobApprovalRequest {
    pub approved: bool,
    pub sudo_password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerFailoverRequest {
    pub source_machine_id: String,
    pub target_machine_id: String,
    pub battery_percent: u8,
    pub git_repo: String,
    pub git_branch: String,
    pub git_commit: String,
    pub task_summary: String,
}
