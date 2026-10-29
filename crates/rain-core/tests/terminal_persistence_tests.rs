use rain_core::{AgentSupervisor, ChatStore};
use rain_protocol::{AgentLaunchRequest, AgentStatus};
fn request(interactive: bool) -> AgentLaunchRequest {
    AgentLaunchRequest {
        name: "terminal".into(),
        provider: "custom".into(),
        command: "sh".into(),
        args: vec![
            "-c".into(),
            "test -t 0 && printf 'tty-ready'; read answer; printf 'received:%s' \"$answer\"".into(),
        ],
        cwd: None,
        env_vars: Default::default(),
        interactive,
        machine: None,
    }
}
#[tokio::test]
async fn pty_handles_prompt_without_newline_and_input() {
    let supervisor = AgentSupervisor::new();
    let mut events = supervisor.subscribe();
    let agent = supervisor.start_agent(request(true)).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let rain_protocol::AgentEvent::Output { line, .. } = events.recv().await.unwrap() {
                if line.contains("tty-ready") {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
    supervisor.send_input(agent.id, "/help").await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            if let rain_protocol::AgentEvent::Output { line, .. } = events.recv().await.unwrap() {
                if line.contains("received:/help") {
                    break;
                }
            }
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn history_restores_and_deleted_group_stays_deleted() {
    let path = std::env::temp_dir().join(format!("rain-{}.json", uuid::Uuid::new_v4()));
    let supervisor = AgentSupervisor::new();
    let store = ChatStore::with_supervisor(supervisor.clone());
    let mut req = request(false);
    req.args = vec!["-c".into(), "read answer".into()];
    let agent = supervisor.start_agent(req).await.unwrap();
    let group = store
        .create_conversation("Persistent group".into(), true, vec![agent.id.to_string()])
        .await;
    store.persist(&path).await.unwrap();
    let restored_supervisor = AgentSupervisor::new();
    let restored = ChatStore::restore(restored_supervisor.clone(), &path)
        .await
        .unwrap();
    assert_eq!(restored.list_conversations().await[0].id, group.id);
    assert!(!restored.get_messages(group.id).await.is_empty());
    let record = restored_supervisor.get_agent(agent.id).await.unwrap();
    assert_eq!(record.status, AgentStatus::Terminated);
    assert!(record.pid.is_none());
    restored.delete_conversation(group.id).await.unwrap();
    restored.persist(&path).await.unwrap();
    assert!(ChatStore::restore(AgentSupervisor::new(), &path)
        .await
        .unwrap()
        .list_conversations()
        .await
        .is_empty());
    supervisor.stop_agent(agent.id).await.unwrap();
    std::fs::remove_file(path).unwrap();
}
#[tokio::test]
async fn unknown_remote_machine_is_rejected() {
    let mut req = request(false);
    req.machine = Some(format!("unknown-{}", uuid::Uuid::new_v4()));
    assert!(AgentSupervisor::new()
        .start_agent(req)
        .await
        .unwrap_err()
        .contains("not configured"));
}
#[test]
fn ssh_argument_quoting_roundtrips_shell_metacharacters() {
    let value = "a'b $(exit 5); two words";
    let out = std::process::Command::new("sh")
        .args([
            "-c",
            &format!("printf %s {}", rain_core::terminal::quote(value)),
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap(), value);
}
