# RAIN — Remote Agent Infrastructure Network

**Project Vision & Product Specification**

Version: 0.2  
Status: Concept / Early Planning  
Product Category: Self-hosted, cross-platform AI agent messenger and orchestration interface

---

## 1. What Is RAIN?

RAIN is a self-hosted messaging application that lets users communicate with, coordinate, and monitor AI coding agents running on their own computers.

It provides a familiar, WhatsApp-like interface where people, computers, and AI agent sessions participate in conversations together.

Users can connect multiple computers, launch multiple AI sessions on each machine, organize sessions into temporary group conversations, exchange files, delegate work, and observe activity — all without needing to understand the underlying infrastructure.

RAIN is not another AI model, coding IDE, terminal emulator, or remote desktop application.

**RAIN is the conversational interface for coordinating AI work across machines.**

### Core principle

> WhatsApp-level simplicity on the surface. Serious engineering underneath.

---

## 2. The Problem RAIN Solves

AI coding tools such as Codex CLI and Claude Code can already perform substantial work independently. They can execute tools, modify files, and—in supported configurations—delegate tasks to their own subagents.

However, running multiple AI sessions across different computers creates a coordination problem.

Users may need to:

- Switch between multiple terminals and AI sessions.
- Manually transfer instructions between agents.
- Check which agents are busy, idle, waiting, or disconnected.
- Copy files and reports between machines.
- Track which session produced a particular result.
- Coordinate parallel work without causing conflicting file modifications.
- Return to a computer to inspect progress or approve an action.

RAIN brings these interactions into a single conversational interface.

Instead of manually managing terminals and machines, users communicate with their AI sessions through familiar chats and groups.

---

## 3. The User Experience

### 3.1 Simple onboarding

Connecting a computer should feel similar to pairing WhatsApp Web.

1. Install RAIN on the computer.
2. Open RAIN on the phone.
3. Scan a QR code displayed by the computer.
4. Approve the connection.
5. Select or launch the AI sessions to make available.

Additional computers can be connected through the same process.

RAIN handles device identity, secure communication, session discovery, and platform-specific implementation behind the scenes.

The user should not need to configure ports, manually enter IP addresses, write scripts, or understand networking.

### 3.2 Familiar messaging

RAIN uses familiar messaging concepts:

- One-to-one conversations.
- Temporary group chats.
- Contacts and connected computers.
- Message delivery status.
- Online and activity indicators.
- File attachments.
- Conversation history.
- Notifications and requests for attention.

The interface should feel like a messaging application, not an infrastructure management dashboard.

### 3.3 Temporary agent groups

Users can create a group such as:

**Website Launch Team**

Participants might include:

- Codex — Frontend, running on the office PC.
- Codex — Backend, running on the office PC.
- Claude Code — Reviewer, running on the laptop.
- Codex — Tester, running on another computer.
- The human user.

The user can send a message to the group, message a particular session, or direct an instruction to selected participants.

Groups are organizational spaces, not the owners of the underlying agent sessions. Removing a participant from a group or deleting a group should not automatically terminate its sessions.

### 3.4 Presence and activity

RAIN should communicate the state of each participant clearly.

Possible states include:

- **Online:** Connected and available.
- **Thinking:** Processing or preparing a response.
- **Working:** Executing a task.
- **Responding:** Sending a response or result.
- **Waiting:** Blocked on user input, approval, or another dependency.
- **Completed:** The current task has reported completion.
- **Disconnected:** The session or host is unreachable.
- **Error:** An execution or integration failure has occurred.

These states must reflect real signals from the provider, session manager, and connection layer. RAIN must not invent activity or mark a task complete merely because an agent has stopped producing messages.

Where a provider does not expose a particular state, RAIN should communicate uncertainty rather than pretend to know.

---

## 4. Multiple AI Sessions Per Computer

A single computer can host multiple independent AI sessions.

For example, one Linux workstation might run:

- Codex — Frontend
- Codex — Backend
- Codex — Research
- Claude Code — Reviewer
- Another Codex session for testing

