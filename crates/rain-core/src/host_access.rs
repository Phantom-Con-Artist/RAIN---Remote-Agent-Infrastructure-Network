//! Host operations use the same SSH allowlist and host-key policy as agent execution.
use serde_json::Value;
use std::{collections::HashMap, process::Stdio};
use tokio::{io::AsyncWriteExt, process::Command};

pub fn ssh_target(machine: &str) -> Result<String, String> {
    let hosts: HashMap<String, String> =
        serde_json::from_str(&std::env::var("RAIN_SSH_HOSTS").unwrap_or_else(|_| "{}".into()))
            .map_err(|e| e.to_string())?;
    let target = hosts
        .get(machine)
        .ok_or("Machine is not configured in RAIN_SSH_HOSTS")?;
    if target.is_empty() || target.starts_with('-') || target.contains('\0') {
        return Err("Invalid SSH target".into());
    }
    Ok(target.clone())
}

pub async fn operate(machine: Option<&str>, request: Value) -> Result<Value, String> {
    let source = include_str!("host_probe.py");
    let mut cmd = if let Some(machine) = machine.filter(|s| !s.is_empty()) {
        let target = ssh_target(machine)?;
        let mut c = Command::new("ssh");
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
    tokio::time::timeout(std::time::Duration::from_secs(20), async {
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
