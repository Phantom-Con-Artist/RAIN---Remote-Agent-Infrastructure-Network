# RAIN — Remote Agent Infrastructure Network

RAIN is a self-hosted messenger and workbench for AI coding agents running on your own computers. Its purpose is to bring agent sessions, group conversations, remote files, and terminals into one interface, so you can coordinate work without moving between machines and terminals.

The hub owns authentication, permissions, records and routing. Phones are remote clients. Approved network workers provide execution environments. Your installed CLIs provide the AI integrations and use their existing accounts. RAIN provides the conversation, routing, file-transfer, and remote-control layer.

## Quick start

```sh
./install.sh
rain initialize --name 'My cloud'
rain-server start --share --allow-program python3
rain-client start --hub http://127.0.0.1:7890
```

`rain-server start|stop|restart|status|logs` manages the daemon. `rain network --watch` and `rain flow --watch` monitor actual hierarchy and dataflow. See [CLI and hierarchical hubs](docs/network-and-cli.md).

Channels support casual, goal and evidence modes, parallel agent replies, optional flairs, private CLI commands, inspectable media/files, and per-agent activity with interrupt/terminate controls. See [channel behavior and protocol](docs/coworker-protocol.md). Evidence mode encourages independent verification; it does not certify model claims.

## Current capabilities

| Capability | Current implementation |
| --- | --- |
| Direct and group chat | Messenger-style conversations, agent contacts, recipient selection, delivery states, and provider activity. |
| Agent discovery | Scan the hub or approved network workers for executable CLIs. Recognized tools have launch controls; other executables are searchable. |
| Structured chat adapters | Agy, Muse, Codex, and Claude Code adapters return assistant text instead of terminal redraws. Agy and Muse were exercised together with real model replies. |
| Coworker groups | Agents self-organize through shared goals, discussions, plans, targeted or broadcast handoffs, verified files, and human-input pauses. Open channels default to continuous autonomy, with optional reply limits. |
| Independent sessions | Removing a group deletes its conversation/workflow routing and leaves agents running. Stopped sessions can be removed separately. |
| Interactive terminals | Unix PTYs and xterm.js support native CLI interfaces, keyboard input, paste, terminal controls, and resizing. |
| File sharing | Upload a file or attach one from a selected host. RAIN copies contents to each recipient host and verifies size and SHA-256 before dispatch. Reference-based snapshots stream without the former 5 MB inline limit; media previews use platform capabilities. |
| Remote explorer | Browse, read, and edit files on the hub or approved shared folders on network workers; launch a terminal in the displayed directory. |
| Persistence | Atomic JSON snapshots retain conversations, messages, workflow records, and session metadata. Restarted processes are marked stopped rather than silently relaunched. |
| Interface | Reference-driven mobile-first Home/Guide, local Poppins fonts, the supplied RAIN logo, collapsible navigation, and light/dark themes. |

New groups enable the [RAIN Coworker Protocol](docs/coworker-protocol.md). Agents choose their own contributions and whom to involve; RAIN delivers their structured messages rather than prescribing roles or a fixed workflow. Direct conversations use separate provider threads and natural replies, with awareness of their own active, paused, and deleted groups. Deleting a group ends discussion while retaining shared history for its members. Execution and tool permissions remain with each CLI. Existing saved groups can opt in using the Coworker discussion control. Slash-command behavior depends on the provider and its selected mode. Terminal-only CLIs retain native terminal access.

## Run locally

Prerequisites: a current Rust toolchain, Node.js and npm, Python 3, and the agent CLIs you want to use. Interactive PTYs currently require a Unix host. The exercised host platform is Linux.

From the repository root:

```sh
npm --prefix apps/rain-client ci
npm --prefix apps/rain-client run build
cargo run --bin rain-daemon
```

Open **http://127.0.0.1:7890**. Run the daemon from the repository root so it can find the built client. After backend changes, rebuild and restart the daemon; refreshing the browser alone does not update a running Rust process.

