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
if '--help' in sys.argv:
    print('Options: --model --effort --disable-slash-commands');sys.exit(0)
prompt=sys.argv[sys.argv.index('--print')+1]
with open(os.environ['PROMPT_LOG'],'a') as f: f.write(json.dumps(prompt)+'\n')
# The fake CLI speaks the same final-response protocol as an installed provider.
reply='ack'
if 'START_AUTONOMOUS_TEST' in prompt and os.environ['ROLE']=='alice':
    run=re.search(r'"run_id":"([a-f0-9-]+)"',prompt).group(1)
    peer=re.search(r'Bob \| id=([a-f0-9-]+)',prompt).group(1)
    reply=json.dumps(dict(rain_coworker=1,run_id=run,kind='handoff',text='Review the shared plan',to=[peer],files=['plan.txt']))
elif 'New channel post from Alice:' in prompt:
    run=re.search(r'"run_id":"([a-f0-9-]+)"',prompt).group(1)
    reply=json.dumps(dict(rain_coworker=1,run_id=run,kind='human_input',text='Should we implement this plan?',to=[],files=[]))
print(json.dumps(dict(status='SUCCESS',response=reply)))
"##).unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
        let supervisor = AgentSupervisor::new();
        let mut chat = ChatStore::with_supervisor(supervisor.clone());
        chat.artifacts = rain_core::artifacts::ArtifactStore::at(root.join("artifacts"));
        let chat = Arc::new(chat);
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
        let c = self
            .chat
            .create_conversation(
                "Coworkers".into(),
                true,
                vec![self.alice.to_string(), self.bob.to_string()],
            )
            .await;
        self.chat
            .configure_collaboration(
                c.id,
                CollaborationRequest {
                    enabled: true,
                    max_turns: 8,
                },
            )
            .await
            .unwrap();
        c.id
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
                    flair: None,
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
            if self.supervisor.get_agent(id).await.unwrap().status
                == rain_protocol::AgentStatus::Running
            {
                self.supervisor.stop_agent(id).await.unwrap();
            }
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
        (team.bob.to_string(), vec!["../missing-artifact".into()]),
    ] {
        team.reply(id, run, CoworkerKind::Handoff, vec![target], files)
            .await;
        assert_eq!(team.state(id).await.remaining_turns, 1);
    }
    // Special device streams are not finite files; ordinary files outside the workspace are allowed.
    std::os::unix::fs::symlink(team.root.join("agy"), team.root.join("alice/escape")).unwrap();
    team.reply(
        id,
        run,
        CoworkerKind::Handoff,
        vec![team.bob.to_string()],
        vec!["/dev/null".into()],
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
        flair: None,
        rain_coworker: 1,
        run_id: run,
        kind: CoworkerKind::Discuss,
        text: "Review my evidence".into(),
        to: vec![team.bob.to_string()],
        files: vec![],
    };
    let publication = CoworkerEnvelope {
        flair: None,
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

#[tokio::test]
async fn natural_channels_broadcast_mentions_focus_and_human_controls_override() {
    let team = Team::new().await;
    let channel = team
        .chat
        .create_conversation(
            "Film lounge".into(),
            true,
            vec![team.alice.to_string(), team.bob.to_string()],
        )
        .await;
    assert_eq!(channel.collaboration.as_ref().unwrap().max_turns, 0);
    team.chat
        .send_human_message(
            channel.id,
            "Let's casually discuss films".into(),
            None,
            None,
        )
        .await
        .unwrap();
    let state = team.state(channel.id).await;
    let old = state.run_id.unwrap();
    assert!(state.addressed_ids.is_empty());
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.alice,
            conversation_id: channel.id,
            text: "I love thoughtful science fiction. What do you think?".into(),
        })
        .await;
    let posts = team.chat.get_messages(channel.id).await;
    assert_eq!(
        posts.last().unwrap().coworker.as_ref().unwrap().to,
        vec!["all"]
    );
    team.chat
        .send_human_message(
            channel.id,
            "@Bob give your own recommendation".into(),
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        team.state(channel.id).await.addressed_ids,
        vec![team.bob.to_string()]
    );
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.bob,
            conversation_id: channel.id,
            text: "My recommendation is Arrival.".into(),
        })
        .await;
    assert!(team
        .chat
        .get_messages(channel.id)
        .await
        .last()
        .unwrap()
        .coworker
        .as_ref()
        .unwrap()
        .to
        .is_empty());
    team.chat
        .record_event(AgentEvent::ChatReplyScoped {
            agent_id: team.alice,
            conversation_id: channel.id,
            run_id: Some(old),
            text: "An older response, superseded by the human.".into(),
        })
        .await;
    assert!(team
        .chat
        .get_messages(channel.id)
        .await
        .last()
        .unwrap()
        .coworker
        .is_none());
    let before = team.chat.get_messages(channel.id).await.len();
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.bob,
            conversation_id: channel.id,
            text: "<rain-silent/>".into(),
        })
        .await;
    assert_eq!(team.chat.get_messages(channel.id).await.len(), before);
    team.chat
        .send_human_message(channel.id, "/pause".into(), None, None)
        .await
        .unwrap();
    assert!(
        !team
            .chat
            .list_conversations()
            .await
            .into_iter()
            .find(|c| c.id == channel.id)
            .unwrap()
            .is_active
    );
    assert!(team
        .chat
        .send_human_message(channel.id, "continue anyway".into(), None, None)
        .await
        .is_err());
    team.chat
        .send_human_message(channel.id, "/resume".into(), None, None)
        .await
        .unwrap();
    team.chat
        .send_human_message(channel.id, "/autonomy off".into(), None, None)
        .await
        .unwrap();
    assert!(!team.state(channel.id).await.enabled);
    team.chat
        .send_human_message(channel.id, "/autonomy on".into(), None, None)
        .await
        .unwrap();
    assert!(team.state(channel.id).await.enabled);
    team.cleanup().await;
}