Each session can participate independently in RAIN conversations.

Every session should have its own:

- Session identity.
- Provider and integration type.
- Working directory.
- Conversation or thread identifier.
- Execution state.
- Permissions and approval policy.
- Message delivery state.
- Artifact and task associations.

RAIN should support launching new sessions and attaching to existing sessions when the provider's supported interface permits it.

Launching multiple command-line processes is only part of the problem. RAIN must also reliably observe, communicate with, and manage each session through a supported integration.

### Provider integrations

RAIN should use provider-specific adapters rather than assuming all AI tools behave identically.

**Codex CLI**

Investigate the Codex App Server protocol and its supported thread, event, command, and approval workflows. Use documented capabilities wherever possible.

**Claude Code**

Investigate supported CLI session management, structured output, streaming events, and available control mechanisms.

**Future providers**

The architecture should permit new integrations without redesigning the messaging experience.

Provider features differ. RAIN should expose only capabilities an integration can reliably support and should clearly indicate limitations.

---

## 5. Native Subagents

An essential part of RAIN is recognizing that an AI session may already have its own internal team.

For example:

- Codex Session A launches Subagent A1 and Subagent A2.
- Codex Session B launches Subagent B1 and Subagent B2.
- Both parent sessions participate in a RAIN group.

The parent sessions remain responsible for their own subagents and native orchestration.

RAIN does not need to replace this mechanism.

### Two levels of participation

**Level 1: RAIN participants**

Independent Codex or Claude Code sessions appear as participants in a group conversation.

**Level 2: Provider-managed subagents**

Subagents are created, supervised, and coordinated by their parent sessions using the provider's own capabilities.

By default, the parent session can summarize subagent progress and communicate relevant results to RAIN.

An advanced mode could expose individual subagents as separate participants when the provider exposes sufficient information and control to support it.

RAIN must not assume that every provider allows external applications to address, interrupt, or inspect individual subagents.

---

## 6. Cross-Machine Coordination

RAIN should make machine boundaries largely invisible to the user.

Consider three connected computers:

- Office PC running several Codex sessions.
- Home workstation running Claude Code.
- Laptop running additional Codex sessions.

The user creates a RAIN group and adds selected sessions from all three machines.

The group can exchange messages, progress reports, and files even though the participants execute on different computers.

### Example workflow

The user sends:

"Research three competitors, compare their pricing, and prepare a report."

1. A research session investigates competitors.
2. A second session gathers pricing information.
3. A third session organizes the results into a report.
4. Sessions exchange messages and artifacts through RAIN.
5. A reviewer session checks the final result.
6. The user reviews the report in the group conversation.

This workflow requires explicit task assignment, message routing, artifact permissions, and delivery tracking. Merely placing agents in a group does not guarantee that they will automatically collaborate effectively.

RAIN should help make collaboration reliable without pretending every provider has identical capabilities.

---

## 7. Messaging and Task Coordination

RAIN needs a messaging layer that supports both human-to-agent and agent-to-agent communication.

Messages should be targetable to:

- The entire group.
- A specific session.
- Selected sessions.
- Another session through an explicit handoff.

Messages should carry delivery and processing information so users can distinguish between a message that was sent, delivered, acknowledged, or acted upon.

### Queue and redirect behavior

RAIN may support two ways to handle messages sent to busy agents:

**Queue:** Deliver the message when the session is ready or at an appropriate provider-supported boundary.

**Redirect / Steer:** Attempt to redirect an active session if its provider supports that capability.

RAIN must report the actual outcome. A redirect request does not guarantee that the provider has stopped its current work or changed direction.

### Agent collaboration

Agents should be able to:

- Send a message to another participant.
- Request information or a review.
- Share a result or artifact.
- Delegate a task through a supported workflow.
- Report blockers and completion.
- Ask the user for clarification or approval.

The system should preserve the relationship between messages, tasks, sessions, and resulting artifacts.

---

## 8. Files and Artifacts

RAIN should make file exchange feel like sharing an attachment in a messenger.

An agent might send:

- A PDF report.
- A spreadsheet.
- A generated image.
- A source-code patch.
- A test report.
- A collection of research results.

Users should be able to preview supported files from the phone without manually searching through remote folders.

### Artifact records

Each artifact should have relevant metadata:

- Artifact identifier.
- Filename and type.
- Originating session and computer.
- File size and content hash.
- Creation time.
- Associated conversation or task.
- Access and sharing permissions.

Files can remain on their originating machine until the user or an authorized workflow requests a transfer.

RAIN should distinguish between a file reference, a remotely previewable file, and a transferred copy.

### Safe collaboration on code

Multiple agents editing the same working directory can overwrite one another's changes.

RAIN should support safer workflows such as separate Git branches or worktrees, patch exchange, change previews, and explicit application of changes.

A reviewer should be able to inspect proposed changes before they are merged or applied to a shared working tree.

---

## 9. Cross-Platform Architecture

RAIN should support Windows, Linux, and macOS, with mobile clients for remote interaction.

The product should abstract platform differences rather than require users to understand them.

### Proposed architecture

**RAIN Client**

A familiar messaging interface for mobile and desktop, with conversations, contacts, groups, session activity, attachments, and approvals.

**RAIN Host**

A lightweight background component installed on each computer. It manages local AI sessions, permissions, session events, and access to authorized local resources.

**RAIN Protocol**

A shared communication contract for messages, presence, sessions, tasks, artifacts, and approval requests.

**Provider Adapters**

Integrations for Codex, Claude Code, and future AI tools.

**Platform Adapters**

Platform-specific functionality for Windows, Linux, and macOS, hidden behind a consistent interface.

**RAIN Coordination Layer**

Responsible for group membership, message routing, delivery, task relationships, and cross-machine artifact exchange.

### Possible implementation stack

- **Core and host:** Rust.
- **Desktop shell:** Tauri.
- **Mobile client:** A shared UI architecture where practical.
- **Local persistence:** SQLite.
- **Communication:** Authenticated encrypted connections and a versioned protocol.
- **Networking:** Begin with an existing secure private-network solution; investigate custom connection brokering and relaying only when necessary.

These are implementation proposals, not immutable requirements.

The architecture should remain modular, testable, and replaceable as the project evolves.

---

## 10. Self-Hosted and User-Controlled

RAIN should not require a mandatory cloud backend or a paid AI API merely to coordinate existing local sessions.

Users should be able to connect their own computers and use AI sessions they already have access to, subject to each provider's authentication, subscription, and usage restrictions.

The initial design should favor direct connections over a centralized service whenever practical.

Remote access must work safely across networks. It should not require users to expose unrestricted services to the public internet or manually configure insecure port forwarding.

Future versions may introduce optional relays or coordination services, but self-hosted operation should remain a core design goal.

---

## 11. Security and Permissions

RAIN can potentially access powerful tools through the agents running on connected computers. Security must therefore be fundamental to the architecture.

Key requirements:

- Explicit device pairing and enrollment.
- Cryptographically authenticated device identities.
- Encrypted communication.
- Per-device and per-session permissions.
- Revocation of lost or compromised devices.
- Least-privilege access to files and local capabilities.
- Clear approval flows for sensitive actions.
- Auditable task and action history.
- Secure handling of provider credentials.
- No unrestricted remote shell exposed by default.

RAIN should use established cryptographic libraries and secure networking foundations rather than inventing its own cryptographic primitives.

The local host must enforce permissions. The mobile interface should never be treated as the sole security boundary.

Users should be able to distinguish between inspecting a file, transferring a file, requesting an agent action, and authorizing a consequential operation.

---

## 12. Product Principles

RAIN should follow these principles throughout development.

### 1. Messenger first

Conversations are the primary interface. Infrastructure details remain in the background.

### 2. Hide complexity, not capability

Advanced features may exist, but ordinary users should not need to understand them.

### 3. Sessions are independent participants

Multiple sessions can run on the same computer or across different computers.

### 4. Respect native orchestration

Providers remain responsible for their own subagents and internal execution unless a supported integration explicitly enables deeper control.

