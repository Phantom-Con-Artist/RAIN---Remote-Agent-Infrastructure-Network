//! Local sysadmin operations. Remote operations are routed through RAIN workers.
use serde_json::Value;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

/// SSH execution has been retired. Remote work uses the authenticated cloud worker API.
pub fn ssh_target(_machine: &str) -> Result<String, String> {
    Err("SSH transport is disabled. Enroll a RAIN network worker and use the Cloud task/file controls.".into())
}

pub async fn operate(machine: Option<&str>, request: Value) -> Result<Value, String> {
    operate_timeout(machine, request, 20).await
}

pub async fn operate_timeout(
    machine: Option<&str>,
    request: Value,
    seconds: u64,
) -> Result<Value, String> {
    let source = include_str!("host_probe.py");
    let mut cmd = if let Some(machine) = machine.filter(|s| !s.is_empty()) {
        let target = ssh_target(machine)?;
        let mut c = Command::new(std::env::var("RAIN_SSH_BINARY").unwrap_or_else(|_| "ssh".into()));
        let remote = format!("python3 -c {}", crate::terminal::quote(source));
        c.args([
            "-o",
            "BatchMode=yes",
            "-o",
            "StrictHostKeyChecking=yes",
            "-o",
            "ConnectTimeout=10",
            "-T",
            "--",
            &target,
            &remote,
        ]);
        c
    } else {
        let mut c = Command::new("python3");
        c.args(["-c", source]);
        c
    };
    cmd.kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let bytes = serde_json::to_vec(&request).map_err(|e| e.to_string())?;
    tokio::time::timeout(std::time::Duration::from_secs(seconds), async {
        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Host operation needs Python 3 and SSH: {e}"))?;
        let mut stdin = child.stdin.take().ok_or("Missing stdin")?;
        let writer = tokio::spawn(async move { stdin.write_all(&bytes).await });
        let output = child.wait_with_output().await.map_err(|e| e.to_string())?;
        writer
            .await
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|_| {
            format!(
                "Host operation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )
        })?;
        if let Some(error) = response.get("error").and_then(Value::as_str) {
            return Err(error.to_owned());
        }
        if !output.status.success() {
            return Err("Host operation failed".into());
        }
        response
            .get("data")
            .cloned()
            .ok_or("Host returned invalid response".into())
    })
    .await
    .map_err(|_| "Host operation timed out".to_string())?
}
