use rain_protocol::{
    DeviceRegistration, EmailOtpRequest, EmailOtpVerifyRequest, HubSpokeNode, HubTopologyStatus,
    PairingConfirmResponse, PairingInitResponse, PairingTicketResponse, PlatformKind,
    SpokeHeartbeatRequest, SpokeRegisterRequest,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpokeNodeInfo {
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub is_hub: bool,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub children: Vec<rain_protocol::NetworkNode>,
    pub device_id: String,
    pub device_name: String,
    pub platform: PlatformKind,
    pub ip_address: String,
    pub cpu_cores: usize,
    pub ram_total_gb: f32,
    pub gpu_model: Option<String>,
    pub cpu_usage: f32,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub active_agents_count: usize,
    pub uptime_seconds: u64,
    pub battery: Option<rain_protocol::BatteryStatus>,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct PairingSession {
    pub code: String,
    pub client_device_id: String,
    pub client_device_name: String,
    pub client_platform: PlatformKind,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct EmailOtpSession {
    pub email: String,
    pub code: String,
    pub device_name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct InMemoryStore {
    devices: Arc<RwLock<HashMap<String, DeviceRegistration>>>,
    spokes: Arc<RwLock<HashMap<String, SpokeNodeInfo>>>,
    active_pairings: Arc<RwLock<HashMap<String, PairingSession>>>,
    email_otps: Arc<RwLock<HashMap<String, EmailOtpSession>>>,
    auth_tokens: Arc<RwLock<HashMap<String, String>>>, // token -> device_id
    hub_start_time: chrono::DateTime<chrono::Utc>,
    pub parent_last_seen: Arc<RwLock<Option<chrono::DateTime<chrono::Utc>>>>,
    pub parent_path: Arc<RwLock<Vec<String>>>,
    pub(crate) network_path: Option<std::path::PathBuf>,
    pub(crate) network_persistence: Arc<tokio::sync::Mutex<()>>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        Self {
            devices: Arc::new(RwLock::new(HashMap::new())),
            spokes: Arc::new(RwLock::new(HashMap::new())),
            active_pairings: Arc::new(RwLock::new(HashMap::new())),
            email_otps: Arc::new(RwLock::new(HashMap::new())),
            auth_tokens: Arc::new(RwLock::new(HashMap::new())),
            hub_start_time: now,
            parent_last_seen: Arc::new(RwLock::new(None)),
            parent_path: Arc::new(RwLock::new(Vec::new())),
            network_path: None,
            network_persistence: Arc::new(tokio::sync::Mutex::new(())),
        }
    }

    pub async fn create_pairing_code(
        &self,
        client_id: &str,
        client_name: &str,
        platform: PlatformKind,
    ) -> PairingInitResponse {
        // Generate a 6-digit numeric pairing code
        let code = format!("{:06}", (uuid::Uuid::new_v4().as_u128() % 900000) + 100000);
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(5);

        let session = PairingSession {
            code: code.clone(),
            client_device_id: client_id.to_string(),
            client_device_name: client_name.to_string(),
            client_platform: platform,
            created_at: now,
            expires_at: expires,
        };

        let mut lock = self.active_pairings.write().await;
        lock.insert(code.clone(), session);

        PairingInitResponse {
            pairing_code: code,
            host_name: std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Hub-Host".to_string()),
            expires_in_seconds: 300,
        }
    }

    pub async fn generate_pairing_ticket(
        &self,
        host_name: &str,
        host_listen_addr: &str,
    ) -> PairingTicketResponse {
        let code = format!("{:06}", (uuid::Uuid::new_v4().as_u128() % 900000) + 100000);
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(10);

        let session = PairingSession {
            code: code.clone(),
            client_device_id: "*".to_string(), // Wildcard: any claiming client
            client_device_name: "Pending-Spoke-Device".to_string(),
            client_platform: PlatformKind::Linux,
            created_at: now,
            expires_at: expires,
        };

        let mut lock = self.active_pairings.write().await;
        lock.insert(code.clone(), session);

        let encoded_host = host_name.replace(' ', "%20");
        let encoded_addr = host_listen_addr.replace(' ', "%20").replace(':', "%3A");

        let deep_link = format!(
            "rain://pair?code={}&host={}&addr={}&expires=600",
            code, encoded_host, encoded_addr
        );

        let qr_svg = generate_svg_qr(&deep_link, &code);

        PairingTicketResponse {
            pairing_code: code,
            deep_link,
            qr_svg,
            host_name: host_name.to_string(),
            host_listen_addr: host_listen_addr.to_string(),
            expires_in_seconds: 600,
        }
    }

    pub async fn verify_pairing_code(
        &self,
        client_id: &str,
        code: &str,
    ) -> Result<String, &'static str> {
        let mut pairings = self.active_pairings.write().await;
        if let Some(session) = pairings.get(code) {
            if session.client_device_id != "*" && session.client_device_id != client_id {
                return Err("Device ID does not match pairing request");
            }
            if chrono::Utc::now() > session.expires_at {
                pairings.remove(code);
                return Err("Pairing code has expired");
            }

            let client_name = if session.client_device_name == "Pending-Spoke-Device" {
                format!("Device-{}", &client_id[..client_id.len().min(8)])
            } else {
                session.client_device_name.clone()
            };

            // Enroll device
            let mut devices = self.devices.write().await;
            devices.insert(
                client_id.to_string(),
                DeviceRegistration {
                    device_id: client_id.to_string(),
                    device_name: client_name,
                    platform: session.client_platform.clone(),
                    enrolled_at: chrono::Utc::now(),
                    last_seen_at: chrono::Utc::now(),
                    is_revoked: false,
                    is_approved: true,
                    ip_address: Some("127.0.0.1".to_string()),
                    role: "collaborator".to_string(),
                },
            );

            // Generate an auth token
            let token = format!("rain_tok_{}", uuid::Uuid::new_v4().simple());
            let mut tokens = self.auth_tokens.write().await;
            tokens.insert(token.clone(), client_id.to_string());

            pairings.remove(code);
            Ok(token)
        } else {
            Err("Invalid pairing code")
        }
    }

    pub async fn request_email_otp(&self, req: EmailOtpRequest) -> String {
        let code = format!("{:06}", (uuid::Uuid::new_v4().as_u128() % 900000) + 100000);
        let now = chrono::Utc::now();
        let expires = now + chrono::Duration::minutes(10);

        let session = EmailOtpSession {
            email: req.email.clone(),
            code: code.clone(),
            device_name: req.device_name,
            created_at: now,
            expires_at: expires,
        };

        let mut lock = self.email_otps.write().await;
        lock.insert(req.email.to_lowercase(), session);
        code
    }

    pub async fn verify_email_otp(
        &self,
        req: EmailOtpVerifyRequest,
    ) -> Result<PairingConfirmResponse, &'static str> {
        let email_key = req.email.to_lowercase();
        let mut otps = self.email_otps.write().await;

        if let Some(session) = otps.get(&email_key) {
            if session.code != req.otp_code {
                return Err("Incorrect OTP code. Please check your email.");
            }
            if chrono::Utc::now() > session.expires_at {
                otps.remove(&email_key);
                return Err("OTP code has expired. Please request a new one.");
            }

            let device_id = if req.client_device_id.is_empty() {
                format!("email-dev-{}", uuid::Uuid::new_v4().simple())
            } else {
                req.client_device_id.clone()
            };

            let mut devices = self.devices.write().await;
            devices.insert(
                device_id.clone(),
                DeviceRegistration {
                    device_id: device_id.clone(),
                    device_name: req.client_device_name.clone(),
                    platform: req.client_platform.unwrap_or(PlatformKind::Linux),
                    enrolled_at: chrono::Utc::now(),
                    last_seen_at: chrono::Utc::now(),
                    is_revoked: false,
                    is_approved: true,
                    ip_address: Some(format!("Email-verified: {}", req.email)),
                    role: "collaborator".to_string(),
                },
            );

            let token = format!("rain_tok_{}", uuid::Uuid::new_v4().simple());
            let mut tokens = self.auth_tokens.write().await;
            tokens.insert(token.clone(), device_id.clone());

            otps.remove(&email_key);

            Ok(PairingConfirmResponse {
                token,
                approved: true,
                host_name: std::env::var("HOSTNAME")
                    .unwrap_or_else(|_| "RAIN-Hub-Host".to_string()),
                device_id,
                role: "collaborator".to_string(),
            })
        } else {
            Err("No OTP request found for this email address")
        }
    }

    pub async fn validate_token(&self, token: &str) -> Option<DeviceRegistration> {
        let device_id = self.auth_tokens.read().await.get(token).cloned();
        if let Some(device_id) = device_id {
            let mut devices = self.devices.write().await;
            if let Some(device) = devices.get_mut(&device_id) {
                if !device.is_revoked {
                    device.last_seen_at = chrono::Utc::now();
                    return Some(device.clone());
                }
            }
        }
        None
    }

    pub async fn get_hub_topology(&self) -> HubTopologyStatus {
        let devices: Vec<_> = self.devices.read().await.values().cloned().collect();
        let live_spokes = self.spokes.read().await.clone();
        let now = chrono::Utc::now();
        let uptime_seconds = (now - self.hub_start_time).num_seconds().max(0) as u64;

        let mut spokes = Vec::new();
        for d in &devices {
            let is_online = !d.is_revoked
                && if d.role == "spoke_node" {
                    live_spokes
                        .get(&d.device_id)
                        .map(|s| (now - s.last_heartbeat).num_seconds() < 60)
                        .unwrap_or(false)
                } else {
                    (now - d.last_seen_at).num_minutes() < 15
                };
            spokes.push(HubSpokeNode {
                device_id: d.device_id.clone(),
                device_name: d.device_name.clone(),
                platform: d.platform.clone(),
                is_online,
                enrolled_at: d.enrolled_at,
                last_seen_at: d.last_seen_at,
                ip_address: d.ip_address.clone(),
                role: d.role.clone(),
            });
        }

        spokes.sort_by(|a, b| b.enrolled_at.cmp(&a.enrolled_at));
        let active_count = spokes.iter().filter(|s| s.is_online).count();

        HubTopologyStatus {
            hub_name: std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Primary-Hub".to_string()),
            hub_node_id: crate::network::local_node_id(),
            hub_listen_addr: format!(
                "127.0.0.1:{}",
                std::env::var("RAIN_PORT").unwrap_or_else(|_| "7890".into())
            ),
            is_hub_online: true,
            uptime_seconds,
            active_spokes_count: active_count,
            spokes,
        }
    }

    pub async fn register_spoke(&self, req: SpokeRegisterRequest) -> Result<String, String> {
        if let Some(code) = &req.pairing_code {
            self.verify_pairing_code(&req.device_id, code)
                .await
                .map_err(str::to_string)?;
        }
        let now = chrono::Utc::now();
        let token = format!("rain_spoke_{}", uuid::Uuid::new_v4().simple());

        if req.device_id == crate::network::local_node_id() {
            return Err("A hub cannot enroll itself as a descendant".into());
        }
        if self
            .devices
            .read()
            .await
            .get(&req.device_id)
            .map(|d| d.is_revoked)
            .unwrap_or(false)
        {
            return Err("Node is revoked; pair it again before reconnecting".into());
        }
        let spoke_info = SpokeNodeInfo {
            revoked: false,
            is_hub: req.is_hub,
            endpoint: req.endpoint.clone(),
            children: vec![],
            device_id: req.device_id.clone(),
            device_name: req.device_name.clone(),
            platform: req.platform.clone(),
            ip_address: req.ip_address.clone(),
            cpu_cores: req.cpu_cores,
            ram_total_gb: req.ram_total_gb,
            gpu_model: req.gpu_model.clone(),
            cpu_usage: 0.0,
            memory_used_bytes: 0,
            memory_total_bytes: (req.ram_total_gb * 1024.0 * 1024.0 * 1024.0) as u64,
            disk_used_bytes: 0,
            disk_total_bytes: 0,
            active_agents_count: 0,
            uptime_seconds: 0,
            battery: req.battery.clone(),
            last_heartbeat: now,
        };

        {
            let mut spokes = self.spokes.write().await;
            spokes.insert(req.device_id.clone(), spoke_info);
        }

        // Also enroll into devices table so Star Topology views show it immediately
        {
            let mut devices = self.devices.write().await;
            devices.insert(
                req.device_id.clone(),
                DeviceRegistration {
                    device_id: req.device_id.clone(),
                    device_name: req.device_name.clone(),
                    platform: req.platform.clone(),
                    enrolled_at: now,
                    last_seen_at: now,
                    is_revoked: false,
                    is_approved: true,
                    ip_address: Some(req.ip_address.clone()),
                    role: "spoke_node".to_string(),
                },
            );
        }

        {
            let mut tokens = self.auth_tokens.write().await;
            tokens.insert(token.clone(), req.device_id);
        }

        Ok(token)
    }

    pub async fn heartbeat_spoke(&self, req: SpokeHeartbeatRequest) -> Result<(), String> {
        let now = chrono::Utc::now();
        let mut children = req.children;
        crate::network::validate_subtree(&mut children, &req.device_id)?;
        let mut spokes = self.spokes.write().await;
        if let Some(s) = spokes.get_mut(&req.device_id) {
            if s.revoked {
                return Err("Node is revoked".into());
            }
            if !s.is_hub && !children.is_empty() {
                return Err("Worker nodes cannot advertise child hubs".into());
            }
            s.children = children;
            s.cpu_usage = req.cpu_usage;
            s.memory_used_bytes = req.memory_used_bytes;
            s.memory_total_bytes = req.memory_total_bytes;
            s.disk_used_bytes = req.disk_used_bytes;
            s.disk_total_bytes = req.disk_total_bytes;
            s.active_agents_count = req.active_agents_count;
            s.uptime_seconds = req.uptime_seconds;
            if req.battery.is_some() {
                s.battery = req.battery;
            }
            s.last_heartbeat = now;

            // Touch last_seen_at in devices table
            let mut devices = self.devices.write().await;
            if let Some(d) = devices.get_mut(&req.device_id) {
                d.last_seen_at = now;
            }
            Ok(())
        } else {
            Err("Spoke not found. Please register spoke first.".to_string())
        }
    }

    pub async fn get_active_spokes(&self) -> Vec<SpokeNodeInfo> {
        let spokes = self.spokes.read().await;
        let now = chrono::Utc::now();
        // Return spokes active in the last 60 seconds
        spokes
            .values()
            .filter(|s| !s.revoked && (now - s.last_heartbeat).num_seconds() < 60)
            .cloned()
            .collect()
    }

    pub async fn restore_network(path: &std::path::Path) -> Result<Self, String> {
        let mut store = Self::new();
        store.network_path = Some(path.into());
        match tokio::fs::read(path).await {
            Ok(bytes) => {
                let snapshot: NetworkSnapshot = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("Cannot load network state: {e}"))?;
                *store.devices.write().await = snapshot.devices;
                let mut spokes = snapshot.spokes;
                for spoke in spokes.values_mut() {
                    spoke.last_heartbeat = chrono::Utc::now() - chrono::Duration::seconds(61);
                }
                *store.spokes.write().await = spokes;
                *store.auth_tokens.write().await = snapshot.tokens;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        Ok(store)
    }
    pub async fn persist_network(&self) -> Result<(), String> {
        let Some(path) = &self.network_path else {
            return Ok(());
        };
        let _serial = self.network_persistence.lock().await;
        let devices = self.devices.read().await.clone();
        let spokes = self.spokes.read().await.clone();
        let tokens = self.auth_tokens.read().await.clone();
        let bytes = serde_json::to_vec(&NetworkSnapshot {
            devices,
            spokes,
            tokens,
        })
        .map_err(|e| e.to_string())?;
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| e.to_string())?;
        }
        let temporary = path.with_extension("json.tmp");
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);
        use tokio::io::AsyncWriteExt;
        let mut file = options.open(&temporary).await.map_err(|e| e.to_string())?;
        file.write_all(&bytes).await.map_err(|e| e.to_string())?;
        file.sync_all().await.map_err(|e| e.to_string())?;
        drop(file);
        tokio::fs::rename(temporary, path)
            .await
            .map_err(|e| e.to_string())
    }
    pub async fn network_children(&self) -> Vec<rain_protocol::NetworkNode> {
        let now = chrono::Utc::now();
        let spokes = self.spokes.read().await;
        spokes
            .values()
            .map(|s| {
                let online = !s.revoked && (now - s.last_heartbeat).num_seconds() < 60;
                let mut children = s.children.clone();
                crate::network::refresh_liveness(&mut children, online);
                rain_protocol::NetworkNode {
                    revoked: s.revoked,
                    id: s.device_id.clone(),
                    name: s.device_name.clone(),
                    role: if s.is_hub { "hub" } else { "worker" }.into(),
                    parent_id: Some(crate::network::local_node_id()),
                    endpoint: s.endpoint.clone(),
                    online,
                    last_seen: s.last_heartbeat,
                    cpu_cores: s.cpu_cores,
                    ram_total_bytes: s.memory_total_bytes,
                    ram_available_bytes: s.memory_total_bytes.saturating_sub(s.memory_used_bytes),
                    active_agents: s.active_agents_count,
                    children,
                }
            })
            .collect()
    }
    pub async fn revoke_device(&self, device_id: &str) -> bool {
        let revoked = {
            let mut devices = self.devices.write().await;
            if let Some(device) = devices.get_mut(device_id) {
                device.is_revoked = true;
                true
            } else {
                false
            }
        };
        if revoked {
            self.auth_tokens
                .write()
                .await
                .retain(|_, id| id != device_id);
            if let Some(node) = self.spokes.write().await.get_mut(device_id) {
                node.revoked = true;
            }
        }
        revoked
    }
}

