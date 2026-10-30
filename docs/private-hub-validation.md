# Private hub validation — 2026-10-30

Tested on this Fedora machine with an isolated loopback hub and disposable state.
The review hub is at http://127.0.0.1:17901/#network/connections. It is local to
this machine and does not establish private internet access.

## Passed

- Rust daemon tests, offline daemon build, client production build and TypeScript
  check. The Impeccable source detector reported no findings for the reviewed UI.
- Live HTTPS using Caddy 2.11.7 and the supplied deployment configuration, bound
  to loopback with a temporary internal CA. Python verified TLS against that CA;
  Chromium used a test-only certificate exception. No host trust store was changed.
  UI and API reject missing/wrong gateway login; correct login works; forged
  gateway secrets are overwritten; unapproved origins are denied. The browser
  loads assets, polls APIs and reconnects with a device token present. Device
  tokens use a separate header so they cannot replace the gateway login.
- Real localhost root → subhub → subsubhub → worker enrollment/heartbeats,
  heartbeat authentication, cyclic hierarchy rejection, JSON CLI output,
  7.5 MB upload/download, byte ranges, invalid ranges, parent restart recovery,
  stable identity and worker revocation. The parent restart test confirms
  subhubs remain alive and reconnect; it does not migrate running processes.
- Chromium connection UI at 1440, 768, 390 and 320 px, light and dark screenshots,
  keyboard form order, unsafe URLs, credentials embedded in URLs, paths/queries,
  unreachable hub, sign-in/access denial, non-RAIN HTML, timeout, corrupt saved
  address, credential clearing on hub switch, and offline/retry recovery.
- Actual review-hub shutdown and restart with the browser left open: offline
  notice appears above the workspace and reconnect succeeds. Injected HTTP and
  network failures are used separately for deterministic error-state coverage.

## Reproduce

Build first with `cargo build -p rain-daemon --offline` and
`npm run build --prefix apps/rain-client`. Then:

```sh
cargo test -p rain-daemon --offline
apps/rain-client/node_modules/.bin/tsc --noEmit -p apps/rain-client/tsconfig.json
python3 scripts/verify-private-gateway.py
python3 scripts/verify-local-network.py
RAIN_HOME=/tmp/rain-private-ui bin/rain-server start --port 17901
node scripts/verify-connections-ui.mjs
python3 scripts/verify-private-https.py
```

Browser checks require Playwright. Set `RAIN_PLAYWRIGHT_MODULE` to its module
path and `RAIN_BROWSER` to an installed Chromium executable if necessary.
Set `RAIN_CADDY_BINARY` to an installed Caddy executable for the HTTPS check.
The connection UI check stops/restarts only the review hub owned by
`RAIN_HOME=/tmp/rain-private-ui`; do not use that runtime for real work.
Other integration tests use disposable state and stop their own processes.
Screenshots are written under `/tmp/rain-connections-*`,
`/tmp/rain-hub-offline-mobile.png` and `/tmp/rain-private-https-mobile.png`.

## Remaining deployment checks

WireGuard is not installed here. VPN routing, WAN firewall behavior, router
port forwarding, ISP/CGNAT reachability, certificate installation on real phones,
physical mobile Wi-Fi/cellular transitions and native mobile builds were not
tested. There is no automatic router setup or reachability wizard yet. Local
HTTPS tests do not establish internet isolation or guarantee absence of security
vulnerabilities. No system firewall, router, VPN, DNS or trust store was changed.
The architecture remains single-owner; authenticated owners have full hub control.
