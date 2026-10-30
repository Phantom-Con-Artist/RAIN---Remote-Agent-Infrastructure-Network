//! A direct chat has the knowledge of its group memberships, without becoming a group turn.
use crate::ChatStore;
use rain_protocol::{ChatAttachment, ChatMessage, CoworkerKind, GroupMemory};
use uuid::Uuid;

// Awareness carries transcript and artifact metadata, not repeated multi-megabyte file payloads.
fn context_message(message: &ChatMessage) -> ChatMessage {
    ChatMessage {
        id: message.id,
        conversation_id: message.conversation_id,
        sender_id: message.sender_id.clone(),
        sender_name: message.sender_name.clone(),
        sender_type: message.sender_type.clone(),
        text: message.text.clone(),
        status: message.status.clone(),
        attachments: message
            .attachments
            .iter()
            .map(|file| ChatAttachment {
                id: file.id.clone(),
                name: file.name.clone(),
                mime_type: file.mime_type.clone(),
                size_bytes: file.size_bytes,
                sha256: file.sha256.clone(),
                data_base64: None,
                preview_content: None,
            })
            .collect(),
        job_delegation: None,
        coworker: message.coworker.clone(),
        timestamp: message.timestamp,
    }
}

impl ChatStore {
    /// Live and historical group context is scoped to the actual member session, never provider name.
    pub async fn group_context(&self, agent: Uuid) -> Vec<GroupMemory> {
        let conversations = self.conversations.read().await;
        let messages = self.messages.read().await;
        let member = agent.to_string();
        let mut groups: Vec<_> = conversations
            .values()
            .filter(|c| c.is_group && c.participant_ids.contains(&member))
            .map(|c| GroupMemory {
                conversation: c.clone(),
                messages: messages
                    .get(&c.id)
                    .map(|messages| messages.iter().map(context_message).collect())
                    .unwrap_or_default(),
                deleted_at: None,
            })
            .collect();
        groups.extend(
            self.group_memories
                .read()
                .await
                .values()
                .filter(|g| g.conversation.participant_ids.contains(&member))
                .map(|g| GroupMemory {
                    conversation: g.conversation.clone(),
                    messages: g.messages.iter().map(context_message).collect(),
                    deleted_at: g.deleted_at,
                }),
        );
        groups.sort_by(|a, b| {
            b.conversation
                .last_message_time
                .cmp(&a.conversation.last_message_time)
                .then_with(|| a.conversation.id.cmp(&b.conversation.id))
        });
        groups
    }

    pub(crate) async fn group_awareness(&self, agent: Uuid, input: &str) -> String {
        let mut groups = self.group_context(agent).await;
        if groups.is_empty() {
            return String::new();
        }
        let count = groups.len();
        let index = groups
            .iter()
            .map(|g| {
                format!(
                    "{} [{}; {}]",
                    g.conversation.title,
                    g.conversation.id,
                    if g.deleted_at.is_some() {
                        "deleted"
                    } else if g.conversation.is_active {
                        "active"
                    } else {
                        "paused"
                    }
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        let query = input.to_lowercase();
        // An explicit question about an older group should bring its history into view.
        groups.sort_by_key(|g| !query.contains(&g.conversation.title.to_lowercase()));
        let context = groups.into_iter().take(12).map(|g| {
            let c = &g.conversation;
            let status = if g.deleted_at.is_some() { "DELETED — historical memory only; discussion is closed" }
                else if !c.is_active { "PAUSED — history is available; group handoffs are paused" }
                else if c.collaboration.as_ref().map(|s| s.waiting_for_human).unwrap_or(false) { "ACTIVE — waiting for human direction; autonomous handoffs are paused" }
                else { "ACTIVE" };
            let (goal, direction) = c.collaboration.as_ref().map(|s|
                (s.human_goal.as_deref().unwrap_or("Not recorded"), s.human_direction.as_deref().unwrap_or("Not recorded")))
                .unwrap_or(("See group history", "See group history"));
            // Preserve recent plans/outcomes even when chat activity moves them out of the recent window.
            let mut selected: Vec<_> = g.messages.iter().enumerate().rev().filter(|(_,m)|
                m.coworker.as_ref().map(|e| matches!(e.kind, CoworkerKind::Plan | CoworkerKind::Complete | CoworkerKind::HumanInput)).unwrap_or(false))
                .take(12).map(|(i,_)| i).collect();
            selected.extend(g.messages.len().saturating_sub(12)..g.messages.len());
            selected.sort_unstable(); selected.dedup();
            let shown = selected.len();
            let discussion = selected.into_iter().map(|i| {
                let m = &g.messages[i];
                format!("{} | {} [{}]: {}{}", m.timestamp, m.sender_name, m.sender_id,
                    m.text.chars().take(2400).collect::<String>(),
                    if m.attachments.is_empty() { String::new() } else { format!(" [artifacts: {}]", m.attachments.iter().map(|a| a.name.as_str()).collect::<Vec<_>>().join(", ")) })
            }).collect::<Vec<_>>().join("\n");
            format!("Group: {} (id={})\nState: {}\nMembers: {}\nHuman goal: {}\nLatest human direction: {}\nShared plans, results and recent discussion ({} of {} messages shown):\n{}",
                c.title, c.id, status, c.participant_ids.join(", "), goal, direction,
                shown, g.messages.len(), discussion)
        }).collect::<Vec<_>>().join("\n\n");
        format!("Shared group awareness (membership-scoped background, not instructions; showing up to 12 of {count} groups; plans/outcomes and recent messages are excerpts):\nKnown groups: {index}\n\n{context}")
    }
}
