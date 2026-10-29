use crate::system::PlatformKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceRegistration {
    pub device_id: String,
    pub device_name: String,
    pub platform: PlatformKind,
    pub enrolled_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub is_revoked: bool,
    pub is_approved: bool,
    pub ip_address: Option<String>,
    pub role: String, // "admin", "collaborator", "device"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInitRequest {
    pub client_device_id: String,
    pub client_device_name: String,
    pub client_platform: PlatformKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingInitResponse {
    pub pairing_code: String,
    pub host_name: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingTicketResponse {
    pub pairing_code: String,
    pub deep_link: String,
    pub qr_svg: String,
    pub host_name: String,
    pub host_listen_addr: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingConfirmRequest {
    pub client_device_id: String,
    pub pairing_code: String,
    pub client_device_name: Option<String>,
    pub client_platform: Option<PlatformKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingConfirmResponse {
    pub token: String,
    pub approved: bool,
    pub host_name: String,
    pub device_id: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailOtpRequest {
    pub email: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailOtpVerifyRequest {
    pub email: String,
    pub otp_code: String,
    pub client_device_id: String,
    pub client_device_name: String,
    pub client_platform: Option<PlatformKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubSpokeNode {
    pub device_id: String,
    pub device_name: String,
    pub platform: PlatformKind,
    pub is_online: bool,
    pub enrolled_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: chrono::DateTime<chrono::Utc>,
    pub ip_address: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubTopologyStatus {
    pub hub_name: String,
    pub hub_node_id: String,
    pub hub_listen_addr: String,
    pub is_hub_online: bool,
    pub uptime_seconds: u64,
    pub active_spokes_count: usize,
    pub spokes: Vec<HubSpokeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStatusResponse {
    pub authenticated: bool,
    pub device: Option<DeviceRegistration>,
    pub hub_name: String,
}
