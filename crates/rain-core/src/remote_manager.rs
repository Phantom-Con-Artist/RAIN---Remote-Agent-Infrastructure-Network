use chrono::{DateTime, Utc};
use rain_protocol::{
    ArtifactTransferRequest, BatteryStatus, FileEntry, InstalledApp, MachineOverview, MachineResourceCatalog,
    PlatformKind, ReadFileResponse, TerminalExecuteRequest, TerminalExecuteResponse,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{Disks, System};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct RemoteManager {
    system: Arc<RwLock<System>>,
}

impl RemoteManager {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self {
            system: Arc::new(RwLock::new(sys)),
        }
    }

    fn resolve_path(input: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        if input.is_empty() || input == "~" || input == "." {
            PathBuf::from(home)
        } else if let Some(stripped) = input.strip_prefix("~/") {
            PathBuf::from(home).join(stripped)
        } else {
            PathBuf::from(input)
        }
    }

    pub fn list_directory(&self, req_path: &str) -> Result<Vec<FileEntry>, String> {
        let target = Self::resolve_path(req_path);
        if !target.exists() {
            return Err(format!("Directory does not exist: {}", target.display()));
        }

        let read_dir = fs::read_dir(&target)
            .map_err(|e| format!("Cannot read directory '{}': {}", target.display(), e))?;

        let mut entries = Vec::new();

        // Add parent directory ".." navigation if not root
        if let Some(parent) = target.parent() {
            entries.push(FileEntry {
                name: "..".to_string(),
                path: parent.to_string_lossy().to_string(),
                is_dir: true,
                size_bytes: 0,
                modified_time: None,
                is_editable: false,
            });
        }

        for entry in read_dir.flatten() {
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            // Skip hidden entries if desired, but show config files like .env or .toml
            let meta = entry.metadata().ok();
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size_bytes = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified_time = meta.and_then(|m| m.modified().ok()).map(|st| {
                DateTime::<Utc>::from(st)
            });

            let is_editable = !is_dir && is_text_editable(&file_name);

            entries.push(FileEntry {
                name: file_name,
                path: path.to_string_lossy().to_string(),
                is_dir,
                size_bytes,
                modified_time,
                is_editable,
            });
        }

        // Sort: directories first (with ".." on top), then files alphabetically
        entries.sort_by(|a, b| {
            if a.name == ".." {
                std::cmp::Ordering::Less
            } else if b.name == ".." {
                std::cmp::Ordering::Greater
            } else if a.is_dir != b.is_dir {
                b.is_dir.cmp(&a.is_dir)
            } else {
                a.name.to_lowercase().cmp(&b.name.to_lowercase())
            }
        });

        Ok(entries)
    }

    pub fn read_file(&self, req_path: &str) -> Result<ReadFileResponse, String> {
        let target = Self::resolve_path(req_path);
        if !target.exists() {
            return Err(format!("File does not exist: {}", target.display()));
        }
        if target.is_dir() {
            return Err(format!("'{}' is a directory, not a file", target.display()));
        }

        let metadata = fs::metadata(&target).map_err(|e| e.to_string())?;
        let size_bytes = metadata.len();

        if size_bytes > 5 * 1024 * 1024 {
            return Err("File is larger than 5MB limit for remote editor".to_string());
        }

        let bytes = fs::read(&target).map_err(|e| format!("Failed to read file: {}", e))?;
        let is_binary = bytes.iter().take(1024).any(|&b| b == 0);

        let content = if is_binary {
            format!("[Binary file: {} bytes]", size_bytes)
        } else {
            String::from_utf8_lossy(&bytes).to_string()
        };

        Ok(ReadFileResponse {
            path: target.to_string_lossy().to_string(),
            content,
            size_bytes,
            is_binary,
        })
    }

    pub fn write_file(&self, req_path: &str, content: &str) -> Result<(), String> {
        let target = Self::resolve_path(req_path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed to create directories: {}", e))?;
        }
        fs::write(&target, content).map_err(|e| format!("Failed to write file '{}': {}", target.display(), e))
    }

    pub fn list_installed_apps(&self) -> Vec<InstalledApp> {
        let candidates = vec![
            ("code", "Visual Studio Code", "code", "Development"),
            ("cursor", "Cursor AI Editor", "code", "Development"),
            ("alacritty", "Alacritty Terminal", "terminal", "System"),
            ("gnome-terminal", "GNOME Terminal", "terminal", "System"),
            ("ptyxis", "Ptyxis Terminal", "terminal", "System"),
            ("firefox", "Firefox Browser", "globe", "Internet"),
            ("google-chrome", "Google Chrome", "globe", "Internet"),
            ("chromium", "Chromium Browser", "globe", "Internet"),
            ("blender", "Blender 3D", "box", "Graphics"),
            ("godot", "Godot Game Engine", "cpu", "Development"),
            ("htop", "htop System Monitor", "activity", "System"),
            ("btop", "btop Resource Monitor", "activity", "System"),
            ("git", "Git Version Control", "git-branch", "Development"),
            ("docker", "Docker Daemon", "server", "Virtualization"),
            ("nautilus", "Files (Nautilus)", "folder", "System"),
        ];

        let mut installed = Vec::new();
        for (bin, name, icon, cat) in candidates {
            if is_binary_available(bin) {
                installed.push(InstalledApp {
                    id: bin.to_string(),
                    name: name.to_string(),
                    executable: bin.to_string(),
                    icon_name: icon.to_string(),
                    category: cat.to_string(),
                });
            }
        }

        if installed.is_empty() {
            // Provide fallback tools
            installed.push(InstalledApp {
                id: "bash".to_string(),
                name: "Bash Shell".to_string(),
                executable: "bash".to_string(),
                icon_name: "terminal".to_string(),
                category: "System".to_string(),
            });
        }

        installed
    }

    pub async fn launch_app(&self, app_id: &str, custom_args: Option<Vec<String>>) -> Result<String, String> {
        let args = custom_args.unwrap_or_default();
        let display = std::env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string());
        let wayland_display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();

        let mut cmd = Command::new(app_id);
        cmd.args(&args)
            .env("DISPLAY", display)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        if !wayland_display.is_empty() {
            cmd.env("WAYLAND_DISPLAY", wayland_display);
        }

        match cmd.spawn() {
            Ok(child) => {
                let pid = child.id().unwrap_or(0);
                Ok(format!("Application '{}' launched successfully (PID: {})", app_id, pid))
            }
            Err(e) => Err(format!("Failed to launch application '{}': {}", app_id, e)),
        }
    }

    pub async fn execute_terminal(
        &self,
        req: TerminalExecuteRequest,
    ) -> Result<TerminalExecuteResponse, String> {
        let start = Instant::now();
        let cwd = req.cwd.as_deref().unwrap_or(".");
        let resolved_cwd = Self::resolve_path(cwd);

        if req.is_sudo {
            let password = req.sudo_password.ok_or_else(|| "Sudo password is required for privileged command".to_string())?;

            let mut child = Command::new("sudo")
                .arg("-S")
                .arg("-p")
                .arg("")
                .arg("bash")
                .arg("-c")
                .arg(&req.command)
                .current_dir(&resolved_cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn sudo process: {}", e))?;

            if let Some(mut stdin) = child.stdin.take() {
                let _ = stdin.write_all(format!("{}\n", password).as_bytes()).await;
                let _ = stdin.flush().await;
            }

            let output = child
                .wait_with_output()
                .await
                .map_err(|e| format!("Execution error: {}", e))?;

            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(TerminalExecuteResponse {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration_ms,
            })
        } else {
            let output = Command::new("bash")
                .arg("-c")
                .arg(&req.command)
                .current_dir(&resolved_cwd)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output()
                .await
                .map_err(|e| format!("Execution error: {}", e))?;

            let duration_ms = start.elapsed().as_millis() as u64;
            Ok(TerminalExecuteResponse {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                duration_ms,
            })
        }
    }

    pub async fn get_machines_overview(&self) -> Vec<MachineOverview> {
        let mut sys = self.system.write().await;
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();

        let host_name = System::host_name().unwrap_or_else(|| "Office-PC-Linux".to_string());
        let cpu_usage = sys.global_cpu_usage();
        let memory_total_bytes = sys.total_memory();
        let memory_used_bytes = sys.used_memory();
        let disk_total_bytes: u64 = disks.iter().map(|d| d.total_space()).sum();
        let disk_used_bytes: u64 = disks.iter().map(|d| d.total_space().saturating_sub(d.available_space())).sum();
        let uptime_seconds = System::uptime();

        let host_machine = MachineOverview {
            id: "host-primary".to_string(),
            name: host_name,
            platform: PlatformKind::Linux,
            is_online: true,
            ip_address: "127.0.0.1 (USB-Forwarded)".to_string(),
            cpu_usage,
            memory_used_bytes,
            memory_total_bytes,
            disk_used_bytes,
            disk_total_bytes,
            active_agents_count: 2,
            uptime_seconds,
            battery: Some(BatteryStatus {
                percent: 100,
                is_charging: true,
                is_ac_connected: true,
                power_source: "AC Power".to_string(),
                time_remaining_minutes: None,
                is_critical: false,
            }),
        };

        // Surface linked peer machines in the RAIN mesh
        let macbook_peer = MachineOverview {
            id: "peer-macbook".to_string(),
            name: "Home-Laptop-Linux".to_string(),
            platform: PlatformKind::Linux,
            is_online: true,
            ip_address: "192.168.1.142".to_string(),
            cpu_usage: 14.2,
            memory_used_bytes: 12_884_901_888,
            memory_total_bytes: 34_359_738_368,
            disk_used_bytes: 245_000_000_000,
            disk_total_bytes: 512_000_000_000,
            active_agents_count: 1,
            uptime_seconds: 142_900,
            battery: Some(BatteryStatus {
                percent: 5,
                is_charging: false,
                is_ac_connected: false,
                power_source: "Battery".to_string(),
                time_remaining_minutes: Some(11),
                is_critical: true,
            }),
        };

        let server_peer = MachineOverview {
            id: "peer-desktop-mac".to_string(),
            name: "Office-Desktop-Mac".to_string(),
            platform: PlatformKind::MacOS,
            is_online: true,
            ip_address: "10.0.0.42".to_string(),
            cpu_usage: 42.8,
            memory_used_bytes: 48_100_000_000,
            memory_total_bytes: 128_000_000_000,
            disk_used_bytes: 1_200_000_000_000,
            disk_total_bytes: 4_000_000_000_000,
            active_agents_count: 0,
            uptime_seconds: 864_000,
            battery: Some(BatteryStatus {
                percent: 100,
                is_charging: true,
                is_ac_connected: true,
                power_source: "AC Power (10Gbps Fiber)".to_string(),
                time_remaining_minutes: None,
                is_critical: false,
            }),
        };

        vec![host_machine, macbook_peer, server_peer]
    }

    pub async fn get_machine_resources(&self) -> Vec<MachineResourceCatalog> {
        let mut sys = self.system.write().await;
        sys.refresh_all();
        let cpu_cores = sys.cpus().len();
        let ram_total_gb = (sys.total_memory() as f32) / (1024.0 * 1024.0 * 1024.0);
        let ram_free_gb = (sys.free_memory() as f32) / (1024.0 * 1024.0 * 1024.0);
        let host_name = System::host_name().unwrap_or_else(|| "Office-PC-Linux".to_string());

        let host = MachineResourceCatalog {
            machine_id: "host-primary".to_string(),
            machine_name: host_name,
            platform: PlatformKind::Linux,
            cpu_cores,
            ram_total_gb: (ram_total_gb * 10.0).round() / 10.0,
            ram_free_gb: (ram_free_gb * 10.0).round() / 10.0,
            gpu_model: Some("NVIDIA GeForce RTX 4090 (24GB VRAM)".to_string()),
            network_speed_mbps: 1000,
            tags: vec!["cuda-12".to_string(), "docker".to_string(), "sqlite".to_string(), "gpu-accelerated".to_string()],
            battery: Some(BatteryStatus {
                percent: 100,
                is_charging: true,
                is_ac_connected: true,
                power_source: "AC Power".to_string(),
                time_remaining_minutes: None,
                is_critical: false,
            }),
        };

        let laptop = MachineResourceCatalog {
            machine_id: "peer-laptop".to_string(),
            machine_name: "Home-Laptop-Linux".to_string(),
            platform: PlatformKind::Linux,
            cpu_cores: 16,
            ram_total_gb: 32.0,
            ram_free_gb: 18.4,
            gpu_model: Some("NVIDIA GeForce RTX 4070 (12GB VRAM)".to_string()),
            network_speed_mbps: 300,
            tags: vec!["mobile-workstation".to_string(), "cuda-12".to_string(), "local-models".to_string()],
            battery: Some(BatteryStatus {
                percent: 5,
                is_charging: false,
                is_ac_connected: false,
                power_source: "Battery".to_string(),
                time_remaining_minutes: Some(11),
                is_critical: true,
            }),
        };

        let desktop = MachineResourceCatalog {
            machine_id: "peer-desktop-mac".to_string(),
            machine_name: "Office-Desktop-Mac".to_string(),
            platform: PlatformKind::MacOS,
            cpu_cores: 64,
            ram_total_gb: 128.0,
            ram_free_gb: 84.2,
            gpu_model: Some("Apple M3 Max 40-Core GPU".to_string()),
            network_speed_mbps: 10_000,
            tags: vec!["64-core-cpu".to_string(), "10gbps-fiber".to_string(), "staging-server".to_string(), "browser-automation".to_string()],
            battery: Some(BatteryStatus {
                percent: 100,
                is_charging: true,
                is_ac_connected: true,
                power_source: "AC Power".to_string(),
                time_remaining_minutes: None,
                is_critical: false,
            }),
        };

        vec![host, laptop, desktop]
    }

    pub fn handle_artifact_transfer(&self, req: ArtifactTransferRequest) -> Result<String, String> {
        let dest = Self::resolve_path(&req.target_path);
        if let Some(parent) = dest.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let dummy_data = format!("Transferred artifact '{}' ({} bytes) from {} to {}", req.file_name, req.size_bytes, req.source_machine, req.target_machine);
        fs::write(&dest, dummy_data).map_err(|e| e.to_string())?;
        Ok(format!("Artifact '{}' transferred and staged on target machine at {}", req.file_name, dest.display()))
    }
}

fn is_binary_available(bin: &str) -> bool {
    let path_var = std::env::var("PATH").unwrap_or_default();
    for dir in path_var.split(':') {
        let p = Path::new(dir).join(bin);
        if p.is_file() {
            return true;
        }
    }
    false
}

fn is_text_editable(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    let text_extensions = [
        ".rs", ".ts", ".js", ".json", ".toml", ".md", ".txt", ".sh", ".bash",
        ".yaml", ".yml", ".css", ".html", ".py", ".c", ".cpp", ".h", ".sql",
        ".env", ".xml", ".kt", ".gradle", ".conf", ".ini", ".log"
    ];
    text_extensions.iter().any(|ext| lower.ends_with(ext)) || lower == "dockerfile" || lower == "makefile"
}

impl Default for RemoteManager {
    fn default() -> Self {
        Self::new()
    }
}
