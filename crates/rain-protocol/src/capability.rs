use crate::system::PlatformKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub name: String,
    pub description: String,
    pub category: String,
    pub risk_level: RiskLevel,
    pub supported_platforms: Vec<PlatformKind>,
    pub requires_confirmation: bool,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityInvocation {
    pub id: uuid::Uuid,
    pub capability_name: String,
    pub parameters: serde_json::Value,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityResult {
    pub invocation_id: uuid::Uuid,
    pub success: bool,
    pub data: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}
