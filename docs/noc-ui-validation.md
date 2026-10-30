# NOC frontend validation

2026-10-30, built client served by a disposable real hub at loopback port 17903.

- `npm run build` includes strict TypeScript checking and Vite production build.
- `npm audit --omit=optional`: zero reported advisories at verification time.
- `verify-admin-ui.mjs`: wrong key, authenticated-only controls, saved infrastructure addresses without keys, real agent/group creation, keyboard canvas selection/pan/zoom/reset, motion pause, access records and sign-out. Widths 1440, 768, 390 and 320.
- `verify-cloud-ui.mjs`: live upload, Python worker execution with input, verified downloaded output, remote shared-directory listing, pause/resume, queue cancellation, invitations and draft preservation. Same widths.
- `verify-noc-ui.mjs`: primary routes, real enrollment, address QR, Escape/focus return; fixture human/agent/sudo/code/artifact/working-state messages; 1920, 768 and 390 chat rendering, 896px message-stream bound, 44px visible buttons, reduced-motion suppression. Quiet polling CLS measured 0 across 11 seconds; zero page errors.

The chat fixture intercepts conversation/session display requests. It is not evidence that an external language model performed those actions. Cloud round-trip checks use the real worker and actual task/result endpoints.

Measured exact bubble contrast:

| Theme | Sender | Ratio | AAA normal text |
| --- | --- | ---: | --- |
| Dark | Human | 7.98:1 | Pass |
| Dark | Agent | 14.59:1 | Pass |
| Light | Human | 3.77:1 | Fail, exact palette retained by user request |
| Light | Agent | 17.85:1 | Pass |

Status accents are not universally AAA text. This is not a comprehensive accessibility certification or a universal zero-CLS guarantee. No physical phone, Internet routing, NAT traversal, firewall or CGNAT validation was performed by these browser checks. See `private-hub-validation.md` for actual backend security/isolation checks and `personal-cloud.md` for the private transport setup.

Example invocation after starting a disposable hub and local worker:

```sh
RAIN_URL=http://127.0.0.1:17903 \
RAIN_ADMIN_KEY_FILE=/path/to/disposable-hub/admin.key \
RAIN_PLAYWRIGHT_MODULE=/path/to/playwright/index.mjs \
RAIN_BROWSER=/path/to/chromium \
node scripts/verify-noc-ui.mjs
```

Screenshots are written to `/tmp/rain-noc` by default. The enrollment screenshot contains a short-lived, one-use token: keep test outputs private. The tests create invitations, task/file records and disposable agents/groups; do not run mutation checks against an infrastructure containing important work.
