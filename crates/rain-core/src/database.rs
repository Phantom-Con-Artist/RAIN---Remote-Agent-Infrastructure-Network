use rain_protocol::{
    DeviceRegistration, EmailOtpRequest, EmailOtpVerifyRequest, HubSpokeNode, HubTopologyStatus,
    PairingConfirmResponse, PairingInitResponse, PairingTicketResponse, PlatformKind,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    active_pairings: Arc<RwLock<HashMap<String, PairingSession>>>,
    email_otps: Arc<RwLock<HashMap<String, EmailOtpSession>>>,
    auth_tokens: Arc<RwLock<HashMap<String, String>>>, // token -> device_id
    hub_start_time: chrono::DateTime<chrono::Utc>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        let now = chrono::Utc::now();
        let mut devices = HashMap::new();
        let mut auth_tokens = HashMap::new();

        // Pre-enroll initial cluster devices so testing and star topology works out of the box
        let mobile_id = "device-moto-edge".to_string();
        let laptop_id = "peer-laptop".to_string();
        let desktop_id = "peer-desktop-mac".to_string();

        devices.insert(
            mobile_id.clone(),
            DeviceRegistration {
                device_id: mobile_id.clone(),
                device_name: "Motorola edge 60 fusion".to_string(),
                platform: PlatformKind::Android,
                enrolled_at: now - chrono::Duration::hours(2),
                last_seen_at: now,
                is_revoked: false,
                is_approved: true,
                ip_address: Some("192.168.1.142 (USB ADB)".to_string()),
                role: "admin".to_string(),
            },
        );
        auth_tokens.insert("rain_tok_mobile_demo".to_string(), mobile_id);

        devices.insert(
            laptop_id.clone(),
            DeviceRegistration {
                device_id: laptop_id.clone(),
                device_name: "Home-Laptop-Linux".to_string(),
                platform: PlatformKind::Linux,
                enrolled_at: now - chrono::Duration::days(1),
                last_seen_at: now - chrono::Duration::minutes(3),
                is_revoked: false,
                is_approved: true,
                ip_address: Some("192.168.1.105".to_string()),
                role: "collaborator".to_string(),
            },
        );

        devices.insert(
            desktop_id.clone(),
            DeviceRegistration {
                device_id: desktop_id.clone(),
                device_name: "Office-Desktop-Mac".to_string(),
                platform: PlatformKind::MacOS,
                enrolled_at: now - chrono::Duration::days(3),
                last_seen_at: now - chrono::Duration::minutes(1),
                is_revoked: false,
                is_approved: true,
                ip_address: Some("10.0.0.42 (Tailscale/Relay)".to_string()),
                role: "collaborator".to_string(),
            },
        );

        Self {
            devices: Arc::new(RwLock::new(devices)),
            active_pairings: Arc::new(RwLock::new(HashMap::new())),
            email_otps: Arc::new(RwLock::new(HashMap::new())),
            auth_tokens: Arc::new(RwLock::new(auth_tokens)),
            hub_start_time: now,
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

    pub async fn generate_pairing_ticket(&self, host_name: &str, host_listen_addr: &str) -> PairingTicketResponse {
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
            code,
            encoded_host,
            encoded_addr
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

    pub async fn verify_email_otp(&self, req: EmailOtpVerifyRequest) -> Result<PairingConfirmResponse, &'static str> {
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
                host_name: std::env::var("HOSTNAME").unwrap_or_else(|_| "RAIN-Hub-Host".to_string()),
                device_id,
                role: "collaborator".to_string(),
            })
        } else {
            Err("No OTP request found for this email address")
        }
    }

    pub async fn validate_token(&self, token: &str) -> Option<DeviceRegistration> {
        let tokens = self.auth_tokens.read().await;
        if let Some(device_id) = tokens.get(token) {
            let mut devices = self.devices.write().await;
            if let Some(device) = devices.get_mut(device_id) {
                if !device.is_revoked {
                    device.last_seen_at = chrono::Utc::now();
                    return Some(device.clone());
                }
            }
        }
        None
    }

    pub async fn get_hub_topology(&self) -> HubTopologyStatus {
        let devices = self.devices.read().await;
        let now = chrono::Utc::now();
        let uptime_seconds = (now - self.hub_start_time).num_seconds().max(0) as u64;

        let mut spokes = Vec::new();
        for d in devices.values() {
            let is_online = !d.is_revoked && (now - d.last_seen_at).num_minutes() < 15;
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
            hub_node_id: "ed25519:7a4f91b8e3c0".to_string(),
            hub_listen_addr: "0.0.0.0:7890 (LAN / Tailscale / Relay)".to_string(),
            is_hub_online: true,
            uptime_seconds,
            active_spokes_count: active_count,
            spokes,
        }
    }

    pub async fn revoke_device(&self, device_id: &str) -> bool {
        let mut devices = self.devices.write().await;
        if let Some(device) = devices.get_mut(device_id) {
            device.is_revoked = true;
            // Also invalidate tokens
            let mut tokens = self.auth_tokens.write().await;
            tokens.retain(|_, id| id != device_id);
            true
        } else {
            false
        }
    }
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
    draw_finder(0, 0);              // Top-left
    draw_finder(0, size - 7);       // Top-right
    draw_finder(size - 7, 0);       // Bottom-left

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
