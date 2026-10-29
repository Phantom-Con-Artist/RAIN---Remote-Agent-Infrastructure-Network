use rain_protocol::{AgentEvent, AgentLaunchRequest, AgentProcess, AgentStatus};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex, RwLock};

#[derive(Clone)]
pub struct AgentSupervisor {
    agents: Arc<RwLock<HashMap<uuid::Uuid, AgentProcess>>>,
    inputs:
        Arc<RwLock<HashMap<uuid::Uuid, Arc<Mutex<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>>>>>,
    event_tx: broadcast::Sender<AgentEvent>,
}

impl AgentSupervisor {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(500);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            inputs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub(crate) async fn restore_record(&self, record: AgentProcess) {
        self.agents.write().await.insert(record.id, record);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AgentEvent> {
        self.event_tx.subscribe()
    }

    pub async fn list_agents(&self) -> Vec<AgentProcess> {
        let lock = self.agents.read().await;
        lock.values().cloned().collect()
    }

    pub async fn get_agent(&self, id: uuid::Uuid) -> Option<AgentProcess> {
        let lock = self.agents.read().await;
        lock.get(&id).cloned()
    }

    pub async fn start_agent(&self, req: AgentLaunchRequest) -> Result<AgentProcess, String> {
        let id = uuid::Uuid::new_v4();
        let cwd = req.cwd.clone().unwrap_or_else(|| {
            if req.machine.is_some() {
                ".".into()
            } else {
                std::env::current_dir()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            }
        });

        let mut cmd = if let Some(machine) = &req.machine {
            let hosts: HashMap<String, String> = serde_json::from_str(
                &std::env::var("RAIN_SSH_HOSTS").unwrap_or_else(|_| "{}".into()),
            )
            .map_err(|e| e.to_string())?;
            let target = hosts
                .get(machine)
                .ok_or("Machine is not configured in RAIN_SSH_HOSTS")?;
            if target.starts_with('-') {
                return Err("Invalid SSH target".into());
            }
            let mut remote = format!("cd {} && exec ", crate::terminal::quote(&cwd));
            remote.push_str(&crate::terminal::quote(&req.command));
            for arg in &req.args {
                remote.push(' ');
                remote.push_str(&crate::terminal::quote(arg));
            }
            if !req.env_vars.is_empty() {
                return Err("Remote environment overrides are not supported yet".into());
            }
            let mut c = tokio::process::Command::new("ssh");
            c.args([
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=yes",
                if req.interactive { "-tt" } else { "-T" },
                "--",
                target,
                &remote,
            ]);
            c
        } else {
            let mut c = tokio::process::Command::new(&req.command);
            c.args(&req.args).current_dir(&cwd).envs(&req.env_vars);
            c
        };
        cmd.kill_on_drop(true);
        let terminal = if req.interactive {
            #[cfg(unix)]
            {
                Some(crate::terminal::configure(&mut cmd)?)
            }
            #[cfg(not(unix))]
            {
                return Err("Interactive sessions currently require a Unix host".into());
            }
        } else {
            cmd.stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            None
        };

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{}': {}", req.command, e))?;

        if let Some(stdin) = child.stdin.take() {
            self.inputs
                .write()
                .await
                .insert(id, Arc::new(Mutex::new(Box::new(stdin))));
        }
        if let Some((mut reader, writer)) = terminal {
            self.inputs.write().await.insert(
                id,
                Arc::new(Mutex::new(Box::new(tokio::fs::File::from_std(writer)))),
            );
            let tx = self.event_tx.clone();
            let agents = self.agents.clone();
            tokio::task::spawn_blocking(move || {
                use std::io::Read;
                let mut buffer = [0u8; 4096];
                while let Ok(n) = reader.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    let line = String::from_utf8_lossy(&buffer[..n]).to_string();
                    if let Some(agent) = agents.blocking_write().get_mut(&id) {
                        if agent.recent_logs.len() >= 1000 {
                            agent.recent_logs.remove(0);
                        }
                        agent.recent_logs.push(line.clone());
                    }
                    let _ = tx.send(AgentEvent::Output {
                        agent_id: id,
                        stream: "terminal".into(),
                        line,
                    });
                }
            });
        }
        let pid = child.id();
        let started_at = chrono::Utc::now();

        let initial_process = AgentProcess {
            id,
            name: req.name.clone(),
            provider: req.provider.clone(),
            machine: req.machine.clone(),
            interactive: req.interactive,
            pid,
            command: req.command.clone(),
            args: req.args.clone(),
            cwd: cwd.clone(),
            status: AgentStatus::Running,
            exit_code: None,
            started_at,
            finished_at: None,
            recent_logs: Vec::new(),
        };

