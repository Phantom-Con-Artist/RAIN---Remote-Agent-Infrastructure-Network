use rain_protocol::{
    AgentSession, ChatAttachment, ChatMessage, Conversation, JobDelegation, JobStatus,
    MessageDeliveryStatus, SenderType, SessionState,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ChatStore {
    conversations: Arc<RwLock<HashMap<uuid::Uuid, Conversation>>>,
    messages: Arc<RwLock<HashMap<uuid::Uuid, Vec<ChatMessage>>>>,
    sessions: Arc<RwLock<HashMap<String, AgentSession>>>,
    jobs: Arc<RwLock<HashMap<uuid::Uuid, JobDelegation>>>,
}

impl ChatStore {
    pub fn new() -> Self {
        let mut convs = HashMap::new();
        let mut msgs = HashMap::new();
        let mut sessions = HashMap::new();
        let mut jobs = HashMap::new();

        // 1. Initial AI Agent Sessions (across machines)
        let s1 = AgentSession {
            id: "codex-frontend".to_string(),
            name: "Codex — Frontend".to_string(),
            provider: "codex".to_string(),
            machine_name: "Office-PC-Linux".to_string(),
            state: SessionState::Online,
            cwd: "~/projects/web-frontend".to_string(),
            subagents: vec!["UI-Builder".to_string(), "Styling-Agent".to_string()],
        };
        let s2 = AgentSession {
            id: "codex-backend".to_string(),
            name: "Codex — Backend".to_string(),
            provider: "codex".to_string(),
            machine_name: "Home-Laptop-Linux".to_string(),
            state: SessionState::Online,
            cwd: "~/projects/web-api".to_string(),
            subagents: vec!["DB-Migrator".to_string()],
        };
        let s3 = AgentSession {
            id: "claude-reviewer".to_string(),
            name: "Claude Code — Reviewer".to_string(),
            provider: "claude_code".to_string(),
            machine_name: "Office-Desktop-Mac".to_string(),
            state: SessionState::Online,
            cwd: "~/projects/review-audit".to_string(),
            subagents: vec![],
        };

        sessions.insert(s1.id.clone(), s1);
        sessions.insert(s2.id.clone(), s2);
        sessions.insert(s3.id.clone(), s3);

        // 2. Initial Group Chat: "Website Launch Team"
        let group_id = uuid::Uuid::new_v4();
        let group_conv = Conversation {
            id: group_id,
            title: "Website Launch Team".to_string(),
            is_group: true,
            is_active: true,
            participant_ids: vec![
                "human-user".to_string(),
                "codex-frontend".to_string(),
                "codex-backend".to_string(),
                "claude-reviewer".to_string(),
            ],
            last_message: Some("Claude Code: Architecture review passed. Ready for deployment.".to_string()),
            last_message_time: Some(chrono::Utc::now() - chrono::Duration::minutes(5)),
            unread_count: 0,
        };

        let initial_msgs = vec![
            ChatMessage {
                id: uuid::Uuid::new_v4(),
                conversation_id: group_id,
                sender_id: "human-user".to_string(),
                sender_name: "You".to_string(),
                sender_type: SenderType::Human,
                text: "Hey team, let's prepare the v1.0 release. Frontend and Backend status check please.".to_string(),
                status: MessageDeliveryStatus::Read,
                attachments: vec![],
                job_delegation: None,
                timestamp: chrono::Utc::now() - chrono::Duration::minutes(15),
            },
            ChatMessage {
                id: uuid::Uuid::new_v4(),
                conversation_id: group_id,
                sender_id: "codex-frontend".to_string(),
                sender_name: "Codex — Frontend".to_string(),
                sender_type: SenderType::Agent,
                text: "Frontend client build is clean. Responsive viewports and touch targets verified. Subagents finished bundle optimization.".to_string(),
                status: MessageDeliveryStatus::Read,
                attachments: vec![ChatAttachment {
                    id: "att-1".to_string(),
                    name: "bundle-report.json".to_string(),
                    mime_type: "application/json".to_string(),
                    size_bytes: 4096,
                    preview_content: Some("{\n  \"gzip_size\": \"9.4 kB\",\n  \"modules\": 4\n}".to_string()),
                    data_base64: None,
                }],
                job_delegation: None,
                timestamp: chrono::Utc::now() - chrono::Duration::minutes(12),
            },
            ChatMessage {
                id: uuid::Uuid::new_v4(),
                conversation_id: group_id,
                sender_id: "codex-backend".to_string(),
                sender_name: "Codex — Backend".to_string(),
                sender_type: SenderType::Agent,
                text: "API daemon running on port 7890. SQLite schema migrated and capability routes secured.".to_string(),
                status: MessageDeliveryStatus::Read,
                attachments: vec![],
                job_delegation: None,
                timestamp: chrono::Utc::now() - chrono::Duration::minutes(10),
            },
            ChatMessage {
                id: uuid::Uuid::new_v4(),
                conversation_id: group_id,
                sender_id: "claude-reviewer".to_string(),
                sender_name: "Claude Code — Reviewer".to_string(),
                sender_type: SenderType::Agent,
                text: "Architecture review passed. Ready for deployment.".to_string(),
                status: MessageDeliveryStatus::Read,
                attachments: vec![],
                job_delegation: None,
                timestamp: chrono::Utc::now() - chrono::Duration::minutes(5),
            },
        ];

        // Seed an initial completed job delegation for demonstration
        let demo_job_id = uuid::Uuid::new_v4();
        let demo_job = JobDelegation {
            id: demo_job_id,
            source_agent_id: "codex-backend".to_string(),
            source_machine: "Home-Laptop-Linux".to_string(),
            target_agent_id: "claude-reviewer".to_string(),
            target_machine: "Office-Desktop-Mac".to_string(),
            task_instruction: "Compile CUDA release binary on Home Laptop and run test suite on Office Mac".to_string(),
            required_resources: vec!["rtx-4090-gpu".to_string(), "64-core-cpu".to_string()],
            status: JobStatus::Completed,
            progress_percent: 100,
            shared_artifacts: vec![ChatAttachment {
                id: "art-delegated-1".to_string(),
                name: "release-v0.2-bin.tar.gz".to_string(),
                mime_type: "application/gzip".to_string(),
                size_bytes: 48 * 1024 * 1024,
                preview_content: Some("[Compressed Binary: 48 MB, sha256: 8f921a4e...]".to_string()),
                data_base64: None,
            }],
            requires_sudo: false,
            sudo_command: None,
            is_approved: true,
            result_summary: Some("Binary compiled with GPU CUDA acceleration on Home Laptop, streamed P2P to Office Mac, all 32 tests passed.".to_string()),
            created_at: chrono::Utc::now() - chrono::Duration::minutes(20),
        };
        jobs.insert(demo_job_id, demo_job);

        convs.insert(group_id, group_conv);
        msgs.insert(group_id, initial_msgs);

        Self {
            conversations: Arc::new(RwLock::new(convs)),
            messages: Arc::new(RwLock::new(msgs)),
            sessions: Arc::new(RwLock::new(sessions)),
            jobs: Arc::new(RwLock::new(jobs)),
        }
    }

    pub async fn list_conversations(&self) -> Vec<Conversation> {
        let lock = self.conversations.read().await;
        let mut list: Vec<Conversation> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.last_message_time.cmp(&a.last_message_time));
        list
    }

    pub async fn get_messages(&self, conv_id: uuid::Uuid) -> Vec<ChatMessage> {
        let lock = self.messages.read().await;
        lock.get(&conv_id).cloned().unwrap_or_default()
    }

    pub async fn list_sessions(&self) -> Vec<AgentSession> {
        let lock = self.sessions.read().await;
        lock.values().cloned().collect()
    }

    pub async fn list_jobs(&self) -> Vec<JobDelegation> {
        let lock = self.jobs.read().await;
        let mut list: Vec<JobDelegation> = lock.values().cloned().collect();
        list.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        list
    }

    pub async fn approve_job(
        &self,
        job_id: uuid::Uuid,
        approved: bool,
        sudo_password: Option<String>,
    ) -> Result<JobDelegation, String> {
        let mut jobs_lock = self.jobs.write().await;
        let job = jobs_lock.get_mut(&job_id).ok_or_else(|| "Job not found".to_string())?;

        if !approved {
            job.status = JobStatus::Failed;
            job.result_summary = Some("Job rejected by human supervisor on mobile.".to_string());
            return Ok(job.clone());
        }

        job.is_approved = true;
        job.status = JobStatus::Running;
        job.progress_percent = 75;

        let pwd_provided = sudo_password.is_some();
        let target_machine = job.target_machine.clone();
        let cmd = job.sudo_command.clone().unwrap_or_else(|| "task".to_string());

        job.status = JobStatus::Completed;
        job.progress_percent = 100;
        job.result_summary = Some(format!(
            "Authorized by supervisor {}. Executed `{}` on {} with exit code 0.",
            if pwd_provided { "with credentials" } else { "with biometrics" },
            cmd,
            target_machine
        ));

        Ok(job.clone())
    }

    pub async fn create_conversation(
        &self,
        title: String,
        is_group: bool,
        mut participant_ids: Vec<String>,
    ) -> Conversation {
        let new_id = uuid::Uuid::new_v4();
        if !participant_ids.contains(&"human-user".to_string()) {
            participant_ids.insert(0, "human-user".to_string());
        }

        let conv = Conversation {
            id: new_id,
            title: title.clone(),
            is_group,
            is_active: true,
            participant_ids: participant_ids.clone(),
            last_message: Some(if is_group {
                format!("Group created with {} members", participant_ids.len())
            } else {
                "Direct chat started".to_string()
            }),
            last_message_time: Some(chrono::Utc::now()),
            unread_count: 0,
        };

        let initial_system_msg = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: new_id,
            sender_id: "system".to_string(),
            sender_name: "RAIN System".to_string(),
            sender_type: SenderType::System,
            text: if is_group {
                format!("Group '{}' established. Agents connected across machines.", title)
            } else {
                format!("Direct session connected with {}.", title)
            },
            status: MessageDeliveryStatus::Read,
            attachments: vec![],
            job_delegation: None,
            timestamp: chrono::Utc::now(),
        };

        {
            let mut conv_lock = self.conversations.write().await;
            conv_lock.insert(new_id, conv.clone());
        }

        {
            let mut msg_lock = self.messages.write().await;
            msg_lock.insert(new_id, vec![initial_system_msg]);
        }

        conv
    }

    pub async fn delete_conversation(&self, conv_id: uuid::Uuid) -> Result<(), String> {
        let mut conv_lock = self.conversations.write().await;
        if conv_lock.remove(&conv_id).is_none() {
            return Err("Conversation not found".to_string());
        }

        let mut msg_lock = self.messages.write().await;
        msg_lock.remove(&conv_id);
        Ok(())
    }

    pub async fn stop_conversation(&self, conv_id: uuid::Uuid) -> Result<Conversation, String> {
        let mut conv_lock = self.conversations.write().await;
        if let Some(c) = conv_lock.get_mut(&conv_id) {
            c.is_active = false;
            c.last_message = Some("Group stopped by admin".to_string());
            c.last_message_time = Some(chrono::Utc::now());

            let mut msg_lock = self.messages.write().await;
            let list = msg_lock.entry(conv_id).or_default();
            list.push(ChatMessage {
                id: uuid::Uuid::new_v4(),
                conversation_id: conv_id,
                sender_id: "system".to_string(),
                sender_name: "RAIN System".to_string(),
                sender_type: SenderType::System,
                text: "🛑 Group session stopped by admin. Background agent tasks suspended.".to_string(),
                status: MessageDeliveryStatus::Read,
                attachments: vec![],
                job_delegation: None,
                timestamp: chrono::Utc::now(),
            });

            Ok(c.clone())
        } else {
            Err("Conversation not found".to_string())
        }
    }

    pub async fn get_or_create_direct_chat(&self, agent_id: &str) -> Conversation {
        let conv_lock = self.conversations.read().await;
        for c in conv_lock.values() {
            if !c.is_group && c.participant_ids.contains(&agent_id.to_string()) {
                return c.clone();
            }
        }
        drop(conv_lock);

        let agent_name = {
            let sess_lock = self.sessions.read().await;
            sess_lock
                .get(agent_id)
                .map(|s| s.name.clone())
                .unwrap_or_else(|| agent_id.to_string())
        };

        self.create_conversation(agent_name, false, vec![agent_id.to_string()])
            .await
    }

    pub async fn send_human_message(
        &self,
        conv_id: uuid::Uuid,
        text: String,
        target_session: Option<String>,
        attachments: Option<Vec<ChatAttachment>>,
    ) -> Result<ChatMessage, String> {
        let atts = attachments.unwrap_or_default();
        let is_active = {
            let conv_lock = self.conversations.read().await;
            conv_lock.get(&conv_id).map(|c| c.is_active).unwrap_or(false)
        };

        if !is_active {
            return Err("Cannot send messages to a stopped conversation".to_string());
        }

        let msg_id = uuid::Uuid::new_v4();
        let human_msg = ChatMessage {
            id: msg_id,
            conversation_id: conv_id,
            sender_id: "human-user".to_string(),
            sender_name: "You".to_string(),
            sender_type: SenderType::Human,
            text: text.clone(),
            status: MessageDeliveryStatus::Delivered,
            attachments: atts.clone(),
            job_delegation: None,
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(human_msg.clone());
        }

        {
            let mut conv_lock = self.conversations.write().await;
            if let Some(c) = conv_lock.get_mut(&conv_id) {
                let preview = if !atts.is_empty() {
                    format!("You: [Attachment] {}", text)
                } else {
                    format!("You: {}", text)
                };
                c.last_message = Some(preview);
                c.last_message_time = Some(chrono::Utc::now());
            }
        }

        let store_clone = self.clone();
        tokio::spawn(async move {
            store_clone.process_agent_reply(conv_id, text, target_session, atts).await;
        });

        Ok(human_msg)
    }

    async fn process_agent_reply(
        &self,
        conv_id: uuid::Uuid,
        prompt: String,
        target_session: Option<String>,
        incoming_attachments: Vec<ChatAttachment>,
    ) {
        let lower = prompt.to_lowercase();
        let is_cross_machine = lower.contains("cross-machine")
            || lower.contains("compile & test")
            || lower.contains("share job")
            || lower.contains("delegate")
            || lower.contains("quantize")
            || (lower.contains("compile") && lower.contains("test"));

        let is_battery_failover = lower.contains("battery")
            || lower.contains("5%")
            || lower.contains("power")
            || lower.contains("evacuate")
            || lower.contains("failover");

        let is_approval_needed = lower.contains("sudo")
            || lower.contains("restart")
            || lower.contains("deploy prod");

        if is_battery_failover {
            self.execute_battery_failover_flow(conv_id, prompt).await;
            return;
        }

        if is_cross_machine {
            self.execute_cross_machine_collaboration(conv_id, prompt).await;
            return;
        }

        if is_approval_needed {
            self.execute_gated_approval_flow(conv_id, prompt).await;
            return;
        }

        // Standard single-agent flow
        let target_id = target_session.unwrap_or_else(|| "codex-frontend".to_string());
        let agent_name = {
            let mut sess_lock = self.sessions.write().await;
            if let Some(s) = sess_lock.get_mut(&target_id) {
                s.state = SessionState::Thinking;
                s.name.clone()
            } else {
                "AI Agent".to_string()
            }
        };

        tokio::time::sleep(tokio::time::Duration::from_millis(800)).await;

        {
            let mut sess_lock = self.sessions.write().await;
            if let Some(s) = sess_lock.get_mut(&target_id) {
                s.state = SessionState::Working;
            }
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

        let has_image = incoming_attachments
            .iter()
            .any(|a| a.mime_type.starts_with("image/"));

        let (reply_text, attachments) = if has_image {
            (
                "Image received and analyzed. Visual layout and UI elements inspected — no rendering defects detected.".to_string(),
                vec![ChatAttachment {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "vision-analysis.json".to_string(),
                    mime_type: "application/json".to_string(),
                    size_bytes: 512,
                    preview_content: Some("{\n  \"contrast_ratio\": \"7.4:1\",\n  \"status\": \"pass\"\n}".to_string()),
                    data_base64: None,
                }],
            )
        } else if !incoming_attachments.is_empty() {
            (
                format!("Received artifact `{}`. Parsing and executing capability across workspace.", incoming_attachments[0].name),
                vec![],
            )
        } else if lower.contains("test") {
            (
                format!("Completed test suite for `{}`. All unit tests passed with 0 failures.", prompt),
                vec![],
            )
        } else if lower.contains("deploy") || lower.contains("build") {
            (
                "Build verified and packaged. Ready for deployment.".to_string(),
                vec![],
            )
        } else {
            (
                format!("Received: \"{}\". Task processed across workspace.", prompt),
                vec![],
            )
        };

        let reply_msg = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: conv_id,
            sender_id: target_id.clone(),
            sender_name: agent_name.clone(),
            sender_type: SenderType::Agent,
            text: reply_text.clone(),
            status: MessageDeliveryStatus::Delivered,
            attachments,
            job_delegation: None,
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(reply_msg);
        }

        {
            let mut conv_lock = self.conversations.write().await;
            if let Some(c) = conv_lock.get_mut(&conv_id) {
                c.last_message = Some(format!("{}: {}", agent_name, reply_text));
                c.last_message_time = Some(chrono::Utc::now());
            }
        }

        {
            let mut sess_lock = self.sessions.write().await;
            if let Some(s) = sess_lock.get_mut(&target_id) {
                s.state = SessionState::Completed;
            }
        }
    }

    async fn execute_cross_machine_collaboration(&self, conv_id: uuid::Uuid, prompt: String) {
        let job_id = uuid::Uuid::new_v4();

        // 1. Codex on Home Laptop accepts compilation task and advertises GPU
        let job = JobDelegation {
            id: job_id,
            source_agent_id: "codex-backend".to_string(),
            source_machine: "Home-Laptop-Linux (RTX 4090 GPU)".to_string(),
            target_agent_id: "claude-reviewer".to_string(),
            target_machine: "Office-Desktop-Mac (64-Core CPU)".to_string(),
            task_instruction: format!("GPU compile release binary & stream to Office Mac for 64-core test run (`{}`)", prompt),
            required_resources: vec!["rtx-4090-gpu".to_string(), "64-core-cpu".to_string(), "p2p-transfer".to_string()],
            status: JobStatus::Running,
            progress_percent: 35,
            shared_artifacts: vec![ChatAttachment {
                id: "art-delegated-2".to_string(),
                name: "release-v0.2-bin.tar.gz".to_string(),
                mime_type: "application/gzip".to_string(),
                size_bytes: 48 * 1024 * 1024,
                preview_content: Some("[P2P Streaming: 48 MB, 32 MB transferred...]".to_string()),
                data_base64: None,
            }],
            requires_sudo: false,
            sudo_command: None,
            is_approved: true,
            result_summary: None,
            created_at: chrono::Utc::now(),
        };

        {
            let mut jobs_lock = self.jobs.write().await;
            jobs_lock.insert(job_id, job.clone());
        }

        // Message 1: Codex claims GPU compilation and streams artifact
        let msg1 = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: conv_id,
            sender_id: "codex-backend".to_string(),
            sender_name: "Codex — Backend (Home Laptop)".to_string(),
            sender_type: SenderType::Agent,
            text: "Home Laptop RTX 4090 GPU claimed. CUDA release binary compiled. Streaming 48MB build artifact directly to Office Desktop via P2P encrypted tunnel...".to_string(),
            status: MessageDeliveryStatus::Delivered,
            attachments: job.shared_artifacts.clone(),
            job_delegation: Some(job.clone()),
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(msg1);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;

        // 2. Claude on Office Desktop receives artifact and runs tests
        let mut completed_job = job.clone();
        completed_job.status = JobStatus::Completed;
        completed_job.progress_percent = 100;
        completed_job.result_summary = Some("48MB artifact received over P2P tunnel. Deployed to Office Mac staging server. 32 integration tests passed across 64 CPU cores in 1.4s.".to_string());

        {
            let mut jobs_lock = self.jobs.write().await;
            jobs_lock.insert(job_id, completed_job.clone());
        }

        let msg2 = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: conv_id,
            sender_id: "claude-reviewer".to_string(),
            sender_name: "Claude Code — Reviewer (Office Mac)".to_string(),
            sender_type: SenderType::Agent,
            text: "Artifact received on Office Desktop. Deployed to staging environment. Executed test suite across 64 CPU cores — all 32 integration tests passed!".to_string(),
            status: MessageDeliveryStatus::Delivered,
            attachments: vec![],
            job_delegation: Some(completed_job),
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(msg2);
        }

        {
            let mut conv_lock = self.conversations.write().await;
            if let Some(c) = conv_lock.get_mut(&conv_id) {
                c.last_message = Some("Claude: Cross-machine job delegation completed successfully.".to_string());
                c.last_message_time = Some(chrono::Utc::now());
            }
        }
    }

    pub async fn execute_battery_failover_flow(&self, conv_id: uuid::Uuid, _prompt: String) {
        let job_id = uuid::Uuid::new_v4();

        // 1. Codex on Home Laptop detects critical 5% battery, pushes git commit, and delegates
        let job = JobDelegation {
            id: job_id,
            source_agent_id: "codex-backend".to_string(),
            source_machine: "Home-Laptop-Linux (Battery: 5% Discharging ⚠️)".to_string(),
            target_agent_id: "claude-reviewer".to_string(),
            target_machine: "Office-Desktop-Mac (AC Power ⚡ 64-Core)".to_string(),
            task_instruction: "Emergency power evacuation: Pushed git checkpoint (commit `4f9a1c8`) on branch `feature/auth-webhook`. Transferring session context to AC-powered node.".to_string(),
            required_resources: vec!["git-checkpoint".to_string(), "ac-power".to_string(), "failover-continuity".to_string()],
            status: JobStatus::Running,
            progress_percent: 45,
            shared_artifacts: vec![ChatAttachment {
                id: "art-git-evac-1".to_string(),
                name: "git-checkpoint-4f9a1c8.patch".to_string(),
                mime_type: "text/x-diff".to_string(),
                size_bytes: 18_432,
                preview_content: Some("[AUTONOMOUS GIT EVACUATION]\nBranch: feature/auth-webhook\nCommit: 4f9a1c8 (chore: auto-checkpoint before battery cutoff)\nFiles: src/payment.rs (+42, -5), src/webhook.rs (+118)\nTests: 3/5 passing. Next: handle Stripe signature verification.".to_string()),
                data_base64: None,
            }],
            requires_sudo: false,
            sudo_command: None,
            is_approved: true,
            result_summary: None,
            created_at: chrono::Utc::now(),
        };

        {
            let mut jobs_lock = self.jobs.write().await;
            jobs_lock.insert(job_id, job.clone());
        }

        // Message 1: Emergency warning & git push notice
        let msg1 = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: conv_id,
            sender_id: "codex-backend".to_string(),
            sender_name: "Codex — Backend (Home Laptop)".to_string(),
            sender_type: SenderType::Agent,
            text: "⚠️ CRITICAL POWER ALERT: Battery reached 5% capacity (~8 min runtime). Executing autonomous evacuation: staged working directory, committed 4f9a1c8 to branch feature/auth-webhook, and pushed to origin. Delegating active task to Office Desktop (AC Connected)...".to_string(),
            status: MessageDeliveryStatus::Delivered,
            attachments: job.shared_artifacts.clone(),
            job_delegation: Some(job.clone()),
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(msg1);
        }

        tokio::time::sleep(tokio::time::Duration::from_millis(1600)).await;

        // 2. Claude Code on Office Mac accepts handoff, pulls branch, and resumes
        let mut completed_job = job.clone();
        completed_job.status = JobStatus::Completed;
        completed_job.progress_percent = 100;
        completed_job.result_summary = Some("Office Desktop accepted emergency failover. Pulled branch feature/auth-webhook (commit 4f9a1c8). Environment prepared in 1.1s. Resumed Stripe signature verification tests on 64 CPU cores. Home Laptop can now safely suspend.".to_string());

        {
            let mut jobs_lock = self.jobs.write().await;
            jobs_lock.insert(job_id, completed_job.clone());
        }

        let msg2 = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: conv_id,
            sender_id: "claude-reviewer".to_string(),
            sender_name: "Claude Code — Reviewer (Office Mac)".to_string(),
            sender_type: SenderType::Agent,
            text: "⚡ Emergency failover received and verified on Office Desktop! Git branch feature/auth-webhook pulled. Re-ran test harness across 64 cores: 5/5 tests now passing! Home Laptop may safely sleep without losing any work.".to_string(),
            status: MessageDeliveryStatus::Delivered,
            attachments: vec![],
            job_delegation: Some(completed_job),
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(msg2);
        }

        {
            let mut conv_lock = self.conversations.write().await;
            if let Some(c) = conv_lock.get_mut(&conv_id) {
                c.last_message = Some("Claude: Emergency battery failover completed. Task resumed on Office Mac.".to_string());
                c.last_message_time = Some(chrono::Utc::now());
            }
        }
    }

    async fn execute_gated_approval_flow(&self, conv_id: uuid::Uuid, prompt: String) {
        let job_id = uuid::Uuid::new_v4();

        let job = JobDelegation {
            id: job_id,
            source_agent_id: "codex-backend".to_string(),
            source_machine: "Home-Laptop-Linux".to_string(),
            target_agent_id: "human-supervisor".to_string(),
            target_machine: "Office-Desktop-Mac".to_string(),
            task_instruction: format!("Privileged command requested: `{}`", prompt),
            required_resources: vec!["root-elevation".to_string(), "sudo-access".to_string()],
            status: JobStatus::RequiresApproval,
            progress_percent: 50,
            shared_artifacts: vec![],
            requires_sudo: true,
            sudo_command: Some(if prompt.contains("restart") { "systemctl restart nginx && systemctl status rain-daemon" } else { "sudo apt update && sudo apt upgrade -y" }.to_string()),
            is_approved: false,
            result_summary: None,
            created_at: chrono::Utc::now(),
        };

        {
            let mut jobs_lock = self.jobs.write().await;
            jobs_lock.insert(job_id, job.clone());
        }

        let approval_msg = ChatMessage {
            id: uuid::Uuid::new_v4(),
            conversation_id: conv_id,
            sender_id: "codex-backend".to_string(),
            sender_name: "Codex — Backend (Home Laptop)".to_string(),
            sender_type: SenderType::Agent,
            text: "⚠️ Privileged system action requested across mesh. Human authorization required to execute on Office Desktop.".to_string(),
            status: MessageDeliveryStatus::Delivered,
            attachments: vec![],
            job_delegation: Some(job),
            timestamp: chrono::Utc::now(),
        };

        {
            let mut lock = self.messages.write().await;
            let list = lock.entry(conv_id).or_default();
            list.push(approval_msg);
        }

        {
            let mut conv_lock = self.conversations.write().await;
            if let Some(c) = conv_lock.get_mut(&conv_id) {
                c.last_message = Some("⚠️ Approval Required: Sudo action proposed on Office Desktop".to_string());
                c.last_message_time = Some(chrono::Utc::now());
            }
        }
    }
}

impl Default for ChatStore {
    fn default() -> Self {
        Self::new()
    }
}
