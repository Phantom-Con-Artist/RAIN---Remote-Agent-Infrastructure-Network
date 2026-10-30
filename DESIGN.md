# PROJECT RAIN operations console

The user's NOC specification supersedes the former Studio Kalex direction. The interface is a dark-first, light-switchable sysadmin workbench: obsidian grounds, glass surfaces, thin chisel borders, emerald human messages, slate agent responses, cyan command conduits and amber approval cards. The user explicitly chose the exact palette over changing colors for AAA contrast.

## Architecture

`apps/rain-client/src/theme.css` is the single semantic theme layer, imported once by the client. Tailwind v4's Vite plugin scans TSX. All fifteen legacy stylesheets were removed; component layouts and states use Tailwind utilities. Only xterm's vendor stylesheet remains separate. Compatibility variables in the theme support data-driven SVG and terminal APIs.

The 64px header reports the configured gateway, measured RTT and HTTPS status. RAIN does not infer WireGuard state or end-to-end encryption from a browser connection. The hub is the server; a phone remains a remote client. The enrollment dialog displays an address-only QR and issues real, single-use worker invitations. It does not simulate email or SMS delivery.

Home uses real workers, tasks and sessions. Network combines a keyboard/pointer-operated topology, task-driven SVG conduits, selected-node telemetry, program allowlists, permissions and durable records. Zoom, pan, reset, motion pause and reduced-motion preferences are supported. An unavailable key fingerprint is identified as not reported; node IDs are not misrepresented as cryptographic fingerprints.

Conversations have a centered 896px stream, colorful asymmetric bubbles, provider accents, reported identity/model/machine metadata, actual delivery states, safe React code highlighting, expandable artifact previews and approval controls. The mobile roster and conversation are mutually exclusive. Drafts and keyed context/activity components survive polling. The composer expands to a bounded height, with explicit recipient, artifact and dispatch controls.

Modals have glass frames, blur scrims, contained scrolling, focus containment, Escape dismissal and focus return. Desktop inspector surfaces use elevated frames without introducing panel shadows across ordinary resting content. Controls use 44px minimum targets, and reduced motion disables packet and typing animation.

## Tokens and assets

Dark canvas: #080a0f / #0f141c / #17202e / #1e293b. Light canvas: #f8fafc / #ffffff / #f1f5f9 / #e2e8f0. Online #10b981; working #06b6d4; warning #f59e0b; danger #ef4444; offline #64748b. Human bubbles: #005c4b dark, #059669 light, with white text. Agent bubbles: #182232 with #f1f5f9 dark; white with #0f172a light. Borders retain the user's specified RGBA values.

Poppins Regular/Medium and variable JetBrains Mono are self-hosted with OFL files in `src/assets/fonts`. JetBrains Mono came from the Google Fonts repository (`ofl/jetbrainsmono/JetBrainsMono[wght].ttf`). Poppins and the supplied burgundy RAIN logo were existing project assets. No new raster illustration or external font request is introduced. Typography uses optical feature settings and tabular monospace for changing figures. Metadata and operations use compact technical labels; body text uses normal prose.

Design-taste app draw: Data terminal, seed 877000488, retaining the user's palette and typography. User-specified glass, navigation, motion and colorful messaging override that draw's conflicting defaults. Browser screenshots and behavior, rather than the draw, determine the final implementation.

## Verification limits

Run `npm run build` in `apps/rain-client` for both strict TypeScript checking and production compilation. `scripts/verify-admin-ui.mjs` and `scripts/verify-cloud-ui.mjs` exercise real authentication, permissions, task/file round trips and canvas controls against a disposable hub. `scripts/verify-noc-ui.mjs` combines real enrollment with clearly isolated chat display fixtures, captures both themes and mobile/ultrawide layouts, measures quiet polling shifts and records actual bubble contrast.

Exact light emerald with white text is approximately 3.77:1 and fails AA normal text and AAA. Status colors can also fail AAA when used as text, particularly on light surfaces. These are explicit palette exceptions, not an accessibility certification. Observed zero quiet-polling CLS does not prove zero shifts for every interaction, network condition or content length. Browser viewport checks do not replace physical phone, WAN, router, CGNAT or assistive-technology testing.
