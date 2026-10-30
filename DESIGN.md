# RAIN interface

The current direction follows the user's Studio Kalex reference, inspected at desktop and mobile widths on 2026-10-30: Poppins, white ground and black ink, tightly spaced display headings, generous margins, flat surfaces, a simple typeset wordmark, and large text navigation when the menu opens. The burgundy supplied RAIN node mark remains the product logo. Do not copy the reference's studio identity, imagery or marketing claims.

## Frame and pages

Mobile is the base layout: 20px side gutters, 76px header, one task pane at a time, no persistent bottom bar. Desktop uses 40px gutters and an 88px header with text navigation. The menu is a full, opaque dialog with keyboard focus containment, Escape dismissal and focus return. Desktop menu destinations have descriptive sublines; mobile keeps large labels. Light is the initial theme; saved light/dark preferences persist.

Home is an entry and manual page, with a two-line display heading, direct actions, actual running agents and recent conversations, setup commands and Guide links. Its largest heading is intentionally 84px at desktop width, reducing to 32–42px on phones. The large heading is an explicit reference choice for the entry page; working app pages keep 26–36px titles. Guide has its own route and topic pages, search, copyable commands and collapsible mobile contents.

Conversations use a roster and message pane on desktop, and a pushed detail pane with Back on mobile. Group context and activity use separate stable React keys: polling must never duplicate either. Their disclosures collapse by default. Channel settings and attachments are also separately disclosed. The message feed takes the remaining height; the composer remains visible. Agent posts have quiet text treatment, and human posts use one light surface. Destructive actions sit under Options; pause stays directly available.

Machines, Files and Network preserve selected host context. Files retains a text editor and any-file inspector. Network separates Topology, Dataflow and Connections, with a keyboard-accessible graph and bounded scrolling tables.

## Type and color

Poppins Regular and Medium are self-hosted in src/assets/fonts with the Google Fonts OFL license. Poppins is the task, navigation and display face. The existing monospace stack is reserved for commands, paths, IDs, terminals and terminal ASCII identity. Do not assert a specific monospace font is installed.

Light: ground/surface #ffffff, secondary surface #f6f6f6, ink #101010, secondary ink #595959, muted ink #676767, quiet edges #ededed and control edges #dddddd. Dark: ground/surface #111111, secondary surface #1c1c1c, ink #f5f5f5, secondary ink #b9b9b9, edges #282828/#383838. Semantic errors and live status retain existing accessible theme tokens. Brand burgundy is confined to the logo and root-node identity. No gradients or blue-tinted dark surfaces.

Display headings use weight 500, tracking no tighter than -0.04em, and roughly 1.1 leading. Working body text is 14–15px at 1.6–1.85 leading. Metadata uses 12px, with 11px reserved for compact timestamps. IDs and changing figures use tabular numbers.

Primary entry actions use black/white inverted fills, 46px minimum height and the full rounded shape from the reference. Utility controls use a 6px radius and 40px minimum height, with larger touch areas where needed. Panels use tonal separation or a quiet edge; no resting panel shadows. Shadows belong to floating menus/dialogs. Icons use the existing Feather set at a consistent light stroke. No icon tiles or decorative illustrations.

## Verification and limits

Design source lint passes. Rendered Home inspection passes at 1440, 768 and 390 px, allowing only its intentionally large entry-page heading. Browser review also covers 320 px, both theme behavior, font loading, menu dismissal, Guide search/topics, chat suggestions, drafts, activity controls, topology drill-down and audio/video seeking. A polling regression checks one context panel and one activity panel after repeated refreshes. These checks are not physical-device, physical SSH network, or comprehensive accessibility certification.

The previous Material-style workbench documentation and sidecar specimens describe the earlier design. The current implementation is the visual authority. The design-taste app direction was editorial-greeting (seed 1051706069); the pinned reference superseded its suggested permanent sidebar shell.
