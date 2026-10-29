use rain_core::RemoteManager;
use rain_protocol::{SpokeHeartbeatRequest, SpokeRegisterRequest};
use tracing::{debug, error, info, warn};

pub(crate) async fn run_spoke_agent(
    hub_url: &str,
    spoke_name: &str,
    pairing_code: Option<String>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let device_id = format!("spoke-{}", uuid::Uuid::new_v4().simple());
    let mut sys = sysinfo::System::new_all();
    sys.refresh_all();
    let cpu_cores = sys.cpus().len();
    let ram_total_gb = (sys.total_memory() as f32) / (1024.0 * 1024.0 * 1024.0);

    let reg_req = SpokeRegisterRequest {
        device_id: device_id.clone(),
        device_name: spoke_name.to_string(),
        platform: rain_protocol::PlatformKind::Linux,
        ip_address: "127.0.0.1 (Spoke Mesh)".to_string(),
        pairing_code,
        cpu_cores,
        ram_total_gb: (ram_total_gb * 10.0).round() / 10.0,
        gpu_model: Some("Peer GPU / Worker Node".to_string()),
        battery: RemoteManager::read_real_battery(),
    };

    info!(
        "Registering Spoke node '{}' ({}) with Hub at {}",
        spoke_name, device_id, hub_url
    );
    let reg_url = format!(
        "{}/api/v1/hub/spokes/register",
        hub_url.trim_end_matches('/')
    );

    match client.post(&reg_url).json(&reg_req).send().await {
        Ok(res) if res.status().is_success() => {
            info!(
                "Successfully registered with Hub! Entering continuous telemetry heartbeat loop..."
            );
        }
        Ok(res) => {
            error!("Failed to register with Hub: HTTP {}", res.status());
            return Ok(());
        }
        Err(e) => {
            error!("Cannot connect to Hub at {}: {}", reg_url, e);
            return Ok(());
        }
    }

    let heartbeat_url = format!(
        "{}/api/v1/hub/spokes/heartbeat",
        hub_url.trim_end_matches('/')
    );
    let mut ticker = tokio::time::interval(tokio::time::Duration::from_secs(3));
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let disk_total_bytes: u64 = disks.iter().map(|d| d.total_space()).sum();

    loop {
        ticker.tick().await;
        sys.refresh_all();

        let hb = SpokeHeartbeatRequest {
            device_id: device_id.clone(),
            cpu_usage: sys.global_cpu_usage(),
            memory_used_bytes: sys.used_memory(),
            memory_total_bytes: sys.total_memory(),
            disk_used_bytes: disks
                .iter()
                .map(|d| d.total_space().saturating_sub(d.available_space()))
                .sum(),
            disk_total_bytes,
            active_agents_count: 1,
            uptime_seconds: sysinfo::System::uptime(),
            battery: RemoteManager::read_real_battery(),
        };

        if let Err(e) = client.post(&heartbeat_url).json(&hb).send().await {
            warn!("Spoke heartbeat failed: {}", e);
        } else {
            debug!(
                "Heartbeat delivered to Hub (CPU: {:.1}%, RAM: {:.1} GB)",
                hb.cpu_usage,
                (hb.memory_used_bytes as f32) / (1024.0 * 1024.0 * 1024.0)
            );
        }
    }
}