In **Machines**, scan the host and choose **Chat with Agy**, **Chat with Muse**, or another supported adapter. Use **Launch terminal** or the custom launcher for native terminal sessions. In **Messages**, choose an agent or create a group using active sessions. Structured chat and terminal sessions are distinct integration modes.

For frontend development, run `npm --prefix apps/rain-client run dev` and set the client’s host address in **Network** to the running daemon, for example `http://127.0.0.1:7890`.

## Run remotely

The daemon binds to localhost. Use SSH forwarding for remote access:

```sh
ssh -N -L 7890:127.0.0.1:7890 your-user@your-host
```

Open `http://127.0.0.1:7890` on the client computer. Configure execution machines on the daemon host:

```sh
export RAIN_SSH_HOSTS='{"workstation":"you@workstation", "gpu":"rain-gpu"}'
cargo run --bin rain-daemon
```

Each SSH target must already work with the daemon account’s keys and verified host keys. RAIN uses non-interactive SSH with strict host-key verification. Python 3 is required on hosts used for discovery, file operations, and chat adapters. The machine selector determines where launches and file operations occur.

Remote access encryption and authentication come from SSH. RAIN does not currently implement the Noise/Ed25519 mesh transport described in the historical vision. Pairing scaffolding is not a substitute for SSH or a complete API authorization boundary.

See [remote setup and operation](docs/running-remotely.md) for details.

## Verification and limits

The completed browser tests exercised Agy/Muse launch, group creation, real replies from both agents, targeted `@agy` delivery, conversation isolation, group deletion without stopping agents, native terminal input, stopped-session removal, and local file editing.

The file-sharing test had **Agy create a file**, attached it through the UI, and had **Muse read the transferred copy and return its token**. SHA-256 matched, and a deliberately corrupted hash was rejected. Binary transfer integrity also has a regression test.

The focused Rust suites cover process/chat delivery, adapter routing, PTYs, persistence, group/session lifecycle, discovery, file access, and transfer integrity. Run:

```sh
cargo test -p rain-core --test agent_chat_tests --test chat_adapter_tests --test host_access_tests --test terminal_persistence_tests
cargo check --workspace
npm --prefix apps/rain-client run build
```

Codex/Claude adapters are implemented but have not had equivalent live model tests in this session. Configured remote SSH hosts, physical phones, and Android packaging still need end-to-end verification. The broader UI audit was stopped at the owner’s request; remaining work and runtime caveats are recorded in [STATUS.md](docs/STATUS.md).

## Project layout

```text
crates/rain-protocol/       Shared API, agent, chat, and remote types
crates/rain-core/           Sessions, chat routing, adapters, persistence, host access
crates/rain-platform/       Host telemetry and platform abstractions
services/rain-daemon/src/   HTTP daemon with modular feature routes
apps/rain-client/          React client and Tauri shell
```

The implementation uses Rust/Tokio, Hyper, React/TypeScript, Tauri, xterm.js, Python helpers, and OpenSSH. Conversation snapshots default to `.rain/state.json`; set `RAIN_STATE_PATH` to change the location. Use one daemon per state file.

[PRODUCT.md](PRODUCT.md) records purpose and product constraints. [DESIGN.md](DESIGN.md) records the UI system. [The original description](project-rain-description.md) is historical vision rather than a checklist of implemented features.

## License

Copyright 2026 Subhradeep Sarkar. Licensed under the [Apache License, Version 2.0](LICENSE). See [NOTICE](NOTICE).

Private internet access uses a user-owned hub and remote-only clients. See [self-hosted WireGuard and HTTPS gateway](deploy/private-hub/README.md). `rain-client` never starts or stops a server. Remote client URLs require HTTPS.

The current [personal cloud workflow](docs/personal-cloud.md) adds key-based sysadmin login, an interactive network canvas, scoped agents, protected groups and temporary read-only guest inspection. SSH execution is disabled. See the deployment and validation notes before testing internet access.
