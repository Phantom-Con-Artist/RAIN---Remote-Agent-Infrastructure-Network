use rain_core::{AgentSupervisor, ChatStore};
use rain_protocol::{AgentLaunchRequest, ChatAttachment};

async fn spawn_mock_agent(supervisor: &AgentSupervisor, name: &str) -> uuid::Uuid {
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
async fn test_update_message_meta_pin_and_bookmark() {
    let supervisor = AgentSupervisor::new();
    let store = ChatStore::with_supervisor(supervisor.clone());
    let agent_id = spawn_mock_agent(&supervisor, "dev-worker").await;

    let conv = store
        .create_conversation(
            "Feature Planning".into(),
            true,
            vec!["human-user".into(), agent_id.to_string()],
        )
        .await;

    let msg = store
        .send_human_message(
            conv.id,
            "Let us implement persistent message bookmarks.".into(),
            None,
            None,
        )
        .await
        .unwrap();

    assert!(!msg.is_pinned);
    assert!(!msg.is_bookmarked);
    assert!(msg.tags.is_empty());

    // Pin the message
    let updated = store
        .update_message_meta(conv.id, msg.id, Some(true), None, None, None)
        .await
        .unwrap();
    assert!(updated.is_pinned);
    assert!(!updated.is_bookmarked);

    // Bookmark with a note and tags
    let updated2 = store
        .update_message_meta(
            conv.id,
            msg.id,
            None,
            Some(true),
            Some("Crucial design decision".into()),
            Some(vec!["Architecture".into(), "v1".into(), "architecture".into()]),
        )
        .await
        .unwrap();
    assert!(updated2.is_pinned);
    assert!(updated2.is_bookmarked);
    assert_eq!(
        updated2.bookmark_note.as_deref(),
        Some("Crucial design decision")
    );
    // Tags should be deduplicated and lowercase
    assert_eq!(updated2.tags, vec!["architecture", "v1"]);

    // Query pinned messages
    let pinned = store.get_pinned_messages(conv.id).await;
    assert_eq!(pinned.len(), 1);
    assert_eq!(pinned[0].id, msg.id);

    // Query bookmarked messages
    let bookmarked = store.get_bookmarked_messages(Some(conv.id)).await;
    assert_eq!(bookmarked.len(), 1);
    assert_eq!(bookmarked[0].id, msg.id);

    // Unpin
    let unpinned = store
        .update_message_meta(conv.id, msg.id, Some(false), None, None, None)
        .await
        .unwrap();
    assert!(!unpinned.is_pinned);
    let pinned_after = store.get_pinned_messages(conv.id).await;
    assert!(pinned_after.is_empty());

    // Unbookmark
    let unbookmarked = store
        .update_message_meta(conv.id, msg.id, None, Some(false), None, None)
        .await
        .unwrap();
    assert!(!unbookmarked.is_bookmarked);
    assert!(unbookmarked.bookmark_note.is_none());
    let bookmarked_after = store.get_bookmarked_messages(Some(conv.id)).await;
    assert!(bookmarked_after.is_empty());
}

#[tokio::test]
async fn test_conversation_stats_calculation() {
    let supervisor = AgentSupervisor::new();
    let store = ChatStore::with_supervisor(supervisor.clone());
    let agent_id = spawn_mock_agent(&supervisor, "analyst-agent").await;

    let conv = store
        .create_conversation(
            "Metrics Review".into(),
            false,
            vec!["human-user".into(), agent_id.to_string()],
        )
        .await;

    // Send two messages
    let msg1 = store
        .send_human_message(
            conv.id,
            "First observation regarding query latency across endpoints.".into(),
            None,
            None,
        )
        .await
        .unwrap();

    let temp_dir = std::env::temp_dir().join(format!("test-art-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let file_path = temp_dir.join("metrics.csv");
    std::fs::write(&file_path, b"latency,count\n12,40\n").unwrap();
    let art = store
        .artifacts
        .capture(None, file_path.to_str().unwrap())
        .await
        .unwrap();

    let msg2 = store
        .send_human_message(
            conv.id,
            "Second observation with high priority findings.".into(),
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

    // Pin msg1, tag msg2
    store
        .update_message_meta(
            conv.id,
            msg1.id,
            Some(true),
            None,
            None,
            Some(vec!["latency".into(), "p99".into()]),
        )
        .await
        .unwrap();

    store
        .update_message_meta(
            conv.id,
            msg2.id,
            None,
            Some(true),
            Some("Note for follow-up".into()),
            Some(vec!["latency".into(), "audit".into()]),
        )
        .await
        .unwrap();

    let stats = store.get_conversation_stats(conv.id).await.unwrap();

    // Note: create_conversation adds 1 system message initially
    assert_eq!(stats.total_messages, 3);
    assert_eq!(stats.human_messages, 2);
    assert_eq!(stats.system_messages, 1);
    assert_eq!(stats.pinned_messages, 1);
    assert_eq!(stats.bookmarked_messages, 1);
    assert_eq!(stats.attachments_count, 1);
    assert!(stats.total_words > 10);
    assert!(stats.estimated_tokens > stats.total_words);

    // Tag counts check: "latency" appears twice, "audit" and "p99" appear once
    let tag_map: std::collections::HashMap<String, usize> = stats.top_tags.into_iter().collect();
    assert_eq!(tag_map.get("latency"), Some(&2));
    assert_eq!(tag_map.get("audit"), Some(&1));
    assert_eq!(tag_map.get("p99"), Some(&1));

    assert!(stats.active_participants.iter().any(|p| p == "You"));
}
