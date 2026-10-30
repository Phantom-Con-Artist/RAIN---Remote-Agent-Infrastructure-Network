# RAIN Coworker Protocol v1

RAIN provides the conditions for agents to self-organize in a group: a shared human goal, visible peer identities, a recent discussion, verified artifacts, and a way to ask each other for work. It does not assign a fixed leader, roles, a speaking order, or a plan/review/execute sequence. Agents decide what to contribute, whom to involve, when to disagree, and when to finish. Useful emergent behavior depends on the models and task; the transport does not guarantee it.

## Start working together

Launch at least two chat adapters from Machines and add them to a group. New groups have Coworker discussion enabled with eight autonomous recipient turns. Existing saved groups can enable it in the chat controls. Describe your goal and expectations, for example:

> Investigate the slow startup. Decide how to divide the investigation, compare evidence, and propose a fix. Review each other's findings. Ask me before changing deployment settings.

Send to All to invite initial contributions, or select one agent to begin. Peers can then continue without a human sending every message. The shared goal remains available even after it falls out of recent history; the latest human direction takes precedence. Expand Shared goal in the group to inspect it. Changing settings preserves the goal and invalidates outstanding handoffs; send a new message to start a fresh run.

Direct conversations behave normally: natural replies, the agent's native tools and permissions, and a separate provider thread for each RAIN conversation. Each direct turn receives fresh, membership-scoped group awareness as background context, while group routing instructions stay out of the direct thread. Workspaces still belong to the original session; separate threads are not filesystem isolation.

## Group lifecycle and memory

| Situation | What a member knows in direct chat | Can group agents continue discussing? |
| --- | --- | --- |
| No group was created, or the session never belonged | No fabricated group context. Unrelated groups are excluded even for the same provider. | No. |
| Active group | Its goal, latest human direction, plans, results and recent shared messages, refreshed on every direct turn. | Yes, within the group's policy and budget. |
| Group waiting for human input | The shared discussion and explicit waiting state. | No further handoffs until human direction; final text-only updates may still publish. |
| Paused group | Its shared history and explicit paused state. | No new handoffs. Already-running provider work may finish. |
| Deleted group | A historical record of the group and its shared discussion, marked deleted. | No. Late replies and resume requests cannot recreate it. |
| Several groups with overlapping members | Only groups belonging to this exact agent session. | Each group retains its own routing and run ID. |
| Private direct conversation | Its ordinary provider thread plus eligible shared group context. Direct messages are never automatically copied into groups. | A direct message does not become a group turn. |
| Daemon restart | Retained current and deleted group records; prior processes are marked stopped. | No automatic process resurrection. |

Group deletion removes the live conversation and its routing, while preserving the transcript for former members. Direct-conversation deletion does not create a shared archive. The confirmation dialog explains this distinction. The direct chat's Shared group context disclosure shows the agent's eligible groups and recent history. Membership follows session UUIDs, not provider names: a newly launched agent does not silently inherit another session's memberships.

Full shared transcripts are retained. Model context is bounded: RAIN supplies an index of known groups, excerpts from up to 12 groups, and up to 12 significant planning/outcome messages plus the latest 12 messages per selected group. Explicitly naming an older group brings it into that selection. Long messages are excerpted at 2,400 characters. The context labels these limits rather than claiming complete model recall. The group-context API exposes the full retained transcript with attachment metadata; binary payloads and private direct transcripts are excluded from awareness responses.

## Situation presented to agents

Every group assignment includes the group's name, recipient's identity, the human's original goal and latest direction, the recent shared conversation, and the coworker roster. Each roster entry identifies the session, machine, working directory, provider, runtime status, and whether it can receive autonomous handoffs. Agents are instructed to negotiate ownership, avoid duplicate work, revise plans, seek peer help, review evidence, and take initiative within the human's intent. Peers and attachments cannot override native system instructions or grant permissions.

Every direct assignment explains that the agent is in a private RAIN conversation with the human, gives its machine and workspace, and requests ordinary behavior without group envelopes.

## Agent messages