### 5. Accurate presence

Show real activity and connection states. Never fabricate progress or completion.

### 6. User-controlled permissions

The user decides which sessions, computers, files, and actions are accessible.

### 7. Reliable communication

Messages, acknowledgements, events, and artifacts must survive ordinary disconnections and reconnects without silently disappearing or being duplicated in ways that trigger unintended work.

### 8. No mandatory AI API

RAIN coordinates existing authorized sessions. Its basic messaging and orchestration layers should not depend on purchasing another model API.

### 9. Keep the scope disciplined

Build what makes coordinating AI work easier. Avoid becoming a generic remote desktop, operating-system manager, or all-purpose automation platform.

---

## 13. MVP Roadmap

The first version should validate the central experience rather than attempt every possible capability.

### Phase 1 — A working messenger

- Establish secure pairing between a phone and one computer.
- Connect to a supported Codex session.
- Display session presence and meaningful activity events.
- Send messages and receive responses.
- Persist conversations and recover from reconnects.

### Phase 2 — Multiple sessions

- Run or attach to multiple independent Codex sessions.
- Give each session a distinct identity.
- Route messages to individual sessions or selected participants.
- Show independent status and message delivery.
- Keep session and working-directory boundaries clear.

### Phase 3 — Group conversations

- Create and delete temporary groups.
- Add sessions from multiple computers.
- Support group messages and direct messages.
- Establish explicit task assignment and handoff workflows.
- Preserve session lifecycles independently of group membership.

### Phase 4 — Artifacts and review

- Share reports and other supported files.
- Preview file content on the phone.
- Inspect code changes and patches.
- Track artifact origins and permissions.
- Add safer collaboration workflows for concurrent code changes.

### Phase 5 — Native subagents and additional providers

- Improve subagent activity visibility where supported.
- Explore optional subagent-level participation.
- Integrate Claude Code and other providers.
- Expose provider-specific capabilities without compromising the common user experience.

### Phase 6 — Broader accessibility

- Simplify installation and updates.
- Improve notifications and approval workflows.
- Add clear recovery from offline machines and interrupted sessions.
- Test onboarding with non-developer users.
- Consider optional relaying and advanced networking only as justified by real usage.

---

## 14. Key Risks

### Integration differences

Providers expose different APIs, events, and control mechanisms. RAIN must adapt to these differences and avoid promising unsupported capabilities.

### Unreliable activity reporting

A silent session may be waiting, blocked, disconnected, or still working. Status must be grounded in available evidence.

### Conflicting changes

Parallel agents can modify the same files. Separate workspaces, patches, and review mechanisms are important.

### Security exposure

A compromised phone or host could expose sensitive files or authorize powerful actions. Permissions, revocation, authentication, and auditing are essential.

### Excessive complexity

Supporting every provider, platform, and orchestration feature from the beginning would delay the product's core value.

### Unproven differentiation

The key assumption to validate is that a familiar conversational interface materially improves the coordination of AI work across machines.

---

## 15. How RAIN Should Be Evaluated

The most important question is:

**Does RAIN make coordinating AI work across sessions and computers meaningfully easier than switching between terminals and provider interfaces?**

Useful early success measures include:

- Time required to pair a new computer.
- Time required to connect and message an agent.
- Reliability of message delivery and session recovery.
- Accuracy of agent presence and activity.
- Ease of coordinating multiple sessions in one group.
- Time required to inspect and retrieve an agent's results.
- Whether a non-developer can complete the basic setup without technical assistance.

If RAIN succeeds, the user should spend less time managing AI tools and more time deciding what work needs to be done.

---

## 16. The Long-Term Vision

RAIN begins as a messenger for AI sessions running on the user's own computers.

Over time, it can become a unified conversational workspace in which independent AI sessions collaborate across machines, manage their own subagents, exchange artifacts, and coordinate complex projects under the user's direction.

The user remains in control. The agents perform the work. The machines provide the execution environments. RAIN connects everything through conversations.

**RAIN — Your AI work, connected.**