#[tokio::test]
async fn direct_agents_can_publish_system_files_outside_their_workspace() {
    let team = Team::new().await;
    let file = team.root.join("shared-song.wav");
    std::fs::write(&file, b"RIFF\0\0\0\0WAVEtest-binary\xff").unwrap();
    let direct = team
        .chat
        .get_or_create_direct_chat(&team.alice.to_string())
        .await;
    let text = format!(
        "Here is the file you requested. <rain-files>{}</rain-files>",
        serde_json::to_string(&vec![file.to_string_lossy().to_string()]).unwrap()
    );
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.alice,
            conversation_id: direct.id,
            text,
        })
        .await;
    let messages = team.chat.get_messages(direct.id).await;
    let message = messages.last().unwrap();
    assert_eq!(message.text, "Here is the file you requested.");
    assert_eq!(message.attachments.len(), 1);
    assert!(message.attachments[0].data_base64.is_none());
    let id = Uuid::parse_str(&message.attachments[0].id).unwrap();
    assert_eq!(
        team.chat.artifacts.read(id, 0, 1024).await.unwrap(),
        std::fs::read(&file).unwrap()
    );
    assert_eq!(
        team.chat.artifacts.get(id).await.unwrap().original_path,
        file.to_str().unwrap()
    );
    team.cleanup().await;
}

