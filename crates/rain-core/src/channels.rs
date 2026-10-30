use crate::ChatStore;
use rain_protocol::{
    ChatMessage, CollaborationRequest, CoworkerEnvelope, MessageDeliveryStatus, SenderType,
};
use std::sync::Arc;
use uuid::Uuid;
impl ChatStore {
    pub(crate) async fn invalidate_human_deliveries(&self, id: Uuid) -> Uuid {
        let ticket = Uuid::new_v4();
        self.human_tickets.lock().await.insert(id, ticket);
        ticket
    }

    pub async fn list_dispatches(&self) -> Vec<rain_protocol::ChatDispatch> {
        let mut list = self.dispatches.read().await.clone();
        list.reverse();
        list.truncate(250);
        list
    }
    pub(crate) async fn dispatch_message(
        &self,
        conversation_id: Uuid,
        message_id: Uuid,
        source_id: &str,
        source_name: &str,
        target: &rain_protocol::AgentProcess,
        run_id: Option<Uuid>,
        text: &str,
        human: bool,
    ) -> Result<(), String> {
        let dispatch = rain_protocol::ChatDispatch {
            id: Uuid::new_v4(),
            conversation_id,
            message_id,
            run_id,
            source_id: source_id.into(),
            source_name: source_name.into(),
            target_id: target.id.to_string(),
            target_name: target.name.clone(),
            target_machine: target.machine.clone(),
            status: "queued".into(),
            queued_at: chrono::Utc::now(),
            started_at: None,
            finished_at: None,
            error: None,
        };
        let id = dispatch.id;
        {
            let mut list = self.dispatches.write().await;
            list.push(dispatch);
            if list.len() > 5000 {
                let excess = list.len() - 5000;
                list.drain(..excess);
            }
        }
        let options = self
            .conversations
            .read()
            .await
            .get(&conversation_id)
            .filter(|c| !c.is_group)
            .map(|c| c.native_settings.clone());
        let result = self
            .supervisor
            .send_tracked_input(target.id, text, conversation_id, run_id, human, id, options)
            .await;
        if let Err(error) = &result {
            if let Some(dispatch) = self
                .dispatches
                .write()
                .await
                .iter_mut()
                .find(|d| d.id == id)
            {
                dispatch.status = "failed".into();
                dispatch.finished_at = Some(chrono::Utc::now());
                dispatch.error = Some(error.clone());
            }
        }
        result
    }
    pub(crate) async fn conversation_gate(&self, id: Uuid) -> Arc<tokio::sync::Mutex<()>> {
        self.collaboration_locks
            .lock()
            .await
            .entry(id)
            .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
            .clone()
    }
    pub(crate) async fn mentioned_peers(
        &self,
        id: Uuid,
        text: &str,
        sender: Option<Uuid>,
    ) -> Result<Option<Vec<String>>, String> {
        let c = self
            .conversations
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or("Conversation missing")?;
        let agents = self.supervisor.list_agents().await;
        let mut labels = Vec::new();
        for (_, suffix) in text.match_indices("@{").map(|(i, _)| (i, &text[i + 2..])) {
            if let Some((label, _)) = suffix.split_once('}') {
                labels.push(label.to_string());
            }
        }
        for token in text.split_whitespace() {
            let mention = if let Some(value) = token.strip_prefix("<@") {
                value.split_once('>').map(|p| p.0)
            } else {
                token.strip_prefix('@').filter(|v| !v.starts_with('{'))
            };
            if let Some(label) = mention {
                let label = label.trim_end_matches(|c: char| ",:;!?".contains(c));
                if !label.is_empty() {
                    labels.push(label.to_string());
                }
            }
        }
        if labels.is_empty() || labels.iter().any(|l| l.eq_ignore_ascii_case("all")) {
            return Ok(None);
        }
        let mut recipients = Vec::new();
        for label in labels {
            let matched: Vec<_> = agents
                .iter()
                .filter(|a| {
                    c.participant_ids.contains(&a.id.to_string())
                        && Some(a.id) != sender
                        && (a.name.eq_ignore_ascii_case(&label) || a.id.to_string() == label)
                })
                .collect();
            if matched.len() != 1 {
                return Err(format!("@{label} does not uniquely identify a channel member. Use @{{Full Name}} or a session UUID."));
            }
            let id = matched[0].id.to_string();
            if !recipients.contains(&id) {
                recipients.push(id);
            }
        }
        Ok(Some(recipients))
    }
    pub(crate) async fn channel_command(
        &self,
        id: Uuid,
        text: &str,
    ) -> Option<Result<ChatMessage, String>> {
        let command = text.trim();
        let result = match command {
            "/pause" | "/stop" => self.stop_conversation(id).await.map(|_| ()),
            "/resume" => self.resume_conversation(id).await.map(|_| ()),
            "/autonomy on" | "/autonomy off" => self
                .configure_collaboration(
                    id,
                    CollaborationRequest {
                        enabled: command.ends_with("on"),
                        max_turns: 0,
                    },
                )
                .await
                .map(|_| ()),
            _ => return None,
        };
        if let Err(e) = result {
            return Some(Err(e));
        }
        let message = ChatMessage {
            id: Uuid::new_v4(),
            conversation_id: id,
            sender_id: "human-user".into(),
            sender_name: "You".into(),
            sender_type: SenderType::Human,
            text: command.into(),
            status: MessageDeliveryStatus::Delivered,
            attachments: vec![],
            job_delegation: None,
            coworker: None,
            timestamp: chrono::Utc::now(),
        };
        self.messages
            .write()
            .await
            .entry(id)
            .or_default()
            .push(message.clone());
        Some(Ok(message))
    }
    pub(crate) async fn publish_passive_reply(&self, sender: Uuid, id: Uuid, text: &str) {
        if text.trim() == "<rain-silent/>" {
            return;
        }
        let visible = serde_json::from_str::<CoworkerEnvelope>(text)
            .map(|e| e.text)
            .unwrap_or_else(|_| {
                crate::file_messages::extract_files(text)
                    .map(|v| v.0)
                    .unwrap_or_else(|_| text.into())
            });
        let name = self
            .supervisor
            .get_agent(sender)
            .await
            .map(|a| a.name)
            .unwrap_or_else(|| sender.to_string());
        let mut conversations = self.conversations.write().await;
        let Some(c) = conversations.get_mut(&id) else {
            return;
        };
        if !c.participant_ids.contains(&sender.to_string()) {
            return;
        }
        c.last_message = Some(format!("{name}: {visible}"));
        c.last_message_time = Some(chrono::Utc::now());
        self.messages
            .write()
            .await
            .entry(id)
            .or_default()
            .push(ChatMessage {
                id: Uuid::new_v4(),
                conversation_id: id,
                sender_id: sender.to_string(),
                sender_name: name,
                sender_type: SenderType::Agent,
                text: visible,
                status: MessageDeliveryStatus::Delivered,
                attachments: vec![],
                job_delegation: None,
                coworker: None,
                timestamp: chrono::Utc::now(),
            });
    }
}