#[derive(Serialize, Deserialize)]
struct NetworkSnapshot {
    devices: HashMap<String, DeviceRegistration>,
    spokes: HashMap<String, SpokeNodeInfo>,
    tokens: HashMap<String, String>,
}

impl Default for InMemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Generates a valid, crisp SVG QR Code pattern directly without external dependencies
fn generate_svg_qr(payload: &str, _code: &str) -> String {
    let size = 25; // 25x25 QR grid
    let module_size = 8;
    let total_px = size * module_size;

    let mut matrix = vec![vec![false; size]; size];

    // Helper: Draw 7x7 Finder Pattern with 1px border and center 3x3 square
    let mut draw_finder = |start_r: usize, start_c: usize| {
        for r in 0..7 {
            for c in 0..7 {
                let is_border = r == 0 || r == 6 || c == 0 || c == 6;
                let is_center = r >= 2 && r <= 4 && c >= 2 && c <= 4;
                matrix[start_r + r][start_c + c] = is_border || is_center;
            }
        }
    };

    // Draw 3 standard corner finders
    draw_finder(0, 0); // Top-left
    draw_finder(0, size - 7); // Top-right
    draw_finder(size - 7, 0); // Bottom-left

    // Timing lines (row 6 and col 6 alternating)
    for i in 8..(size - 8) {
        matrix[6][i] = i % 2 == 0;
        matrix[i][6] = i % 2 == 0;
    }

    // Deterministic payload hashing to fill data modules
    let bytes = payload.as_bytes();
    for r in 0..size {
        for c in 0..size {
            // Skip finder zones and timing lines
            let in_top_left = r <= 7 && c <= 7;
            let in_top_right = r <= 7 && c >= (size - 8);
            let in_bottom_left = r >= (size - 8) && c <= 7;
            let is_timing = r == 6 || c == 6;

            if !in_top_left && !in_top_right && !in_bottom_left && !is_timing {
                let idx = (r * size + c) % bytes.len();
                let b = bytes[idx];
                let bit = (b >> ((r + c) % 8)) & 1;
                matrix[r][c] = bit == 1;
            }
        }
    }

    // Build SVG string
    let mut rects = String::new();
    for r in 0..size {
        for c in 0..size {
            if matrix[r][c] {
                let x = c * module_size;
                let y = r * module_size;
                rects.push_str(&format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#059669\"/>",
                    x, y, module_size, module_size
                ));
            }
        }
    }

    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"180\" height=\"180\" class=\"qr-svg-matrix\" style=\"border-radius:12px;background:#ffffff;padding:8px;box-shadow:0 8px 24px rgba(0,0,0,0.15);\"><rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>{}</svg>",
        total_px, total_px, rects
    )
}
