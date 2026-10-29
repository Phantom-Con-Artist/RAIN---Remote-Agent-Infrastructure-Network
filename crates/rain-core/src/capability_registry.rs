use rain_platform::{current_platform_adapter, PlatformAdapter};
use rain_protocol::{
    Capability, CapabilityInvocation, CapabilityResult, DiskInfo, ProcessSummary, SystemStatus,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct CapabilityRegistry {
    platform: Arc<Mutex<Box<dyn PlatformAdapter + Send + Sync>>>,
    capabilities: HashMap<String, Capability>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        let adapter = current_platform_adapter();
        let mut caps = HashMap::new();
        for cap in adapter.get_supported_capabilities() {
            caps.insert(cap.name.clone(), cap);
        }

        Self {
            platform: Arc::new(Mutex::new(adapter)),
            capabilities: caps,
        }
    }

    pub fn list_capabilities(&self) -> Vec<Capability> {
        self.capabilities.values().cloned().collect()
    }

    pub async fn get_system_status(&self) -> Result<SystemStatus, String> {
        let mut plat = self.platform.lock().await;
        plat.get_system_status().map_err(|e| e.to_string())
    }

    pub async fn list_processes(&self, limit: usize) -> Result<Vec<ProcessSummary>, String> {
        let mut plat = self.platform.lock().await;
        plat.list_processes(limit).map_err(|e| e.to_string())
    }

    pub async fn list_disks(&self) -> Result<Vec<DiskInfo>, String> {
        let mut plat = self.platform.lock().await;
        plat.list_disks().map_err(|e| e.to_string())
    }

    pub async fn invoke(&self, req: CapabilityInvocation) -> CapabilityResult {
        let start = std::time::Instant::now();
        let name = req.capability_name.as_str();

        match name {
            "system.get_status" => match self.get_system_status().await {
                Ok(status) => CapabilityResult {
                    invocation_id: req.id,
                    success: true,
                    data: serde_json::to_value(status).ok(),
                    error: None,
                    duration_ms: start.elapsed().as_millis() as u64,
                },
                Err(e) => CapabilityResult {
                    invocation_id: req.id,
                    success: false,
                    data: None,
                    error: Some(e),
                    duration_ms: start.elapsed().as_millis() as u64,
                },
            },
            "process.list" => {
                let limit = req
                    .parameters
                    .get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(20) as usize;
                match self.list_processes(limit).await {
                    Ok(procs) => CapabilityResult {
                        invocation_id: req.id,
                        success: true,
                        data: serde_json::to_value(procs).ok(),
                        error: None,
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                    Err(e) => CapabilityResult {
                        invocation_id: req.id,
                        success: false,
                        data: None,
                        error: Some(e),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            "terminal.execute_command" => {
                let cmd = req
                    .parameters
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if cmd.is_empty() {
                    return CapabilityResult {
                        invocation_id: req.id,
                        success: false,
                        data: None,
                        error: Some("Command parameter is required".to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    };
                }

                // Execute shell command
                let output = tokio::process::Command::new("sh")
                    .arg("-c")
                    .arg(cmd)
                    .output()
                    .await;

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                        let code = out.status.code().unwrap_or(-1);
                        CapabilityResult {
                            invocation_id: req.id,
                            success: out.status.success(),
                            data: Some(serde_json::json!({
                                "stdout": stdout,
                                "stderr": stderr,
                                "exit_code": code,
                            })),
                            error: if out.status.success() {
                                None
                            } else {
                                Some(format!("Command exited with status {}", code))
                            },
                            duration_ms: start.elapsed().as_millis() as u64,
                        }
                    }
                    Err(err) => CapabilityResult {
                        invocation_id: req.id,
                        success: false,
                        data: None,
                        error: Some(err.to_string()),
                        duration_ms: start.elapsed().as_millis() as u64,
                    },
                }
            }
            unknown => CapabilityResult {
                invocation_id: req.id,
                success: false,
                data: None,
                error: Some(format!("Unknown capability: {}", unknown)),
                duration_ms: start.elapsed().as_millis() as u64,
            },
        }
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
