# Personal-cloud validation — 2026-10-30

The network/authentication implementation was tested on this machine with
loopback hubs, separate worker processes, disposable state, and a real Caddy
HTTPS gateway. This is a focused implementation/security review, not a claim
that no vulnerabilities exist or that public internet deployment is validated.

## Completed checks

- Rust daemon tests cover gateway/origin rejection, admin key login/logout and
  rotation, audit records, one-use enrollment, scoped leases and restart safety.
  Core file/explorer tests cover immutable binary snapshots, corruption checks,
  special paths and rejection of the retired SSH transport.
- Real hub plus two active outbound workers: task placement, resources, a client
  upload → worker execution → verified result → download, folder listing,
  retrieval and sending, cancellation, timeout, pause/resume, restart recovery,
  revocation, cross-worker isolation and task-specific input authorization.
- Lost enrollment response recovery with a pre-saved credential; saved identity
  reuse; duplicate-process locking; worker credential hashes on the hub.
- Actual managed-agent execution through bubblewrap cannot see the hub admin
  key or worker identity. Guest inspection cannot traverse out of its folder,
  follow external symlinks, overwrite files, or use command/write actions.
- Group membership gates messages, read-only snapshots and folder inspection.
  Outsiders cannot see group history or shared files. Either the sysadmin or the
  requested machine's owner can approve a 15-minute read-only guest grant.
  Another machine's owner cannot approve it. Revocation and membership removal
  remove result/file access. Agent credentials cannot reach the hub filesystem.
- Real HTTPS, validated against a disposable CA: anonymous admin API/wrong key
  rejection; HttpOnly/Secure/SameSite session; forged gateway header overwrite;
  origin checks; worker execution and file return without an admin credential;
  browser admin login; key rotation invalidating prior keys and sessions.
  Forced gateway failure confirms custom credential headers and keys do not
  appear in the error log. Browser TLS tests use a test-only certificate
  exception; Python independently validates the temporary CA.
- Explicit infrastructure initialization, private key permissions, stable key
  and identity across initialization, saved named profiles, refusal to silently
  replace missing keys, local sharing lifecycle and absence of admin keys from
  redirected startup output/server logs.
- Browser: login denial/gating, credential/group controls, audit records,
  keyboard canvas selection, zoom/pan/reset, motion controls, reduced motion,
  saved infrastructure addresses without keys, sign-out, client file upload,
  real worker execution/download, folder results, pause/cancel and drafts.
  Screenshots cover 1440, 768, 390 and 320 px and dark mode. JavaScript page-error
  checks pass. The design source detector reports no findings for this stage.
- Client production build and TypeScript check pass. Updating Vite resolved the
  reported dependency advisories; `npm audit` currently reports zero findings.

## Fixes from the review

Privileged APIs now require admin authentication even on loopback. Credentials
are split by role. Admin secrets are not saved to browser storage or sent in
URLs. Gateway error logging redacts custom auth headers. Responses prevent
caching of authenticated JSON, framing of the admin document and active content
in uploaded artifacts. Client credential requests reject redirects. Worker
telemetry and results are type/size bounded before reaching the UI.

Infrastructure keys cannot silently regenerate after a saved key is lost. A
profile cannot attach to another hub already occupying its port. Successful
heartbeat/lease keepalives avoid durable snapshot/audit writes; mutations and
failed access attempts are recorded. Dashboard previews and history requests are
bounded, and full task/group records are available on demand.

## Reproduce

```sh
cargo build -p rain-daemon --offline
cargo test -p rain-daemon --offline
cargo test -p rain-core --test host_access_tests --test artifact_network_tests --offline
npm run build --prefix apps/rain-client
apps/rain-client/node_modules/.bin/tsc --noEmit -p apps/rain-client/tsconfig.json
python3 scripts/verify-infrastructure.py
python3 scripts/verify-cloud.py
python3 scripts/verify-private-gateway.py
python3 scripts/verify-private-https.py
```

Browser checks require Playwright (`RAIN_PLAYWRIGHT_MODULE`) and optionally a
Chromium path (`RAIN_BROWSER`). HTTPS checks require Caddy (`RAIN_CADDY_BINARY`).
The review UI is at `http://127.0.0.1:17903/#network/cloud`, using
`RAIN_HOME=/tmp/rain-cloud-admin-ui`; its admin key is in that directory.
`verify-admin-ui.mjs` and `verify-cloud-ui.mjs` accept `RAIN_URL` and
`RAIN_ADMIN_KEY_FILE`. They create test records; use a disposable hub. Screenshots
are saved under `/tmp/rain-admin-*`, `/tmp/rain-cloud-*` and
`/tmp/rain-private-https-mobile.png`.

## Limits and deployment work

No production firewall, router, VPN, DNS, or host trust store was changed.
WireGuard activation, WAN/CGNAT reachability, physical phone certificate trust,
cellular transitions, native mobile builds and macOS/Windows agent isolation
are untested. Managed cloud agents require Linux bubblewrap and fail closed
elsewhere. Existing trusted local hub agent sessions retain their OS-account
permissions; they are not covered by the cloud-agent sandbox.

The network preparer generates configurations/keys/QR locally, but activation,
router reachability and private CA trust remain one-time deployment tasks.
No SMS or email provider was configured and no key/message was sent externally.
This is single-hub operation, not transparent process migration or HA failover.
Two-worker checks are not a large-scale load or exhaustive penetration test.
