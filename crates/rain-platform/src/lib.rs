pub mod error;
pub mod linux;
pub mod traits;

pub use error::PlatformError;
pub use linux::LinuxAdapter;
pub use traits::PlatformAdapter;

pub fn current_platform_adapter() -> Box<dyn PlatformAdapter + Send + Sync> {
    #[cfg(target_os = "linux")]
    {
        Box::new(LinuxAdapter::new())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Box::new(LinuxAdapter::new())
    }
}
