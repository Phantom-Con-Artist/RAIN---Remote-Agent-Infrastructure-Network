# Coworker validation — 30 October 2026

Validation used installed Agy and Muse CLIs through RAIN's real chat adapters on Linux, with an isolated daemon at localhost:17901 and separate temporary workspaces. Both used their existing provider accounts. The ordinary RAIN daemon and user workspaces were left running separately. No backup, deployment or remote operation was performed; the test task was to draft and peer-review a small backup-verification checklist.

## Observed direct behavior

- Before a group existed, both agents identified the interaction as direct chat and reported knowing no shared groups.
- With a shared group active, both described its actual goal, checklist artifacts, peer review, and current discussion stage from their direct conversations.
- After pause, Agy identified the group as paused without being told its state in the question.
- After deletion, both retained the group's purpose and planning history and stated that the deleted channel could not receive further discussion. They continued answering naturally in direct chat.

## Native response framing defects found and fixed

Agy created a checklist file and selected Muse for independent review. RAIN copied that file into Muse's local inbox; the source and destination SHA-256 values matched. Muse produced a substantive review file.

The first live run exposed Muse returning two complete protocol envelopes concatenated together. A subsequent run exposed a complete envelope followed by an ordinary closing sentence. The original single-object parser displayed these replies without forwarding the review. RAIN now accepts a bounded contiguous sequence of up to four envelopes, suppresses exact duplicates within a response, and displays trailing prose without treating it as a routing instruction. Leading prose is not searched for embedded commands. Truncated JSON tails invalidate the batch before routing. Regression coverage reproduces these native response shapes.

## Final live rerun

With the framing fixes loaded in a freshly restarted test daemon, Agy created `checklist_draft.md` and addressed Muse for review. Muse returned `review_checklist_v2.md` as a routed `discuss` message with a verified checksum. Its source and recipient inbox copies both hashed to `5852253b62ece7620dbcd99508712b5d6ed0b6ea5f3e40d8d26ee5a495028104`.

Muse challenged the draft's minimum throughput, depth of nightly consistency checks, and failure-path cleanup. Both agents then submitted valid `human_input` messages with a concrete approval question. The group stopped autonomous handoffs. Their subsequent direct replies correctly described the actual draft, review, revised result and waiting state. Agy recognized the later pause. Both retained the result after deletion and identified the deleted channel as closed. The test runner then stopped both test agent sessions.

The roles and substantive review were selected by the agents after receiving a common goal and a request for peer review; RAIN did not prescribe an author/reviewer state machine. These observations demonstrate one useful self-organized exchange, not a guarantee that every model/task will converge usefully.

## Automated and rendered checks

`cargo test -p rain-core -p rain-protocol --tests` passes 19 tests, including real local adapter processes with deterministic provider fixtures. The suite covers membership and run scoping, direct/group provider-thread separation for Codex/Claude/Agy/Muse, binary transfer integrity, path and symlink boundaries, autonomous handoffs, human-input pauses, turn budgets, pause/resume, archive persistence, private-direct exclusion, and native response framing.

The client production build, TypeScript check, daemon build, and whitespace checks pass. HTTP checks verify group context for members and exclusion for nonmembers, policy changes, pause/resume, deletion history, and PATCH preflight support. Browser checks at 320, 390 and 1440 pixels show no horizontal page overflow and keep the composer visible. The discussion toggle, pause/resume, and direct-chat disclosure of deleted group history were exercised. Real group and direct conversations were also inspected in the browser.

This evidence verifies the tested coordination and memory behavior. It does not establish unrestricted emergent intelligence, support for every provider output variation, physical-device behavior, or transfers over configured SSH hosts. Membership belongs to a session UUID; newly launched sessions do not automatically inherit old session memberships.

## Expanded channel, media, topology and interface validation (2026-10-30)

- 26 Rust integration/unit tests passed across rain-core, rain-protocol and rain-daemon. Coverage includes >5 MB immutable files, actual host-probe peer-copy code through an SSH fixture, forced hub fallback, partial resume and corruption recovery; persisted hierarchy/liveness/revocation; channel routing and memory; private native settings; mode persistence; optional flairs; interrupt scope; trace provenance and persistence.
- The installed CLI hierarchy harness (`scripts/verify-local-network.py`) passed with real localhost root → subhub → subsubhub → worker processes, authenticated heartbeats, cycle rejection, parent restart/reconnect, revocation propagation, and a 7.5 MB upload with exact ranged reads and full download. Physical SSH hosts were not available; transfer branches use a deterministic SSH fixture.
- Real Agy and Muse started initial group turns 7 ms apart and overlapped for 35 seconds. Both independently read a two-line fixture and replied in Evidence mode. Actual tool/task events appeared in the activity stream. Both private chats identified their shared group as paused and returned the requested file as an attachment. This is a behavior check, not an accuracy benchmark or proof that hallucinations are eliminated.
- Browser checks passed at 320/390/768/1440 px without page overflow. Self-hosted Poppins loaded. Home, menu Escape/focus, Guide search and topics, slash/mention completion, per-chat drafts, interrupt action, topology focus and dataflow rendering were exercised. Native WAV and WebM previews played and sought through the real ranged artifact endpoint without browser errors.
- A user-reported disappearing-chat bug was reproduced: context and activity siblings shared a React key. They now have distinct stable keys. `scripts/verify-chat-ui.mjs` checks repeated polling leaves one context panel, one activity panel, and a usable message viewport.
- UI source lint and rendered Home inspection pass. The large Home headline is an intentional reference-driven exception to working-app title size; working pages retain compact task titles.

Optional browser regression: install Playwright in your test environment and run `node scripts/verify-chat-ui.mjs` against a running built server. Set `RAIN_URL`, `RAIN_PLAYWRIGHT_MODULE`, or `RAIN_BROWSER` when using an alternate server/module/browser location. The test intercepts its API data and does not mutate the server's conversations.
