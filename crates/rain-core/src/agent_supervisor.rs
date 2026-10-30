use rain_protocol::{AgentEvent, AgentLaunchRequest, AgentProcess, AgentStatus};
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{broadcast, Mutex, RwLock};

#[derive(Clone)]
pub struct AgentSupervisor {
    agents: Arc<RwLock<HashMap<uuid::Uuid, AgentProcess>>>,
    inputs:
        Arc<RwLock<HashMap<uuid::Uuid, Arc<Mutex<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>>>>>,
    stop_requests: Arc<RwLock<HashSet<uuid::Uuid>>>,
    terminals: Arc<RwLock<HashMap<uuid::Uuid, Arc<std::fs::File>>>>,
    event_tx: broadcast::Sender<AgentEvent>,
}

impl AgentSupervisor {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(500);
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            stop_requests: Arc::new(RwLock::new(HashSet::new())),
            terminals: Arc::new(RwLock::new(HashMap::new())),
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

        let chat_mode = matches!(
            req.provider.as_str(),
            "codex_chat" | "claude_chat" | "agy_chat" | "muse_chat"
        );
        let command = if chat_mode { "python3" } else { &req.command };
        let args = if chat_mode {
            vec![
                "-u".into(),
                "-c".into(),
                include_str!("chat_adapter.py").into(),
                req.provider.clone(),
                req.command.clone(),
            ]
        } else {
            req.args.clone()
        };
        let interactive = req.interactive && !chat_mode;
        let mut cmd = if let Some(machine) = &req.machine {
            let target = crate::host_access::ssh_target(machine)?;
            let mut remote = format!("cd {} && exec ", crate::terminal::quote(&cwd));
            remote.push_str(&crate::terminal::quote(command));
            for arg in &args {
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
                if interactive { "-tt" } else { "-T" },
                "--",
                &target,
                &remote,
            ]);
            c
        } else {
            let mut c = tokio::process::Command::new(command);
            c.args(&args).current_dir(&cwd).envs(&req.env_vars);
            c
        };
        if interactive {
            cmd.env("TERM", "xterm-256color");
        }
        #[cfg(unix)]
        if !interactive {
            cmd.process_group(0);
        }
        cmd.kill_on_drop(true);
        let terminal = if interactive {
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
        let pid = child.id();
        let started_at = chrono::Utc::now();

        let mut initial_process = AgentProcess {
            id,
            name: req.name.clone(),
            provider: req.provider.clone(),
            machine: req.machine.clone(),
            interactive,
            pid,
            command: req.command.clone(),
            args: req.args.clone(),
            cwd: cwd.clone(),
            status: AgentStatus::Running,
            exit_code: None,
            started_at,
            finished_at: None,
            recent_logs: Vec::new(),
            output_sequence: 0,
            activity: None,
        };

        {
            let mut lock = self.agents.write().await;
            let base = initial_process.name.clone();
            let mut suffix = 2;
            while lock
                .values()
                .any(|agent| agent.name.eq_ignore_ascii_case(&initial_process.name))
            {
                initial_process.name = format!("{}-{}", base, suffix);
                suffix += 1;
            }
            lock.insert(id, initial_process.clone());
        }

        if let Some((mut reader, writer)) = terminal {
            self.terminals
                .write()
                .await
                .insert(id, Arc::new(writer.try_clone().map_err(|e| e.to_string())?));
            self.inputs.write().await.insert(
                id,
                Arc::new(Mutex::new(Box::new(tokio::fs::File::from_std(writer)))),
            );
            let tx = self.event_tx.clone();
            let agents = self.agents.clone();
            tokio::task::spawn_blocking(move || {
                use std::io::Read;
                let mut buffer = [0u8; 4096];
                let mut pending = Vec::new();
                while let Ok(n) = reader.read(&mut buffer) {
                    if n == 0 {
                        break;
                    }
                    pending.extend_from_slice(&buffer[..n]);
                    let valid = match std::str::from_utf8(&pending) {
                        Ok(_) => pending.len(),
                        Err(e) => e.valid_up_to(),
                    };
                    if valid == 0 {
                        if pending.len() > 4096 {
                            pending.clear();
                        }
                        continue;
                    }
                    let line = String::from_utf8_lossy(&pending[..valid]).to_string();
                    pending.drain(..valid);
                    if let Some(agent) = agents.blocking_write().get_mut(&id) {
                        if agent.recent_logs.len() >= 1000 {
                            agent.recent_logs.remove(0);
                        }
                        agent.recent_logs.push(line.clone());
                        agent.output_sequence += 1;
                    }
                    let _ = tx.send(AgentEvent::Output {
                        agent_id: id,
                        stream: "terminal".into(),
                        line,
                    });
                }
            });
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
        let terminals = self.terminals.clone();
        let stop_requests = self.stop_requests.clone();

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
                        if chat_mode {
                            if let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) {
                                match event.get("rain_chat").and_then(|v| v.as_str()) {
                                    Some("reply") => {
                                        if let (Some(conversation), Some(text)) = (event.get("conversation_id").and_then(|v| v.as_str()).and_then(|v| uuid::Uuid::parse_str(v).ok()), event.get("text").and_then(|v| v.as_str())) {
                                            let run_id=event.get("run_id").and_then(|v|v.as_str()).and_then(|v|uuid::Uuid::parse_str(v).ok());
                                            let _ = tx.send(AgentEvent::ChatReplyScoped {agent_id:id,conversation_id:conversation,run_id,text:text.into()});
                                        }
                                    },
                                    Some("trace") => {
                                        if let (Some(dispatch_id),Some(step))=(event["dispatch_id"].as_str().and_then(|v|uuid::Uuid::parse_str(v).ok()),event["step"].as_str()) {let _=tx.send(AgentEvent::Trace{agent_id:id,dispatch_id,step:step.chars().take(500).collect()});}
                                    },
                                    Some("turn") => {
                                        if let (Some(dispatch_id),Some(state))=(event["dispatch_id"].as_str().and_then(|v|uuid::Uuid::parse_str(v).ok()),event["state"].as_str()){
                                            let _=tx.send(AgentEvent::TurnActivity{agent_id:id,dispatch_id,state:state.into(),error:event["error"].as_str().map(str::to_owned)});
                                        }
                                    },
                                    Some("activity") => {
                                        let state = if event["state"] == "working" { rain_protocol::SessionState::Working } else { rain_protocol::SessionState::Online };
                                        if let Some(agent) = agents_ref.write().await.get_mut(&id) {agent.activity = Some(state.clone());}
                                        let _ = tx.send(AgentEvent::ActivityChanged {agent_id:id,state});
                                    }, _ => (),
                                }
                            }
                            continue;
                        }
                        let mut lock = agents_ref.write().await;
                        if let Some(agent) = lock.get_mut(&id) {
                            if agent.recent_logs.len() > 1000 {
                                agent.recent_logs.remove(0);
                            }
                            agent.recent_logs.push(format!("[stdout] {}\r\n", line));
                            agent.output_sequence += 1;
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
                            agent.recent_logs.push(format!("[stderr] {}\r\n", line));
                            agent.output_sequence += 1;
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
                        terminals.write().await.remove(&id);
                        let requested = stop_requests.write().await.remove(&id);
                        let final_status = if requested { AgentStatus::Terminated } else { match exit_code {
                            Some(0) => AgentStatus::Completed,
                            _ => AgentStatus::Failed,
                        }};

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
        let interactive = self
            .get_agent(id)
            .await
            .ok_or("Session not found")?
            .interactive;
        self.send_raw(
            id,
            &format!("{}{}", input, if interactive { "\r" } else { "\n" }),
        )
        .await
    }

    pub async fn send_chat_input(
        &self,
        id: uuid::Uuid,
        text: &str,
        conversation: uuid::Uuid,
    ) -> Result<(), String> {
        self.send_context_input(id, text, conversation, None, true)
            .await
    }

    pub async fn send_context_input(
        &self,
        id: uuid::Uuid,
        text: &str,
        conversation: uuid::Uuid,
        run_id: Option<uuid::Uuid>,
        human: bool,
    ) -> Result<(), String> {
        self.send_tracked_input(
            id,
            text,
            conversation,
            run_id,
            human,
            uuid::Uuid::new_v4(),
            None,
        )
        .await
    }
    pub async fn send_tracked_input(
        &self,
        id: uuid::Uuid,
        text: &str,
        conversation: uuid::Uuid,
        run_id: Option<uuid::Uuid>,
        human: bool,
        dispatch_id: uuid::Uuid,
        options: Option<rain_protocol::NativeChatSettings>,
    ) -> Result<(), String> {
        let agent = self.get_agent(id).await.ok_or("Session not found")?;
        if agent.provider.ends_with("_chat") {
            let frame = serde_json::json!({"text":text,"conversation_id":conversation,"run_id":run_id,"dispatch_id":dispatch_id,"options":options.unwrap_or_default(),"origin":if human {"human"}else{"agent"}});
            self.send_raw(id, &format!("{}\n", frame)).await
        } else {
            self.send_input(id, text).await
        }
    }

    pub async fn interrupt_autonomy(
        &self,
        id: uuid::Uuid,
        conversation: uuid::Uuid,
    ) -> Result<(), String> {
        if self
            .get_agent(id)
            .await
            .map(|a| a.provider.ends_with("_chat"))
            .unwrap_or(false)
        {
            self.send_raw(
                id,
                &format!(
                    "{}\n",
                    serde_json::json!({"control":"cancel_auto","conversation_id":conversation})
                ),
            )
            .await?;
        }
        Ok(())
    }

    pub async fn cancel_chat_conversation(
        &self,
        id: uuid::Uuid,
        conversation: uuid::Uuid,
    ) -> Result<(), String> {
        let agent = self.get_agent(id).await.ok_or("Session not found")?;
        if agent.provider.ends_with("_chat") {
            self.send_raw(
                id,
                &format!(
                    "{}\n",
                    serde_json::json!({"control":"cancel","conversation_id":conversation})
                ),
            )
            .await?;
        }
        Ok(())
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

    pub async fn resize_terminal(
        &self,
        id: uuid::Uuid,
        cols: u16,
        rows: u16,
    ) -> Result<(), String> {
        if !(2..=500).contains(&cols) || !(1..=200).contains(&rows) {
            return Err("Invalid terminal dimensions".into());
        }
        #[cfg(unix)]
        {
            use std::os::fd::AsRawFd;
            let terminals = self.terminals.read().await;
            let file = terminals.get(&id).ok_or("Session has no active terminal")?;
            let size = libc::winsize {
                ws_row: rows,
                ws_col: cols,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            if unsafe { libc::ioctl(file.as_raw_fd(), libc::TIOCSWINSZ, &size) } < 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            Err("Terminal resizing requires Unix".into())
        }
    }

    pub async fn remove_agent(&self, id: uuid::Uuid) -> Result<AgentProcess, String> {
        let mut agents = self.agents.write().await;
        let agent = agents.get(&id).ok_or("Session not found")?;
        if matches!(agent.status, AgentStatus::Starting | AgentStatus::Running) {
            return Err("Stop this session before removing it".into());
        }
        agents.remove(&id).ok_or("Session not found".into())
    }

    pub async fn stop_agent(&self, id: uuid::Uuid) -> Result<(), String> {
        let agent = self.get_agent(id).await.ok_or("Session not found")?;
        if agent.status != AgentStatus::Running {
            return Err("Session is no longer running".into());
        }
        let pid = agent.pid.ok_or("Session has no active PID")?;
        #[cfg(unix)]
        {
            self.stop_requests.write().await.insert(id);
            let target = -(pid as i32);
            if unsafe { libc::kill(target, libc::SIGTERM) } != 0 {
                self.stop_requests.write().await.remove(&id);
                return Err(std::io::Error::last_os_error().to_string());
            }
            tokio::time::timeout(std::time::Duration::from_secs(3), async {
                loop {
                    if self
                        .get_agent(id)
                        .await
                        .map(|a| a.status != AgentStatus::Running)
                        .unwrap_or(true)
                    {
                        return;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                }
            })
            .await
            .map_err(|_| {
                "Stop requested, but the process has not exited. It remains listed as running."
                    .to_string()
            })?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = pid;
            Err("Stopping sessions currently requires a Unix host".into())
        }
    }
}

impl Default for AgentSupervisor {
    fn default() -> Self {
        Self::new()
    }
}