#[tokio::test]
async fn modes_interrupts_and_activity_are_scoped_and_survive_restore() {
    use rain_protocol::{ConversationMode, ModeRequest};
    let team = Team::new().await;
    let id = team.group().await;
    let direct = team
        .chat
        .get_or_create_direct_chat(&team.alice.to_string())
        .await;
    assert!(team
        .chat
        .set_mode(
            direct.id,
            ModeRequest {
                mode: ConversationMode::Evidence,
                context: None
            }
        )
        .await
        .is_err());
    assert!(
        team.chat
            .list_conversations()
            .await
            .into_iter()
            .find(|c| c.id == direct.id)
            .unwrap()
            .is_active
    );
    team.chat
        .set_mode(
            id,
            ModeRequest {
                mode: ConversationMode::Evidence,
                context: Some("Verify the proposed conclusion".into()),
            },
        )
        .await
        .unwrap();
    team.chat
        .send_human_message(
            id,
            "Check independently".into(),
            Some(team.alice.to_string()),
            None,
        )
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(250)).await;
    let prompt = std::fs::read_to_string(team.root.join("alice.jsonl")).unwrap();
    assert!(prompt.contains("Peer messages, consensus, repeated claims"));
    assert!(prompt.contains("Verify the proposed conclusion"));
    let d = team
        .chat
        .list_dispatches()
        .await
        .into_iter()
        .find(|d| d.conversation_id == id)
        .unwrap();
    team.chat
        .record_event(AgentEvent::Trace {
            agent_id: team.bob,
            dispatch_id: d.id,
            step: "wrong sender".into(),
        })
        .await;
    team.chat
        .record_event(AgentEvent::Trace {
            agent_id: team.alice,
            dispatch_id: d.id,
            step: "read_file · completed · /tmp/source".into(),
        })
        .await;
    assert_eq!(
        team.chat.conversation_activity(id).await.unwrap()["traces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        team.chat.conversation_activity(direct.id).await.unwrap()["traces"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(team
        .chat
        .control_member(id, Uuid::new_v4(), false)
        .await
        .is_err());
    team.chat
        .control_member(id, team.alice, false)
        .await
        .unwrap();
    assert!(team
        .state(id)
        .await
        .interrupted_ids
        .contains(&team.alice.to_string()));
    let before = team.chat.list_dispatches().await.len();
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.alice,
            conversation_id: id,
            text: "@Bob continue despite interruption".into(),
        })
        .await;
    assert_eq!(team.chat.list_dispatches().await.len(), before);
    team.chat
        .send_human_message(id, "New direction".into(), None, None)
        .await
        .unwrap();
    assert!(team.state(id).await.interrupted_ids.is_empty());
    let state = team.root.join("history.json");
    team.chat.persist(&state).await.unwrap();
    let restored = ChatStore::restore(AgentSupervisor::new(), &state)
        .await
        .unwrap();
    assert_eq!(
        restored.conversation_activity(id).await.unwrap()["traces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        restored
            .list_conversations()
            .await
            .into_iter()
            .find(|c| c.id == id)
            .unwrap()
            .collaboration
            .unwrap()
            .mode,
        ConversationMode::Evidence
    );
    team.chat.control_member(id, team.bob, true).await.unwrap();
    assert_ne!(
        team.supervisor.get_agent(team.bob).await.unwrap().status,
        rain_protocol::AgentStatus::Running
    );
    team.cleanup().await;
}

#[tokio::test]
async fn native_settings_are_private_and_flairs_are_optional_hints() {
    let team = Team::new().await;
    let group = team.group().await;
    let direct = team
        .chat
        .get_or_create_direct_chat(&team.alice.to_string())
        .await;
    let catalog = team.chat.command_catalog(direct.id).await.unwrap();
    assert!(catalog["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "/model"));
    let channel = team.chat.command_catalog(group).await.unwrap();
    assert!(!channel["commands"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "/model"));
    assert!(team
        .chat
        .send_human_message(group, "/model example".into(), None, None)
        .await
        .is_err());
    team.chat
        .send_human_message(direct.id, "/model example-model".into(), None, None)
        .await
        .unwrap();
    team.chat
        .send_human_message(direct.id, "/effort high".into(), None, None)
        .await
        .unwrap();
    assert!(team
        .chat
        .send_human_message(direct.id, "/model --injected-flag".into(), None, None)
        .await
        .is_err());
    assert!(team
        .chat
        .send_human_message(direct.id, "/effort invented".into(), None, None)
        .await
        .is_err());
    let c = team
        .chat
        .list_conversations()
        .await
        .into_iter()
        .find(|c| c.id == direct.id)
        .unwrap();
    assert_eq!(c.native_settings.model.as_deref(), Some("example-model"));
    assert_eq!(c.native_settings.effort.as_deref(), Some("high"));
    team.chat
        .send_human_message(group, "@Alice review the claim".into(), None, None)
        .await
        .unwrap();
    team.chat
        .record_event(AgentEvent::ChatReply {
            agent_id: team.alice,
            conversation_id: group,
            text: "[review] This claim needs an independent source.".into(),
        })
        .await;
    let message = team.chat.get_messages(group).await.pop().unwrap();
    assert_eq!(message.text, "This claim needs an independent source.");
    assert!(matches!(
        message.coworker.unwrap().flair,
        Some(rain_protocol::MessageFlair::Review)
    ));
    team.cleanup().await;
}
