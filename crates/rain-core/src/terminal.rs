//! Unix PTY allocation. The slave becomes the child's controlling terminal.
use std::{
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    process::Stdio,
};

#[cfg(unix)]
pub fn configure(cmd: &mut tokio::process::Command) -> Result<(File, File), String> {
    let mut master = -1;
    let mut slave = -1;
    let size = libc::winsize {
        ws_row: 32,
        ws_col: 120,
        ws_xpixel: 0,
        ws_ypixel: 0,
    };
    if unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            &size,
        )
    } != 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let master = unsafe { File::from_raw_fd(master) };
    let slave = unsafe { File::from_raw_fd(slave) };
    // Prevent descriptor inheritance into subsequent sessions.
    unsafe {
        libc::fcntl(master.as_raw_fd(), libc::F_SETFD, libc::FD_CLOEXEC);
    }
    cmd.stdin(Stdio::from(slave.try_clone().map_err(|e| e.to_string())?));
    cmd.stdout(Stdio::from(slave.try_clone().map_err(|e| e.to_string())?));
    cmd.stderr(Stdio::from(slave));
    unsafe {
        cmd.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            if libc::ioctl(0, libc::TIOCSCTTY, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    Ok((master.try_clone().map_err(|e| e.to_string())?, master))
}

pub fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}
