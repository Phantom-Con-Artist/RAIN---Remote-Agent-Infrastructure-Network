use rain_protocol::{
    AgentLaunchRequest, AgentProcess, ApiResponse, Capability, HealthStatus,
    PairingConfirmRequest, PairingConfirmResponse, SystemStatus,
};
use std::time::Duration;

#[tauri::command]
async fn ping_host(host_url: String) -> Result<HealthStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/health", host_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Failed to connect to host: {}", e))?;

    let api_res: ApiResponse<HealthStatus> = res
        .json()
        .await
        .map_err(|e| format!("Failed to parse response: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Unknown error".to_string()))
}

#[tauri::command]
async fn fetch_system_status(host_url: String) -> Result<SystemStatus, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/v1/system/status", host_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let api_res: ApiResponse<SystemStatus> = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Failed to fetch status".to_string()))
}

#[tauri::command]
async fn fetch_capabilities(host_url: String) -> Result<Vec<Capability>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/v1/system/capabilities", host_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let api_res: ApiResponse<Vec<Capability>> = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Failed to fetch capabilities".to_string()))
}

#[tauri::command]
async fn fetch_agents(host_url: String) -> Result<Vec<AgentProcess>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/v1/agents", host_url.trim_end_matches('/'));
    let res = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let api_res: ApiResponse<Vec<AgentProcess>> = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Failed to fetch agents".to_string()))
}

#[tauri::command]
async fn launch_agent(
    host_url: String,
    req: AgentLaunchRequest,
) -> Result<AgentProcess, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/v1/agents/start", host_url.trim_end_matches('/'));
    let res = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let api_res: ApiResponse<AgentProcess> = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Failed to start agent".to_string()))
}

#[tauri::command]
async fn terminate_agent(host_url: String, agent_id: String) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/v1/agents/{}/stop", host_url.trim_end_matches('/'), agent_id);
    let res = client
        .post(&url)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let api_res: ApiResponse<String> = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Failed to stop agent".to_string()))
}

#[tauri::command]
async fn pair_device(
    host_url: String,
    req: PairingConfirmRequest,
) -> Result<PairingConfirmResponse, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let url = format!("{}/api/v1/auth/pair/confirm", host_url.trim_end_matches('/'));
    let res = client
        .post(&url)
        .json(&req)
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    let api_res: ApiResponse<PairingConfirmResponse> = res
        .json()
        .await
        .map_err(|e| format!("Parse error: {}", e))?;

    api_res
        .data
        .ok_or_else(|| api_res.error.unwrap_or_else(|| "Pairing failed".to_string()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            ping_host,
            fetch_system_status,
            fetch_capabilities,
            fetch_agents,
            launch_agent,
            terminate_agent,
            pair_device,
        ])
        .run(tauri::generate_context!())
        .expect("error while running RAIN client application");
}
