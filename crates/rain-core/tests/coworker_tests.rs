use rain_core::{AgentSupervisor, ChatStore};
use rain_protocol::{
    AgentEvent, AgentLaunchRequest, CollaborationRequest, CoworkerEnvelope, CoworkerKind,
};
use std::{path::PathBuf, sync::Arc, time::Duration};
use uuid::Uuid;

struct Team {
    chat: Arc<ChatStore>,
    supervisor: AgentSupervisor,
    root: PathBuf,
    alice: Uuid,
    bob: Uuid,
}
impl Team {
    async fn new() -> Self {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!("rain-coworker-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let executable = root.join("agy");
        std::fs::write(&executable, r##"#!/usr/bin/env python3
import sys,json,os,re
prompt=sys.argv[sys.argv.index('--print')+1]
with open(os.environ['PROMPT_LOG'],'a') as f: f.write(json.dumps(prompt)+'\n')
# The fake CLI speaks the same final-response protocol as an installed provider.
reply='ack'
if 'START_AUTONOMOUS_TEST' in prompt and os.environ['ROLE']=='alice':
    run=re.search(r'"run_id":"([a-f0-9-]+)"',prompt).group(1)
    peer=re.search(r'Bob \| id=([a-f0-9-]+)',prompt).group(1)
    reply=json.dumps(dict(rain_coworker=1,run_id=run,kind='handoff',text='Review the shared plan',to=[peer],files=['plan.txt']))
elif 'Alice asks you:' in prompt:
    run=re.search(r'"run_id":"([a-f0-9-]+)"',prompt).group(1)
    reply=json.dumps(dict(rain_coworker=1,run_id=run,kind='human_input',text='Should we implement this plan?',to=[],files=[]))
print(json.dumps(dict(status='SUCCESS',response=reply)))
"##).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let supervisor = AgentSupervisor::new();
        let chat = Arc::new(ChatStore::with_supervisor(supervisor.clone()));
        let mut ids = vec![];
        for (name, role) in [("Alice", "alice"), ("Bob", "bob")] {
            let workspace = root.join(role);
            std::fs::create_dir(&workspace).unwrap();
            std::fs::write(
                workspace.join("plan.txt"),
                b"Plan: discuss, review, ask the human.\0\xff",
            )
            .unwrap();
            ids.push(
                supervisor
                    .start_agent(AgentLaunchRequest {
                        name: name.into(),
                        provider: "agy_chat".into(),
                        command: executable.to_string_lossy().into(),
                        args: vec![],
                        cwd: Some(workspace.to_string_lossy().into()),
                        machine: None,
                        interactive: false,
                        env_vars: [
                            ("ROLE".into(), role.into()),
                            (
                                "PROMPT_LOG".into(),
                                root.join(format!("{role}.jsonl")).to_string_lossy().into(),
                            ),
                        ]
                        .into(),
                    })
                    .await
                    .unwrap()
                    .id,
            );
        }
        Self {
            chat,
            supervisor,
            root,
            alice: ids[0],
            bob: ids[1],
        }
    }
    async fn group(&self) -> Uuid {
        self.chat
            .create_conversation(
                "Coworkers".into(),
                true,
                vec![self.alice.to_string(), self.bob.to_string()],
            )
            .await
            .id
    }
    async fn state(&self, id: Uuid) -> rain_protocol::CollaborationState {
        self.chat
            .list_conversations()
            .await
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .collaboration
            .unwrap()
    }
    async fn reply(
        &self,
        id: Uuid,
        run_id: Uuid,
        kind: CoworkerKind,
        to: Vec<String>,
        files: Vec<String>,
    ) {
        self.chat
            .record_event(AgentEvent::ChatReply {
                agent_id: self.alice,
                conversation_id: id,
                text: serde_json::to_string(&CoworkerEnvelope {
                    rain_coworker: 1,
                    run_id,
                    kind,
                    text: "Here is my plan".into(),
                    to,
                    files,
                })
                .unwrap(),
            })
            .await;
    }
    async fn cleanup(self) {
        for id in [self.alice, self.bob] {
            self.supervisor.stop_agent(id).await.unwrap();
        }
        std::fs::remove_dir_all(self.root).unwrap();
    }
}

#[tokio::test]
async fn coworkers_autonomously_share_verified_files_and_wait_for_the_human() {
    let team = Team::new().await;
    let id = team.group().await;
    let unrelated = team
        .chat
        .get_or_create_direct_chat(&team.bob.to_string())
        .await;
    let mut events = team.supervisor.subscribe();
    let observer = team.chat.clone();
    let pump = tokio::spawn(async move {
        while let Ok(e) = events.recv().await {
            observer.record_event(e).await;
        }
    });
    team.chat
        .send_human_message(
            id,
            "START_AUTONOMOUS_TEST".into(),
            Some(team.alice.to_string()),
            None,
        )
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while !team.state(id).await.waiting_for_human {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let state = team.state(id).await;
    assert_eq!(state.remaining_turns, 7);
    assert_eq!(
        state.human_request.as_deref(),
        Some("Should we implement this plan?")
    );
    let messages = team.chat.get_messages(id).await;
    let handoff = messages
        .iter()
        .find(|m| {
            m.coworker
                .as_ref()
                .map(|e| e.kind == CoworkerKind::Handoff)
                .unwrap_or(false)
        })
        .unwrap();
    assert_eq!(handoff.attachments.len(), 1);
    assert_eq!(handoff.attachments[0].sha256.as_ref().unwrap().len(), 64);
    let logs = std::fs::read_to_string(team.root.join("bob.jsonl")).unwrap();
    let prompt: String = serde_json::from_str(logs.lines().last().unwrap()).unwrap();
    let path = prompt
        .split("RAIN_FILE: ")
        .nth(1)
        .unwrap()
        .lines()
        .next()
        .unwrap();
    assert_eq!(
        std::fs::read(path).unwrap(),
        std::fs::read(team.root.join("alice/plan.txt")).unwrap()
    );
    assert!(!path.starts_with(team.root.join("alice").to_str().unwrap()));
    assert!(!team
        .chat
        .get_messages(unrelated.id)
        .await
        .iter()
        .any(|m| m.text.contains("shared plan")));
    team.chat
        .send_human_message(
            id,
            "Proceed with review only".into(),
            Some(team.bob.to_string()),
            None,
        )
        .await
        .unwrap();
    let next = team.state(id).await;
    assert!(!next.waiting_for_human);
    assert_ne!(state.run_id, next.run_id);
    assert_eq!(next.remaining_turns, 8);
    pump.abort();
    team.cleanup().await;
}

#[tokio::test]
async fn protocol_enforces_membership_workspace_budget_pause_and_run_boundaries() {
    let team = Team::new().await;
    let id = team.group().await;
    team.chat
        .configure_collaboration(
            id,
            CollaborationRequest {
                enabled: true,
                max_turns: 1,
            },
        )
        .await
        .unwrap();
    team.chat
        .send_human_message(id, "Plan".into(), Some(team.alice.to_string()), None)
        .await
        .unwrap();
    let run = team.state(id).await.run_id.unwrap();
    for (target, files) in [
        (team.alice.to_string(), vec![]),
        (Uuid::new_v4().to_string(), vec![]),
        (team.bob.to_string(), vec!["../agy".into()]),
    ] {
        team.reply(id, run, CoworkerKind::Handoff, vec![target], files)
            .await;
        assert_eq!(team.state(id).await.remaining_turns, 1);
    }
    // Resolving symlinks must not allow a file outside the agent's workspace.
    std::os::unix::fs::symlink(team.root.join("agy"), team.root.join("alice/escape")).unwrap();
    team.reply(
        id,
        run,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec!["escape".into()],
    )
    .await;
    assert_eq!(team.state(id).await.remaining_turns, 1);
    team.reply(id, run, CoworkerKind::Plan, vec!["all".into()], vec![])
        .await;
    assert_eq!(team.state(id).await.remaining_turns, 0);
    team.reply(
        id,
        run,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec![],
    )
    .await;
    assert!(team.state(id).await.waiting_for_human);
    team.chat
        .send_human_message(id, "Next plan".into(), Some(team.alice.to_string()), None)
        .await
        .unwrap();
    team.reply(
        id,
        run,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec![],
    )
    .await;
    assert_eq!(team.state(id).await.remaining_turns, 1);
    let current = team.state(id).await.run_id.unwrap();
    team.chat.stop_conversation(id).await.unwrap();
    team.reply(
        id,
        current,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec![],
    )
    .await;
    assert_eq!(team.state(id).await.remaining_turns, 1);
    team.chat.resume_conversation(id).await.unwrap();
    assert!(team.state(id).await.run_id.is_none());
    team.chat
        .configure_collaboration(
            id,
            CollaborationRequest {
                enabled: false,
                max_turns: 1,
            },
        )
        .await
        .unwrap();
    team.reply(
        id,
        current,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec![],
    )
    .await;
    assert_eq!(team.state(id).await.remaining_turns, 1);
    let snapshot = team.root.join("history.json");
    team.chat.persist(&snapshot).await.unwrap();
    let restored = ChatStore::restore(AgentSupervisor::new(), &snapshot)
        .await
        .unwrap();
    assert!(
        !restored.list_conversations().await[0]
            .collaboration
            .as_ref()
            .unwrap()
            .enabled
    );
    assert!(restored
        .get_messages(id)
        .await
        .iter()
        .any(|m| m.coworker.is_some()));
    team.cleanup().await;
}

async fn await_prompt(team: &Team, needle: &str) -> String {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(log) = std::fs::read_to_string(team.root.join("alice.jsonl")) {
                for line in log.lines().rev() {
                    let prompt: String = serde_json::from_str(line).unwrap();
                    if prompt.contains(needle) {
                        return prompt;
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn direct_chats_know_only_their_memberships_and_remember_deleted_group_history() {
    let team = Team::new().await;
    assert!(team.chat.group_context(team.alice).await.is_empty());
    let direct = team
        .chat
        .get_or_create_direct_chat(&team.alice.to_string())
        .await;
    team.chat
        .send_human_message(direct.id, "BEFORE_ANY_GROUP".into(), None, None)
        .await
        .unwrap();
    let before = await_prompt(&team, "BEFORE_ANY_GROUP").await;
    assert!(!before.contains("Shared group awareness"));
    assert!(!before.contains("Coworker Protocol"));
    let id = team.group().await;
    let secret = team
        .chat
        .create_conversation(
            "Bob's unrelated private group".into(),
            true,
            vec![team.bob.to_string()],
        )
        .await;
    team.chat
        .send_human_message(
            id,
            "Diagnose the startup delay; review the plan together".into(),
            Some(team.alice.to_string()),
            None,
        )
        .await
        .unwrap();
    let run = team.state(id).await.run_id.unwrap();
    team.reply(id, run, CoworkerKind::Plan, vec![], vec![])
        .await;
    // Direct messages never enter the group's shared history.
    team.chat
        .send_human_message(direct.id, "PRIVATE_DIRECT_MARKER".into(), None, None)
        .await
        .unwrap();
    let active = await_prompt(&team, "Human message:\nPRIVATE_DIRECT_MARKER").await;
    assert!(active.contains("Group: Coworkers"));
    assert!(active.contains("State: ACTIVE"));
    assert!(active.contains("Here is my plan"));
    assert!(active.contains("Diagnose the startup delay"));
    assert!(!active.contains(&secret.title));
    assert!(!team
        .chat
        .get_messages(id)
        .await
        .iter()
        .any(|m| m.text.contains("PRIVATE_DIRECT_MARKER")));
    team.chat.stop_conversation(id).await.unwrap();
    team.chat
        .send_human_message(direct.id, "PAUSED_GROUP_QUERY".into(), None, None)
        .await
        .unwrap();
    assert!(await_prompt(&team, "PAUSED_GROUP_QUERY")
        .await
        .contains("State: PAUSED"));
    team.chat.delete_conversation(id).await.unwrap();
    assert!(team.chat.get_messages(id).await.is_empty());
    let historical = team.chat.group_context(team.alice).await;
    assert_eq!(historical.len(), 1);
    assert!(historical[0].deleted_at.is_some());
    assert!(historical[0].messages.iter().any(|m| m.coworker.is_some()));
    team.chat
        .send_human_message(direct.id, "DELETED_GROUP_QUERY".into(), None, None)
        .await
        .unwrap();
    let deleted = await_prompt(&team, "DELETED_GROUP_QUERY").await;
    assert!(deleted.contains("State: DELETED"));
    assert!(deleted.contains("Here is my plan"));
    assert!(!deleted.contains(&secret.title));
    // A deleted group cannot be revived by late agent output or the resume endpoint.
    team.reply(
        id,
        run,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec![],
    )
    .await;
    assert!(team.chat.resume_conversation(id).await.is_err());
    assert!(team.chat.get_messages(id).await.is_empty());
    let snapshot = team.root.join("memory.json");
    team.chat.persist(&snapshot).await.unwrap();
    let restored = ChatStore::restore(AgentSupervisor::new(), &snapshot)
        .await
        .unwrap();
    assert_eq!(restored.group_context(team.alice).await.len(), 1);
    assert!(restored.group_context(team.alice).await[0]
        .deleted_at
        .is_some());
    team.cleanup().await;
}

#[tokio::test]
async fn provider_concatenated_envelopes_route_once_and_malformed_suffixes_do_not_route() {
    let team = Team::new().await;
    let id = team.group().await;
    team.chat
        .send_human_message(
            id,
            "Test provider response framing".into(),
            Some(team.alice.to_string()),
            None,
        )
        .await
        .unwrap();
    let run = team.state(id).await.run_id.unwrap();
    let handoff = CoworkerEnvelope {
        rain_coworker: 1,
        run_id: run,
        kind: CoworkerKind::Discuss,
        text: "Review my evidence".into(),
        to: vec![team.bob.to_string()],
        files: vec![],
    };
    let publication = CoworkerEnvelope {
        text: "Review shared; awaiting peer feedback".into(),
        to: vec![],
        ..handoff.clone()
    };
    let first = serde_json::to_string(&handoff).unwrap();
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.alice,
            conversation_id: id,
            text: format!("{first}{{\"rain_coworker\":"),
        })
        .await;
    assert_eq!(team.state(id).await.remaining_turns, 8);
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.alice,
            conversation_id: id,
            text: format!(
                "{first}{first}{}Artifact verified present; awaiting peer review.",
                serde_json::to_string(&publication).unwrap()
            ),
        })
        .await;
    assert_eq!(team.state(id).await.remaining_turns, 7);
    assert_eq!(
        team.chat
            .get_messages(id)
            .await
            .iter()
            .filter(|m| m.coworker.is_some())
            .count(),
        2
    );
    assert!(team
        .chat
        .get_messages(id)
        .await
        .iter()
        .any(|m| m.coworker.is_none()
            && m.text == "Artifact verified present; awaiting peer review."));
    team.cleanup().await;
}
