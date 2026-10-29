use rain_core::{AgentSupervisor, ChatStore};
use rain_protocol::AgentLaunchRequest;
use std::sync::Arc;

#[tokio::test]
async fn adapters_route_group_and_direct_replies_without_crosstalk() {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("rain-adapter-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let agy = root.join("agy");
    std::fs::write(&agy,"#!/usr/bin/env python3\nimport sys,json\nprompt=sys.argv[sys.argv.index('--print')+1]\nprint(json.dumps({'conversation_id':'test-thread','status':'SUCCESS','response':'AGY: '+prompt}))\n").unwrap();
    let muse = root.join("muse");
    std::fs::write(&muse,"#!/usr/bin/env python3\nimport sys,json\nprompt=open(sys.argv[sys.argv.index('--prompt-file')+1]).read()\nprint(json.dumps({'payload_type':'run.terminal.completed','payload':{'terminal':'completed','text':'MUSE: '+prompt}}))\n").unwrap();
    for p in [&agy, &muse] {
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)).unwrap()
    }
    let supervisor = AgentSupervisor::new();
    let chat = Arc::new(ChatStore::with_supervisor(supervisor.clone()));
    let mut events = supervisor.subscribe();
    let observer = chat.clone();
    let pump = tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            observer.record_event(event).await
        }
    });
    let mut ids = vec![];
    for (name, command, provider) in [("agy", &agy, "agy_chat"), ("muse", &muse, "muse_chat")] {
        let agent = supervisor
            .start_agent(AgentLaunchRequest {
                name: name.into(),
                command: command.to_string_lossy().into_owned(),
                provider: provider.into(),
                args: vec![],
                cwd: Some(root.to_string_lossy().into_owned()),
                interactive: false,
                machine: None,
                env_vars: Default::default(),
            })
            .await
            .unwrap();
        ids.push(agent.id);
    }
    let group = chat
        .create_conversation(
            "Team".into(),
            true,
            ids.iter().map(|id| id.to_string()).collect(),
        )
        .await;
    let direct = chat.get_or_create_direct_chat(&ids[0].to_string()).await;
    chat.send_human_message(group.id, "group task".into(), None, None)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let msgs = chat.get_messages(group.id).await;
            if msgs
                .iter()
                .filter(|m| {
                    m.text.contains("group task")
                        && m.sender_type == rain_protocol::SenderType::Agent
                })
                .count()
                == 2
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await
        }
    })
    .await
    .unwrap();
    assert!(!chat
        .get_messages(direct.id)
        .await
        .iter()
        .any(|m| m.text.contains("group task")));
    chat.send_human_message(group.id, "@agy targeted".into(), None, None)
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if chat
                .get_messages(group.id)
                .await
                .iter()
                .any(|m| m.sender_id == ids[0].to_string() && m.text.contains("targeted"))
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await
        }
    })
    .await
    .unwrap();
    assert!(!chat
        .get_messages(group.id)
        .await
        .iter()
        .any(|m| m.sender_id == ids[1].to_string() && m.text.contains("targeted")));
    for id in ids {
        supervisor.stop_agent(id).await.unwrap()
    }
    pump.abort();
    std::fs::remove_dir_all(root).unwrap();
}
