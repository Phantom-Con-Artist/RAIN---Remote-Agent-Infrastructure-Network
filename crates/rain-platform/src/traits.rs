use crate::error::PlatformError;
use rain_protocol::{Capability, DiskInfo, PlatformKind, ProcessSummary, SystemStatus};

pub trait PlatformAdapter {
    fn name(&self) -> &'static str;
    fn kind(&self) -> PlatformKind;
    fn refresh(&mut self);
    fn get_system_status(&mut self) -> Result<SystemStatus, PlatformError>;
    fn list_processes(&mut self, limit: usize) -> Result<Vec<ProcessSummary>, PlatformError>;
    fn list_disks(&mut self) -> Result<Vec<DiskInfo>, PlatformError>;
    fn get_supported_capabilities(&self) -> Vec<Capability>;
}
