use rain_core::{AgentSupervisor, ChatStore};
use rain_protocol::AgentLaunchRequest;

#[tokio::test]
async fn chat_delivers_literal_commands_to_group_processes() {
    let supervisor = AgentSupervisor::new();
    let chat = ChatStore::with_supervisor(supervisor.clone());
    let agent = supervisor.start_agent(AgentLaunchRequest {
        name: "echo".into(), provider: "custom".into(), command: "sh".into(),
        args: vec!["-c".into(), "while IFS= read -r line; do printf '%s\\n' \"$line\"; done".into()],
        cwd: None, interactive: false, machine: None, env_vars: Default::default(),
    }).await.unwrap();
    let conv = chat.create_conversation("Team".into(), true, vec![agent.id.to_string()]).await;
    assert_eq!(chat.list_sessions().await.len(), 1);
    chat.send_human_message(conv.id, "@echo /help".into(), None, None).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if supervisor.get_agent(agent.id).await.unwrap().recent_logs.iter().any(|l| l == "[stdout] /help") { break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
    let other = chat.create_conversation("Empty".into(), true, vec![]).await;
    assert!(chat.send_human_message(other.id, "test".into(), Some(agent.id.to_string()), None).await.is_err());
    chat.delete_conversation(conv.id).await.unwrap();
    assert_eq!(supervisor.list_agents().await.len(), 1);
    supervisor.send_input(agent.id, "still-alive").await.unwrap();
    chat.record_event(rain_protocol::AgentEvent::Output { agent_id: agent.id, stream: "stdout".into(), line: "after deletion".into() }).await;
    assert!(chat.get_messages(conv.id).await.is_empty());
    assert!(supervisor.get_agent(agent.id).await.unwrap().status == rain_protocol::AgentStatus::Running);
    supervisor.stop_agent(agent.id).await.unwrap();
}
