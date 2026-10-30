use crate::ChatStore;
use rain_protocol::{ChatMessage, MessageDeliveryStatus, SenderType};
use uuid::Uuid;

pub(crate) fn extract_files(text: &str) -> Result<(String, Vec<String>), String> {
    let Some(start) = text.find("<rain-files>") else {
        return Ok((text.into(), vec![]));
    };
    let end = text[start..]
        .find("</rain-files>")
        .map(|i| i + start)
        .ok_or("File share is missing its closing tag")?;
    let paths: Vec<String> = serde_json::from_str(&text[start + 12..end])
        .map_err(|_| "File share expects a JSON array of paths")?;
    let visible = format!("{}{}", &text[..start], &text[end + 13..]);
    Ok((visible.trim().to_string(), paths))
}
impl ChatStore {
    pub(crate) async fn record_direct_files(&self, id: Uuid, sender: Uuid, text: &str) -> bool {
        let c = self.conversations.read().await.get(&id).cloned();
        if !c
            .as_ref()
            .map(|c| !c.is_group && c.is_active && c.participant_ids.contains(&sender.to_string()))
            .unwrap_or(false)
            || !text.contains("<rain-files>")
        {
            return false;
        }
        let result = async {
            let (text, paths) = extract_files(text)?;
            let source = self
                .supervisor
                .get_agent(sender)
                .await
                .ok_or("Session missing")?;
            let mut files = vec![];
            for path in paths {
                let path = if std::path::Path::new(&path).is_absolute() || path.starts_with('~') {
                    path
                } else {
                    std::path::Path::new(&source.cwd)
                        .join(path)
                        .to_string_lossy()
                        .into_owned()
                };
                files.push(
                    self.artifacts
                        .capture(source.machine.as_deref(), &path)
                        .await?
                        .attachment(),
                );
            }
            let text = if text.is_empty() {
                "Shared files".into()
            } else {
                text
            };
            let mut conversations = self.conversations.write().await;
            let c = conversations.get_mut(&id).ok_or("Conversation missing")?;
            c.last_message = Some(format!("{}: {}", source.name, text));
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
                    sender_name: source.name,
                    sender_type: SenderType::Agent,
                    text,
                    status: MessageDeliveryStatus::Delivered,
                    attachments: files,
                    job_delegation: None,
                    coworker: None,
                    timestamp: chrono::Utc::now(),
                });
            Ok::<_, String>(())
        }
        .await;
        if let Err(e) = result {
            self.coworker_notice(id, &format!("File sharing failed: {e}"), true)
                .await;
        }
        true
    }
}

pub(crate) fn extract_flair(text: &str) -> (String, Option<rain_protocol::MessageFlair>) {
    if let Some(tail) = text.strip_prefix('[') {
        if let Some((label, body)) = tail.split_once(']') {
            if let Ok(flair) = serde_json::from_value::<rain_protocol::MessageFlair>(
                serde_json::json!(label.to_lowercase()),
            ) {
                return (body.trim_start().into(), Some(flair));
            }
        }
    }
    (text.into(), None)
}
