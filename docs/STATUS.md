# RAIN implementation handoff

Work stopped at the owner’s request to conserve remaining quota. This record distinguishes verified behavior from unfinished work.

## Verified during this session

- Rust workspace compilation, frontend production builds, and frontend TypeScript checks passed at their recorded revisions.
- The focused regression suites passed for chat delivery, adapter routing, PTYs, persistence, discovery, filesystem access, and binary transfer integrity.
- Actual Agy and Muse chat adapters were launched through the browser, added to a group, and each returned the requested response. A subsequent `@agy` instruction returned only Agy’s reply. Group replies did not leak into direct chats.
- Deleting the test group left its agents running. Stopping/removing test sessions retained conversation history.
- Native terminal output rendered through xterm.js, with prompt delivery, incremental output without duplicate replay, and terminal resizing exercised. ANSI redraws no longer become chat bubbles.
- Agy created `/tmp/rain-agent-share-e2e/agy-produced.txt`. The browser attached it from the host, Muse read its staged copy, and returned the expected token. The exported SHA-256 was `151f1c7d7ddf3874ee5a477b0d07f072f578af6684edafb4e27cb2768dcd4799`. A corrupted-hash transfer was rejected.
- Neutral light/dark layouts were inspected at desktop, short laptop, tablet, phone, and 320px widths in the earlier design pass. The editor’s visible keyboard focus and dark placeholder contrast were corrected.

## What changed after the initial UI review

The interface now prioritizes messenger conversations with agent contacts, search, direct/group threads, message bubbles, delivery labels, and a multiline composer. Native terminals are secondary. Structured chat adapters were added for Agy, Muse, Codex, and Claude Code. File attachments now transfer real bytes and verified recipient paths. Group creation failures retain their draft, dialog keyboard focus is managed, and file editor state survives tab changes with unsaved-change protection.

Agy and Muse preserve their provider session identifiers across turns. Chat-adapter replies carry the originating RAIN conversation identifier. Recent group context is included on assigned turns. Unsupported CLI integrations keep terminal mode rather than inventing assistant responses.

## Remaining verification/work

- Finish the full interactive UI audit. File-draft recovery across tabs, host-change races, modal failure states, pairing, and every narrow-screen state have not all received a final end-to-end pass after the latest changes.
- Exercise actual remote SSH machines and multi-host attachment transfer. The transfer mechanism uses the SSH host-operation layer, but the real Agy/Muse sharing test used the local host.
- Exercise live Codex and Claude model conversations and provider-specific slash commands/approval behavior. Do not assume every TUI slash command is supported by a headless adapter.
- Pairing is legacy development scaffolding: the existing QR generator is not a standards-compliant QR encoder, email OTP is not backed by an email delivery service, and enrollment data is in memory. The current API is not a complete per-device/per-session authorization system. Keep remote access behind SSH.
- Spoke mode publishes telemetry; execution uses SSH, not spoke job dispatch. Network status contains legacy placeholder values. Universal autonomous delegation, reliable event replay, process reattachment after daemon restart, and battery-driven session migration are not implemented.
- Legacy job approval/artifact-transfer endpoints still contain prototype behavior. The legacy power/failover endpoint can stage, commit, and push the daemon’s repository; it is not verified autonomous task migration.
- Attached file contents remain in saved message history; per-recipient copies are staged in private temporary inbox directories. No automatic inbox retention cleanup is implemented.
- Generic process output still uses a bounded broadcast channel; overload can drop events and is logged. Structured adapters currently return completed-turn text rather than token streaming.

## Runtime and restart notes

A live issue was traced to the old daemon still serving port 7890 while newer frontend assets were on disk. The daemon was refreshed only after checking that its sessions were stopped, with state backups written under `/tmp/rain-before-refresh-*.json`.

The live daemon was last refreshed with the messenger/chat-adapter build before the final attachment implementation. The committed source and built assets are newer. Rebuild/restart the host before relying on the latest attachment endpoint; refresh the browser afterwards. A restart closes supervised processes, so stop or finish active work first.

Isolated browser tests used ports 17890–17892 and temporary history files under `/tmp`. The Agy/Muse file-sharing test completed successfully before the stop instruction. Its agent workers were stopped/removed; no further model tests are being run.

## Sources used for adapters

- Installed `agy --help` and actual JSON print output.
- Installed `muse exec --help`, JSONL terminal events, and two-turn session-id echo checks.
- [Official Codex non-interactive documentation](https://learn.chatgpt.com/docs/non-interactive-mode).
- [Official Claude Code CLI reference](https://code.claude.com/docs/en/cli-usage).
- [xterm.js terminal API](https://xtermjs.org/docs/api/terminal/classes/terminal/).
