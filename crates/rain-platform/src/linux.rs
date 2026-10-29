use crate::error::PlatformError;
use crate::traits::PlatformAdapter;
use rain_protocol::{
    Capability, DiskInfo, PlatformKind, ProcessSummary, RiskLevel, SystemStatus,
};
use sysinfo::{Disks, System};

pub struct LinuxAdapter {
    sys: System,
    disks: Disks,
}

impl LinuxAdapter {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        Self { sys, disks }
    }
}

impl Default for LinuxAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl PlatformAdapter for LinuxAdapter {
    fn name(&self) -> &'static str {
        "rain-platform-linux"
    }

    fn kind(&self) -> PlatformKind {
        PlatformKind::Linux
    }

    fn refresh(&mut self) {
        self.sys.refresh_all();
        self.disks.refresh(true);
    }

    fn get_system_status(&mut self) -> Result<SystemStatus, PlatformError> {
        self.refresh();

        let hostname = System::host_name().unwrap_or_else(|| "unknown-host".to_string());
        let os_name = System::name().unwrap_or_else(|| "Linux".to_string());
        let os_version = System::os_version().unwrap_or_else(|| "unknown".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "unknown".to_string());
        let uptime_seconds = System::uptime();

        let cpu_count = self.sys.cpus().len();
        let cpu_usage_percent = self.sys.global_cpu_usage();

        let memory_total_bytes = self.sys.total_memory();
        let memory_used_bytes = self.sys.used_memory();
        let memory_free_bytes = self.sys.free_memory();
        let swap_total_bytes = self.sys.total_swap();
        let swap_used_bytes = self.sys.used_swap();

        let disks = self.list_disks()?;
        let process_count = self.sys.processes().len();

        Ok(SystemStatus {
            hostname,
            os_name,
            os_version,
            kernel_version,
            platform: PlatformKind::Linux,
            uptime_seconds,
            cpu_count,
            cpu_usage_percent,
            memory_total_bytes,
            memory_used_bytes,
            memory_free_bytes,
            swap_total_bytes,
            swap_used_bytes,
            disks,
            process_count,
            timestamp: chrono::Utc::now(),
        })
    }

    fn list_processes(&mut self, limit: usize) -> Result<Vec<ProcessSummary>, PlatformError> {
        self.sys.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

        let mut procs: Vec<ProcessSummary> = self
            .sys
            .processes()
            .iter()
            .map(|(pid, proc_info)| ProcessSummary {
                pid: pid.as_u32(),
                name: proc_info.name().to_string_lossy().to_string(),
                cpu_usage: proc_info.cpu_usage(),
                memory_bytes: proc_info.memory(),
                status: format!("{:?}", proc_info.status()),
            })
            .collect();

        // Sort descending by CPU usage
        procs.sort_by(|a, b| {
            b.cpu_usage
                .partial_cmp(&a.cpu_usage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if limit > 0 && procs.len() > limit {
            procs.truncate(limit);
        }

        Ok(procs)
    }

    fn list_disks(&mut self) -> Result<Vec<DiskInfo>, PlatformError> {
        let mut list = Vec::new();
        for disk in &self.disks {
            list.push(DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_bytes: disk.total_space(),
                available_bytes: disk.available_space(),
                file_system: disk.file_system().to_string_lossy().to_string(),
            });
        }
        Ok(list)
    }

    fn get_supported_capabilities(&self) -> Vec<Capability> {
        vec![
            Capability {
                name: "system.get_status".to_string(),
                description: "Retrieve comprehensive host hardware, memory, CPU and OS status"
                    .to_string(),
                category: "system".to_string(),
                risk_level: RiskLevel::Low,
                supported_platforms: vec![
                    PlatformKind::Linux,
                    PlatformKind::Windows,
                    PlatformKind::MacOS,
                ],
                requires_confirmation: false,
                input_schema: None,
                output_schema: None,
            },
            Capability {
                name: "process.list".to_string(),
                description: "List currently running processes with CPU and memory usage"
                    .to_string(),
                category: "process".to_string(),
                risk_level: RiskLevel::Low,
                supported_platforms: vec![
                    PlatformKind::Linux,
                    PlatformKind::Windows,
                    PlatformKind::MacOS,
                ],
                requires_confirmation: false,
                input_schema: None,
                output_schema: None,
            },
            Capability {
                name: "agent.start".to_string(),
                description: "Launch a supervised AI agent CLI process in a restricted directory"
                    .to_string(),
                category: "agent".to_string(),
                risk_level: RiskLevel::Medium,
                supported_platforms: vec![
                    PlatformKind::Linux,
                    PlatformKind::Windows,
                    PlatformKind::MacOS,
                ],
                requires_confirmation: false,
                input_schema: None,
                output_schema: None,
            },
            Capability {
                name: "agent.stop".to_string(),
                description: "Gracefully or forcefully terminate a running AI agent task"
                    .to_string(),
                category: "agent".to_string(),
                risk_level: RiskLevel::Medium,
                supported_platforms: vec![
                    PlatformKind::Linux,
                    PlatformKind::Windows,
                    PlatformKind::MacOS,
                ],
                requires_confirmation: false,
                input_schema: None,
                output_schema: None,
            },
            Capability {
                name: "terminal.execute_command".to_string(),
                description: "Execute an approved system command inside the host environment"
                    .to_string(),
                category: "terminal".to_string(),
                risk_level: RiskLevel::High,
                supported_platforms: vec![
                    PlatformKind::Linux,
                    PlatformKind::Windows,
                    PlatformKind::MacOS,
                ],
                requires_confirmation: true,
                input_schema: None,
                output_schema: None,
            },
        ]
    }
}
