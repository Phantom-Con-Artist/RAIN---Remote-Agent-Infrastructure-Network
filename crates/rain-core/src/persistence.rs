use crate::{AgentSupervisor, ChatStore};
use rain_protocol::{AgentProcess, AgentStatus, ChatMessage, Conversation, JobDelegation};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Serialize, Deserialize)]
struct Snapshot {
    version: u32,
    conversations: HashMap<uuid::Uuid, Conversation>,
    messages: HashMap<uuid::Uuid, Vec<ChatMessage>>,
    jobs: HashMap<uuid::Uuid, JobDelegation>,
    agents: Vec<AgentProcess>,
    #[serde(default)]
    dispatches: Vec<rain_protocol::ChatDispatch>,
    #[serde(default)]
    traces: Vec<rain_protocol::AgentTrace>,
    #[serde(default)]
    group_memories: HashMap<uuid::Uuid, rain_protocol::GroupMemory>,
}
impl ChatStore {
    pub async fn restore(supervisor: AgentSupervisor, path: &Path) -> Result<Self, String> {
        let mut store = Self::with_supervisor(supervisor);
        if std::env::var("RAIN_ARTIFACT_DIR").is_err() {
            store.artifacts = crate::artifacts::ArtifactStore::at(
                path.parent().unwrap_or(Path::new(".")).join("artifacts"),
            );
        }
        match tokio::fs::read(path).await {
            Ok(bytes) => {
                let snapshot: Snapshot = serde_json::from_slice(&bytes)
                    .map_err(|e| format!("Cannot load history: {e}"))?;
                if snapshot.version != 1 {
                    return Err("Unsupported history version".into());
                }
                *store.conversations.write().await = snapshot.conversations;
                *store.messages.write().await = snapshot.messages;
                *store.jobs.write().await = snapshot.jobs;
                let mut dispatches = snapshot.dispatches;
                for dispatch in &mut dispatches {
                    if matches!(dispatch.status.as_str(), "queued" | "running") {
                        dispatch.status = "interrupted".into();
                        dispatch.finished_at = Some(chrono::Utc::now());
                    }
                }
                *store.dispatches.write().await = dispatches;
                *store.traces.write().await = snapshot.traces;
                *store.group_memories.write().await = snapshot.group_memories;
                for mut agent in snapshot.agents {
                    if matches!(agent.status, AgentStatus::Running | AgentStatus::Starting) {
                        agent.status = AgentStatus::Terminated;
                        agent.finished_at = Some(chrono::Utc::now());
                        agent.recent_logs.push("Daemon restarted; this process is no longer attached. Launch a new session to continue.".into());
                    }
                    agent.pid = None;
                    store.supervisor.restore_record(agent).await;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            Err(e) => return Err(e.to_string()),
        }
        Ok(store)
    }

    pub async fn persist(&self, path: &Path) -> Result<(), String> {
        let _serial = self.persistence_lock.lock().await;
        let conversations = self.conversations.read().await;
        let messages = self.messages.read().await;
        let jobs = self.jobs.read().await;
        let snapshot = Snapshot {
            version: 1,
            conversations: conversations.clone(),
            messages: messages.clone(),
            jobs: jobs.clone(),
            dispatches: self.dispatches.read().await.clone(),
            traces: self.traces.read().await.clone(),
            group_memories: self.group_memories.read().await.clone(),
            agents: self.supervisor.list_agents().await,
        };
        let bytes = serde_json::to_vec(&snapshot).map_err(|e| e.to_string())?;
        drop(jobs);
        drop(messages);
        drop(conversations);
        let path = path.to_owned();
        tokio::task::spawn_blocking(move || -> std::io::Result<()> {
            use std::io::Write;
            let parent = path
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            std::fs::create_dir_all(parent)?;
            let temp = path.with_extension("json.tmp");
            let mut options = std::fs::OpenOptions::new();
            options.write(true).create(true).truncate(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            let mut file = options.open(&temp)?;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temp, &path)?;
            std::fs::File::open(parent)?.sync_all()?;
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
    }
}
