use rain_protocol::{AgentEvent, AgentLaunchRequest, AgentProcess, AgentStatus};
use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct AgentSupervisor {
    agents: Arc<RwLock<HashMap<uuid::Uuid, AgentProcess>>>,
    event_tx: broadcast::Sender<AgentEvent>,
}

impl AgentSupervisor {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(500);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
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
        let cwd = req
            .cwd
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap().to_string_lossy().to_string());

        let mut cmd = tokio::process::Command::new(&req.command);
        cmd.args(&req.args);
        cmd.current_dir(&cwd);
        for (k, v) in &req.env_vars {
            cmd.env(k, v);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn process '{}': {}", req.command, e))?;

        let pid = child.id();
        let started_at = chrono::Utc::now();

        let initial_process = AgentProcess {
            id,
            name: req.name.clone(),
            provider: req.provider.clone(),
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

                        let final_status = match exit_code {
                            Some(0) => AgentStatus::Completed,
                            _ => AgentStatus::Failed,
                        };

                        let mut lock = agents_ref.write().await;
                        if let Some(agent) = lock.get_mut(&id) {
                            agent.status = final_status.clone();
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

    pub async fn stop_agent(&self, id: uuid::Uuid) -> Result<(), String> {
        let mut lock = self.agents.write().await;
        if let Some(agent) = lock.get_mut(&id) {
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
