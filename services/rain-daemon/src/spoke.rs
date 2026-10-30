//! A worker or subhub reconnects to its parent with stable identity, authenticated heartbeats and backoff.
use rain_core::{AgentSupervisor, InMemoryStore, RemoteManager};
use rain_protocol::{ApiResponse, SpokeHeartbeatRequest, SpokeRegisterRequest};
use std::sync::Arc;
use tracing::{info, warn};

pub(crate) async fn run_spoke_agent(
    hub_url: &str,
    name: &str,
    pin: Option<String>,
    store: Option<Arc<InMemoryStore>>,
    supervisor: Option<Arc<AgentSupervisor>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let identity_file = std::path::PathBuf::from(
        std::env::var("RAIN_SPOKE_ID_FILE").unwrap_or_else(|_| ".rain/device-id".into()),
    );
    let device_id = if store.is_some() {
        rain_core::network::local_node_id()
    } else {
        match tokio::fs::read_to_string(&identity_file).await {
            Ok(id) => id.trim().into(),
            Err(_) => {
                let id = format!("spoke-{}", uuid::Uuid::new_v4());
                if let Some(parent) = identity_file.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }
                tokio::fs::write(&identity_file, &id).await?;
                id
            }
        }
    };
    let token_file = identity_file.with_extension("token");
    let mut token = tokio::fs::read_to_string(&token_file)
        .await
        .ok()
        .map(|s| s.trim().to_string());
    let mut pin = pin;
    let mut delay = 1u64;
    let mut system = sysinfo::System::new_all();
    let cluster_key = std::env::var("RAIN_CLUSTER_KEY").ok();
    let base = hub_url.trim_end_matches('/');
    loop {
        if let Some(store) = &store {
            // Resolve the parent's ancestry before enrolling to reject accidental hub loops.
            if let Ok(response) = client
                .get(format!("{base}/api/v1/network/topology"))
                .send()
                .await
            {
                if let Ok(value) = response.json::<serde_json::Value>().await {
                    let mut path: Vec<String> = value["data"]["parent_path"]
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|s| s.as_str().map(str::to_owned))
                                .collect()
                        })
                        .unwrap_or_default();
                    if let Some(parent) = value["data"]["root"]["id"].as_str() {
                        path.push(parent.into());
                    }
                    if path.contains(&device_id) {
                        warn!("Parent hierarchy would form a cycle; parent connection suspended");
                        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                        continue;
                    }
                    *store.parent_path.write().await = path;
                }
            }
        }
        system.refresh_all();
        if token.is_none() {
            let registration = SpokeRegisterRequest {
                device_id: device_id.clone(),
                device_name: name.into(),
                platform: match std::env::consts::OS {
                    "linux" => rain_protocol::PlatformKind::Linux,
                    "macos" => rain_protocol::PlatformKind::MacOS,
                    "windows" => rain_protocol::PlatformKind::Windows,
                    _ => rain_protocol::PlatformKind::Unknown,
                },
                ip_address: std::env::var("RAIN_ADVERTISE_URL")
                    .unwrap_or_else(|_| "Not advertised".into()),
                pairing_code: pin.clone(),
                cpu_cores: system.cpus().len(),
                ram_total_gb: system.total_memory() as f32 / (1024.0 * 1024.0 * 1024.0),
                gpu_model: None,
                battery: RemoteManager::read_real_battery(),
                is_hub: store.is_some(),
                endpoint: std::env::var("RAIN_ADVERTISE_URL").ok(),
            };
            let mut request = client
                .post(format!("{base}/api/v1/hub/spokes/register"))
                .json(&registration);
            if let Some(key) = &cluster_key {
                request = request.bearer_auth(key);
            }
            match request.send().await {
                Ok(response) if response.status().is_success() => {
                    let result = response.json::<ApiResponse<String>>().await?;
                    if !result.success || result.data.is_none() {
                        return Err("Parent did not return a node token".into());
                    }
                    token = result.data;
                    pin = None;
                    if let Some(parent) = token_file.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    use tokio::io::AsyncWriteExt;
                    let mut options = tokio::fs::OpenOptions::new();
                    options.create(true).truncate(true).write(true);
                    #[cfg(unix)]
                    options.mode(0o600);
                    let mut file = options.open(&token_file).await?;
                    file.write_all(token.as_ref().unwrap().as_bytes()).await?;
                    file.sync_all().await?;
                    info!("Node '{}' connected to parent {}", name, base);
                    delay = 1;
                }
                Ok(response) => warn!(
                    "Parent registration rejected with HTTP {}",
                    response.status()
                ),
                Err(error) => warn!("Parent unavailable: {}", error),
            }
        }
        if let Some(key) = &token {
            let disks = sysinfo::Disks::new_with_refreshed_list();
            let heartbeat = SpokeHeartbeatRequest {
                device_id: device_id.clone(),
                cpu_usage: system.global_cpu_usage(),
                memory_used_bytes: system.used_memory(),
                memory_total_bytes: system.total_memory(),
                disk_used_bytes: disks
                    .iter()
                    .map(|d| d.total_space().saturating_sub(d.available_space()))
                    .sum(),
                disk_total_bytes: disks.iter().map(|d| d.total_space()).sum(),
                active_agents_count: if let Some(s) = &supervisor {
                    s.list_agents()
                        .await
                        .iter()
                        .filter(|a| a.status == rain_protocol::AgentStatus::Running)
                        .count()
                } else {
                    0
                },
                uptime_seconds: sysinfo::System::uptime(),
                battery: RemoteManager::read_real_battery(),
                children: if let Some(s) = &store {
                    s.network_children().await
                } else {
                    vec![]
                },
            };
            match client
                .post(format!("{base}/api/v1/hub/spokes/heartbeat"))
                .bearer_auth(key)
                .json(&heartbeat)
                .send()
                .await
            {
                Ok(response) if response.status().is_success() => {
                    if let Some(s) = &store {
                        *s.parent_last_seen.write().await = Some(chrono::Utc::now());
                    }
                    delay = 1;
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
                Ok(response) if matches!(response.status().as_u16(), 401 | 403 | 404) => {
                    token = None;
                    let _ = tokio::fs::remove_file(&token_file).await;
                }
                Ok(response) => warn!("Parent heartbeat rejected with HTTP {}", response.status()),
                Err(error) => warn!("Heartbeat interrupted; retaining local work: {}", error),
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
        delay = (delay * 2).min(30);
    }
}
