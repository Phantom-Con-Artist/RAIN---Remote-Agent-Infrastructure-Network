use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PlatformKind {
    Linux,
    Windows,
    MacOS,
    Android,
    IOS,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_bytes: u64,
    pub available_bytes: u64,
    pub file_system: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub pid: u32,
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStatus {
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub kernel_version: String,
    pub platform: PlatformKind,
    pub uptime_seconds: u64,
    pub cpu_count: usize,
    pub cpu_usage_percent: f32,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    pub memory_free_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub disks: Vec<DiskInfo>,
    pub process_count: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
