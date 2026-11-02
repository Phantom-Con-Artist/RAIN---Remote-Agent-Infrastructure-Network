use rain_core::{AgentSupervisor, ChatStore};
use rain_protocol::{AgentLaunchRequest, ChatAttachment, ConversationExportFormat};

async fn spawn_echo_agent(supervisor: &AgentSupervisor, name: &str) -> uuid::Uuid {
    let agent = supervisor
        .start_agent(AgentLaunchRequest {
            name: name.into(),
            provider: "custom".into(),
            command: "sh".into(),
            args: vec![
                "-c".into(),
                "while IFS= read -r line; do printf '%s\\n' \"$line\"; done".into(),
            ],
            cwd: None,
            interactive: false,
            machine: None,
            env_vars: Default::default(),
        })
        .await
        .unwrap();
    agent.id
}

#[tokio::test]
async fn test_export_transcript_markdown_and_json() {
    let supervisor = AgentSupervisor::new();
    let store = ChatStore::with_supervisor(supervisor.clone());
    let agent_id = spawn_echo_agent(&supervisor, "sec-bot").await;

    let temp_dir = std::env::temp_dir().join(format!("test-art-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let file_path = temp_dir.join("gateway_spec.md");
    std::fs::write(&file_path, b"# Spec content").unwrap();
    let art = store
        .artifacts
        .capture(None, file_path.to_str().unwrap())
        .await
        .unwrap();

    let conv = store
        .create_conversation(
            "Security Audit Channel".to_string(),
            true,
            vec!["human-user".into(), agent_id.to_string()],
        )
        .await;

    // Send a message
    store
        .send_human_message(
            conv.id,
            "Please review the gateway auth token length.".to_string(),
            None,
            Some(vec![ChatAttachment {
                id: art.id.to_string(),
                name: art.name,
                mime_type: art.mime_type,
                size_bytes: art.size_bytes,
                preview_content: Some("Spec content".into()),
                sha256: Some(art.sha256),
                data_base64: None,
            }]),
        )
        .await
        .unwrap();

    // Give the echo agent a brief moment to finish its reply
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // Export markdown
    let export_md = store
        .export_transcript(conv.id, ConversationExportFormat::Markdown)
        .await
        .unwrap();

    assert_eq!(export_md.conversation_id, conv.id);
    assert_eq!(export_md.title, "Security Audit Channel");
    assert!(export_md.is_group);
    assert!(export_md.message_count >= 1);
    assert!(export_md.content.contains("# RAIN Conversation: Security Audit Channel"));
    assert!(export_md.content.contains("gateway_spec.md"));
    assert!(export_md.estimated_tokens > 0);

    // Export JSON
    let export_json = store
        .export_transcript(conv.id, ConversationExportFormat::Json)
        .await
        .unwrap();

    assert_eq!(export_json.message_count, export_md.message_count);
    let parsed: serde_json::Value = serde_json::from_str(&export_json.content).unwrap();
    assert_eq!(parsed["title"], "Security Audit Channel");
    assert_eq!(parsed["messages"].as_array().unwrap().len(), export_md.message_count);

    // Export PlainText
    let export_txt = store
        .export_transcript(conv.id, ConversationExportFormat::PlainText)
        .await
        .unwrap();
    assert!(export_txt.content.contains("RAIN TRANSCRIPT: Security Audit Channel"));
    assert!(export_txt.content.contains("gateway_spec.md"));

    supervisor.stop_agent(agent_id).await.unwrap();
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_search_messages_with_filters_and_attachments() {
    let supervisor = AgentSupervisor::new();
    let store = ChatStore::with_supervisor(supervisor.clone());
    let agent_id = spawn_echo_agent(&supervisor, "deploy-bot").await;

    let temp_dir = std::env::temp_dir().join(format!("test-art-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let file_path = temp_dir.join("deploy_manifest.yaml");
    std::fs::write(&file_path, b"apiVersion: apps/v1").unwrap();
    let art = store
        .artifacts
        .capture(None, file_path.to_str().unwrap())
        .await
        .unwrap();

    let conv = store
        .create_conversation(
            "Deployment Chat".to_string(),
            false,
            vec!["human-user".into(), agent_id.to_string()],
        )
        .await;

    // Send message 1
    store
        .send_human_message(
            conv.id,
            "Initiating zero-downtime deployment to worker node-alpha.".to_string(),
            None,
            None,
        )
        .await
        .unwrap();

    // Send message 2 with attachment
    store
        .send_human_message(
            conv.id,
            "Verification logs generated.".to_string(),
            None,
            Some(vec![ChatAttachment {
                id: art.id.to_string(),
                name: art.name,
                mime_type: art.mime_type,
                size_bytes: art.size_bytes,
                preview_content: None,
                sha256: Some(art.sha256),
                data_base64: None,
            }]),
        )
        .await
        .unwrap();

    // Wait briefly for echo
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    // 1. Search by keyword
    let matches = store
        .search_messages(Some(conv.id), "zero-downtime", None, None)
        .await;
    assert!(!matches.is_empty());
    assert_eq!(matches[0].conversation_id, conv.id);
    assert!(matches[0].snippet.contains("zero-downtime"));

    // 2. Search by attachment name
    let att_matches = store
        .search_messages(Some(conv.id), "manifest", None, None)
        .await;
    assert_eq!(att_matches.len(), 1);
    assert_eq!(att_matches[0].matched_attachments, vec!["deploy_manifest.yaml"]);

    // 3. Search across all conversations (conv_id = None)
    let all_matches = store.search_messages(None, "deployment", None, None).await;
    assert!(all_matches.len() >= 1);

    // 4. Sender filter test
    let human_matches = store
        .search_messages(Some(conv.id), "deployment", Some("human"), None)
        .await;
    assert_eq!(human_matches.len(), 1);

    let system_matches = store
        .search_messages(Some(conv.id), "deployment", Some("system"), None)
        .await;
    assert_eq!(system_matches.len(), 1);

    let agent_matches = store
        .search_messages(Some(conv.id), "deployment", Some("agent"), None)
        .await;
    assert_eq!(agent_matches.len(), 0);

    supervisor.stop_agent(agent_id).await.unwrap();
    let _ = std::fs::remove_dir_all(&temp_dir);
}