impl ChatStore {
    pub async fn set_mode(
        &self,
        id: Uuid,
        request: rain_protocol::ModeRequest,
    ) -> Result<rain_protocol::Conversation, String> {
        let previous = self
            .conversations
            .read()
            .await
            .get(&id)
            .cloned()
            .ok_or("Conversation missing")?;
        if !previous.is_group {
            return Err("Modes are configured for group channels".into());
        }
        self.stop_conversation(id).await?;
        let mut conversations = self.conversations.write().await;
        let c = conversations.get_mut(&id).ok_or("Conversation missing")?;
        if !c.is_group {
            return Err("Modes are configured for group channels".into());
        }
        let state = c.collaboration.get_or_insert_with(Default::default);
        state.mode = request.mode;
        if let Some(context) = request.context {
            state.human_goal = Some(context.chars().take(16000).collect());
        }
        c.is_active = previous.is_active;
        Ok(c.clone())
    }
    pub async fn conversation_activity(&self, id: Uuid) -> Result<serde_json::Value, String> {
        if !self.conversations.read().await.contains_key(&id) {
            return Err("Conversation missing".into());
        }
        let dispatches: Vec<_> = self
            .list_dispatches()
            .await
            .into_iter()
            .filter(|d| d.conversation_id == id)
            .collect();
        let traces: Vec<_> = self
            .traces
            .read()
            .await
            .iter()
            .rev()
            .filter(|t| t.conversation_id == id)
            .take(250)
            .cloned()
            .collect();
        Ok(serde_json::json!({"dispatches":dispatches,"traces":traces}))
    }
    pub async fn control_member(
        &self,
        id: Uuid,
        agent: Uuid,
        terminate: bool,
    ) -> Result<(), String> {
        let gate = self.conversation_gate(id).await;
        let _serial = gate.lock().await;
        {
            let mut conversations = self.conversations.write().await;
            let c = conversations.get_mut(&id).ok_or("Conversation missing")?;
            if !c.participant_ids.contains(&agent.to_string()) {
                return Err("Agent is not a member".into());
            }
            if let Some(s) = c.collaboration.as_mut() {
                if !s.interrupted_ids.contains(&agent.to_string()) {
                    s.interrupted_ids.push(agent.to_string());
                }
            }
        }
        self.invalidate_human_deliveries(id).await;
        if terminate {
            self.supervisor.stop_agent(agent).await
        } else {
            self.supervisor.cancel_chat_conversation(agent, id).await
        }
    }
}