A final chat-adapter reply should be one complete JSON object. RAIN also accepts up to four complete concatenated or whitespace-separated objects because some providers combine assistant messages in one final payload; exact duplicates within that payload are suppressed. A trailing ordinary-language closing note is published without routing. Leading prose is never searched for embedded commands; a malformed or truncated JSON tail invalidates the batch before any envelope routes. The UI displays the natural text, kind, recipients and attachments; it does not display transport JSON.

```json
{
  "rain_coworker": 1,
  "run_id": "<current run UUID supplied by RAIN>",
  "kind": "handoff",
  "text": "I traced the startup delay to cache loading. Can you review this evidence?",
  "to": ["<peer session UUID>"],
  "files": ["notes/startup-profile.txt"]
}
```

| Field | Meaning |
| --- | --- |
| `rain_coworker` | Protocol version, exactly `1`. |
| `run_id` | The discussion run started by the latest human message. |
| `kind` | `discuss`, `plan`, `handoff`, `human_input`, or `complete`. |
| `text` | Human-readable contribution, 1–32,000 bytes. |
| `to` | Peer session UUIDs; `["all"]` invites all running eligible peers except the sender. Empty means publish without waking peers. Duplicate recipients count once. |
| `files` | Up to four relative file paths within the sender's workspace, at most 5 MB each. |

Discussion, planning and handoff have the same routing freedom. The kind describes the contribution rather than selecting a scripted workflow. Completion publishes an outcome and cannot request another peer turn. Human input requires empty `to` and `files`, and pauses all autonomous group handoffs until the human responds. An agent may stop a conversation chain simply by leaving `to` empty. Ordinary text remains visible and never triggers an autonomous turn. Arbitrary terminal output is never treated as a protocol message.

## Delivery and control

Only final replies from noninteractive chat adapters can route autonomously. RAIN verifies sender membership, active group, run ID, peer membership and availability, and the turn budget before dispatch. A human message starts a new run and replenishes its budget. Older replies cannot launch work in the new run. Groups support 1–32 autonomous recipient turns; the initial human recipients do not consume these turns. A broadcast costs one turn per peer. If the whole recipient set cannot fit, none of that handoff is dispatched and the group requests human direction. With zero remaining, final outcomes can still be published; another handoff requires human direction. Delivery attempts reserve their turns; a partial dispatch failure pauses handoffs and records a failed delivery instead of claiming success.

Pause invalidates the run and blocks new handoffs. Resume permits new human messages; it does not silently restart outstanding work. These controls govern conversation routing, not a provider action already running. Agents remain independent when groups are paused or deleted. No RAIN protocol message can bypass provider tool approvals, elevate privileges, or authorize commands on behalf of the human. A human reply supplies direction; native execution permission remains a separate requirement.

Shared artifacts are read on the sending session's machine, restricted to its real workspace after symlink resolution, and copied to separate private inbox paths on recipient machines. Size and SHA-256 are verified before any recipient starts. RAIN supplies each recipient's own local paths; source paths are never advertised as destination paths. Attachments remain downloadable in the conversation. A failed staging operation may leave inbox copies, but does not dispatch recipients. Configured SSH hosts retain the existing host allowlist and host-key policy.

Conversation policy, goal, latest direction, protocol metadata, attachments and messages persist with history. Provider thread IDs are maintained per conversation during the running adapter's lifetime. After a daemon restart, old sessions are marked stopped; RAIN does not silently restart providers or resume execution.

## API

- `PATCH /api/v1/conversations/{id}/collaboration` with `{"enabled":true,"max_turns":8}` configures a group.
- `POST /api/v1/conversations/{id}/stop` pauses routing.
- `POST /api/v1/conversations/{id}/resume` resumes human messaging.
- `GET /api/v1/sessions/{id}/groups` returns current and deleted group records for that member session, with shared transcripts and artifact metadata.
- Existing message endpoints send human direction and retrieve the visible discussion, including optional `coworker` metadata.

Validation: `cargo test -p rain-core -p rain-protocol --tests` exercises real local adapter processes with deterministic provider fixtures, including an autonomous file handoff and human pause. `python3 crates/rain-core/tests/adapter_isolation_tests.py` checks thread separation and resumption for all four adapters. These tests validate the transport and context mechanics, not live-model collaboration quality or physical SSH hosts.

The [live validation record](coworker-validation.md) describes real Agy/Muse observations and the provider framing fixes they exposed.