        {
            let mut lock = self.agents.write().await;
            lock.insert(id, initial_process.clone());
        }

        let _ = self.event_tx.send(AgentEvent::StatusChanged {
            agent_id: id,
            status: AgentStatus::Running,
            exit_code: None,
        });

        // Spawn background task to read stdout and stderr
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let agents_ref = self.agents.clone();
        let tx = self.event_tx.clone();
        let inputs = self.inputs.clone();

        tokio::spawn(async move {
            let mut stdout_reader = stdout.map(|s| BufReader::new(s).lines());
            let mut stderr_reader = stderr.map(|s| BufReader::new(s).lines());

            loop {
                tokio::select! {
                    Some(line_res) = async {
                        if let Some(r) = &mut stdout_reader {
                            r.next_line().await.ok().flatten()
                        } else {
                            None
                        }
                    } => {
                        let line = line_res;
                        let mut lock = agents_ref.write().await;
                        if let Some(agent) = lock.get_mut(&id) {
                            if agent.recent_logs.len() > 1000 {
                                agent.recent_logs.remove(0);
                            }
                            agent.recent_logs.push(format!("[stdout] {}", line));
                        }
                        let _ = tx.send(AgentEvent::Output {
                            agent_id: id,
                            stream: "stdout".to_string(),
                            line,
                        });
                    }
                    Some(line_res) = async {
                        if let Some(r) = &mut stderr_reader {
                            r.next_line().await.ok().flatten()
                        } else {
                            None
                        }
                    } => {
                        let line = line_res;
                        let mut lock = agents_ref.write().await;
                        if let Some(agent) = lock.get_mut(&id) {
                            if agent.recent_logs.len() > 1000 {
                                agent.recent_logs.remove(0);
                            }
                            agent.recent_logs.push(format!("[stderr] {}", line));
                        }
                        let _ = tx.send(AgentEvent::Output {
                            agent_id: id,
                            stream: "stderr".to_string(),
                            line,
                        });
                    }
                    status = child.wait() => {
                        let exit_code = match status {
                            Ok(s) => s.code(),
                            Err(_) => Some(-1),
                        };

                        inputs.write().await.remove(&id);
                        let final_status = match exit_code {
                            Some(0) => AgentStatus::Completed,
                            _ => AgentStatus::Failed,
                        };

                        let mut lock = agents_ref.write().await;
                        if let Some(agent) = lock.get_mut(&id) {
                            if agent.status != AgentStatus::Terminated { agent.status = final_status.clone(); }
                            agent.exit_code = exit_code;
                            agent.finished_at = Some(chrono::Utc::now());
                        }

                        let _ = tx.send(AgentEvent::StatusChanged {
                            agent_id: id,
                            status: final_status,
                            exit_code,
                        });
                        break;
                    }
                }
            }
        });

        Ok(initial_process)
    }

    pub async fn send_input(&self, id: uuid::Uuid, input: &str) -> Result<(), String> {
        self.send_raw(id, &format!("{}\n", input)).await
    }

    pub async fn send_raw(&self, id: uuid::Uuid, input: &str) -> Result<(), String> {
        let stdin = self
            .inputs
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or("Session has no active input")?;
        let mut stdin = stdin.lock().await;
        stdin
            .write_all(input.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stdin.flush().await.map_err(|e| e.to_string())
    }

    pub async fn stop_agent(&self, id: uuid::Uuid) -> Result<(), String> {
        let mut lock = self.agents.write().await;
        if let Some(agent) = lock.get_mut(&id) {
            if agent.status != AgentStatus::Running {
                return Err("Session is no longer running".into());
            }
            if let Some(pid) = agent.pid {
                #[cfg(unix)]
                {
                    unsafe {
                        libc::kill(pid as i32, libc::SIGTERM);
                    }
                }
                agent.status = AgentStatus::Terminated;
                agent.finished_at = Some(chrono::Utc::now());

                let _ = self.event_tx.send(AgentEvent::StatusChanged {
                    agent_id: id,
                    status: AgentStatus::Terminated,
                    exit_code: Some(143),
                });
                return Ok(());
            }
        }
        Err("Agent not found or has no active PID".to_string())
    }
}

impl Default for AgentSupervisor {
    fn default() -> Self {
        Self::new()
    }
}
