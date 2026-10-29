---
name: RAIN
description: Neutral local and SSH agent workbench
colors:
  dark-primary-ink: "#1b1b1b"
  dark-accent-hover: "#ffffff"
  dark-bg-canvas: "#141414"
  dark-bg-surface: "#1b1b1b"
  dark-bg-subtle: "#242424"
  dark-bg-elevated: "#303030"
  dark-bg-hover: "#353535"
  dark-bg-active: "#3b3b3b"
  dark-border-subtle: "#303030"
  dark-border-default: "#3c3c3c"
  dark-border-strong: "#666666"
  dark-text-primary: "#ededed"
  dark-text-secondary: "#bdbdbd"
  dark-text-muted: "#a6a6a6"
  dark-accent-primary: "#e2e2e2"
  dark-accent-subtle: "#303030"
  dark-accent-text: "#ededed"
  dark-danger-surface: "#450a0a"
  dark-danger-border: "#991b1b"
  dark-danger-text: "#fca5a5"
  dark-danger-btn: "#dc2626"
  dark-warning-surface: "#451a03"
  dark-warning-border: "#92400e"
  dark-warning-text: "#fcd34d"
  dark-success-surface: "#252b25"
  dark-success-border: "#526852"
  dark-success-text: "#bfd2bf"
  dark-bubble-human-bg: "#303030"
  dark-bubble-human-border: "#484848"
  light-primary-ink: "#ffffff"
  light-bg-canvas: "#f5f5f5"
  light-bg-surface: "#ffffff"
  light-bg-subtle: "#eeeeee"
  light-bg-elevated: "#ffffff"
  light-bg-hover: "#dfdfdf"
  light-bg-active: "#d0d0d0"
  light-border-subtle: "#dfdfdf"
  light-border-default: "#d0d0d0"
  light-border-strong: "#767676"
  light-text-primary: "#202020"
  light-text-secondary: "#535353"
  light-text-muted: "#666666"
  light-accent-primary: "#262626"
  light-accent-hover: "#404040"
  light-accent-subtle: "#eeeeee"
  light-accent-text: "#252525"
  light-danger-surface: "#fef2f2"
  light-danger-border: "#fecaca"
  light-danger-text: "#991b1b"
  light-danger-btn: "#dc2626"
  light-warning-surface: "#fffbeb"
  light-warning-border: "#fde68a"
  light-warning-text: "#92400e"
  light-success-surface: "#edf2ed"
  light-success-border: "#bfcebf"
  light-success-text: "#526852"
  light-bubble-human-bg: "#e8e8e8"
  light-bubble-human-border: "#d0d0d0"
typography:
  headline:
    fontFamily: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "24px"
    fontWeight: 500
    lineHeight: 1.3
    letterSpacing: "-0.02em"
  title:
    fontFamily: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "18px"
    fontWeight: 500
  body:
    fontFamily: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.5
  label:
    fontFamily: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "13px"
  metadata:
    fontFamily: "-apple-system, BlinkMacSystemFont, \"Segoe UI\", Roboto, \"Helvetica Neue\", Arial, sans-serif"
    fontSize: "12px"
  path:
    fontFamily: "'JetBrains Mono', 'SF Mono', Consolas, Menlo, Monaco, monospace"
    fontSize: "12px"
    lineHeight: 1.5
  ascii-wordmark:
    fontFamily: "'JetBrains Mono', 'SF Mono', Consolas, Menlo, Monaco, monospace"
    fontSize: "7px"
    fontWeight: 600
    lineHeight: 1.08
rounded:
  sm: "4px"
  md: "6px"
  lg: "8px"
spacing:
  space-1: "4px"
  space-2: "8px"
  space-3: "12px"
  space-4: "16px"
  space-6: "24px"
  space-8: "32px"
components:
  button-primary-dark:
    backgroundColor: "{colors.dark-accent-primary}"
    textColor: "{colors.dark-primary-ink}"
    rounded: "{rounded.sm}"
    padding: "9px 14px"
  button-primary-light:
    backgroundColor: "{colors.light-accent-primary}"
    textColor: "{colors.light-primary-ink}"
    rounded: "{rounded.sm}"
    padding: "9px 14px"
  button-outline:
    backgroundColor: "transparent"
    rounded: "{rounded.sm}"
    padding: "9px 14px"
  input-message:
    rounded: "{rounded.sm}"
    padding: "10px 12px"
  nav-desktop:
    rounded: "{rounded.sm}"
    height: "48px"
    padding: "0 16px"
  machine-row:
    backgroundColor: "transparent"
    rounded: "0"
    padding: "24px 0"
  status-label:
    rounded: "0"
    padding: "0"
  modal:
    rounded: "{rounded.lg}"
    padding: "24px"
---

# Design System: RAIN

## Overview

**Creative North Star: "RAIN workbench"**

RAIN is a neutral Material-like developer workbench for launching and coordinating CLI agents. Paper and graphite surfaces, familiar system typography, and restrained rectangular controls keep machine context and task actions legible. The existing React/Tauri implementation uses plain CSS; this document captures that system rather than prescribing a component-framework migration.

Desktop is the primary workspace; smaller screens remain remote controls. The ASCII RAIN wordmark and matching monogram belong in application chrome. Ordinary task labels stay readable sans serif. The North Star names the existing workbench direction; it is not a separately approved brand concept.

**Key Characteristics:**

- Neutral light and dark themes.
- Visible machine scope for execution and filesystem tasks.
- Flat task regions and compact rectangular controls.
- ASCII identity; system sans for tasks and mono for technical content.

## Colors

The normative frontmatter records the implemented CSS values, with `dark-` and `light-` prefixes identifying the two mappings of the same semantic role. Runtime tokens remain in `apps/rain-client/src/styles/tokens.css`; `workbench.css` supplies the task-surface overrides. Do not use the mode prefixes to hardcode a screen to one theme.

### Primary

Monochrome `accent-primary` supplies filled action emphasis; `primary-ink` supplies its contrasting text. `accent-hover` changes the fill on hover. This is an achromatic action role, not a brand hue.

### Neutral

`bg-canvas` anchors the application; `bg-surface` identifies task and chrome surfaces. `bg-subtle`, `bg-elevated`, `bg-hover`, and `bg-active` separate supporting surfaces and interaction states. `text-primary` carries task content, `text-secondary` supports explanations, and `text-muted` carries lower-priority text and placeholders. Border roles progress from subtle separators through default boundaries to strong field strokes.

Human messages use `bubble-human-bg` and `bubble-human-border`, but their rendered text uses `text-primary` in both modes. Do not use the legacy light-theme `bubble-human-text` value: it is overridden in the workbench. Agent messages use `bg-surface`.

Danger, warning, and muted success roles are semantic exceptions to the neutral palette, not alternate branding. Status text must retain its meaning without relying on hue alone. There is no secondary or tertiary decorative accent.

**The Neutral Actions Rule.** Use monochrome emphasis for primary actions; reserve semantic color for meaningful status, warning, and destructive states.

## Typography

The system sans stack is normative for body text and controls. The mono stack expresses paths, terminal output, technical session metadata, and the ASCII identity. These are fallback stacks; this documentation does not assert that JetBrains Mono is bundled or installed.

The headline, title, body, label, metadata, path, and ASCII roles are recorded in frontmatter. Task headings use medium weight, while ordinary body text uses regular weight. Supporting section titles also occur at 16px; conversation titles at 14px; chat headings at 17px. These observed exceptions are not a new global type scale. Body copy typically stays within 72ch where explanatory paragraphs need a measure; message text uses a 1.65 line height.

On phones the task headline reduces to 22px and the wordmark to 6px. The wordmark uses a compact preformatted line height; its companion monogram is 12px/1 on desktop and 10px/1 on phones. Preserve whitespace in the identity. Supply accessible RAIN text and keep decorative ASCII hidden from assistive technology, as the implemented identity component does.

## Layout

Above 960px, the application is a grid with a 184px navigation rail and flexible main column, beneath an 80px application bar. Rail items are 48px high with 16px horizontal inset and 16px icon-to-label gap; rail padding is 24px 12px. The task viewport owns scrolling while application chrome stays fixed in the layout.

At 960px and below, navigation becomes a 64px bottom bar. The location title and connection-status copy leave the header; theme and connection actions remain. Header horizontal padding progresses from 24px to 20px, then 16px at 600px. Main task padding progresses from 32px to 24px, then 20px 16px at 600px. Files use 24px 32px desktop padding and 20px 16px on phones.

The reusable spacing rhythm is a 4px base with 8px, 12px, 16px, 24px, and 32px steps. Existing component exceptions include a 20px files gap and toolbar padding. Launch forms have a two-column grid that collapses at 768px; metrics collapse to one column at 600px. The conversation roster is nominally 320px wide, with separate roster/conversation visibility on small screens; verify the final cascade when changing this area rather than assuming a general-purpose responsive sidebar.

**The Scope Continuity Rule.** Keep the selected machine visible in launch and file tasks. Changing it clears the explorer and editor; do not imply a local write when the selected filesystem is remote.

## Elevation & Depth

Ordinary task surfaces are flat, separated by tone and thin borders. Machine rows, topology surfaces, and terminals explicitly remove their former shadows. The retained light-theme card-shadow token is a legacy low-elevation option, not the machine-row treatment. Dialogs use the overlay shadow recorded in the sidecar; toasts use the smaller transient shadow. Filter underlines and mobile active-navigation lines are state indicators, not elevation.

**The Flat Workbench Rule.** Use tonal surfaces and dividers for ordinary task structure; reserve pronounced shadows for overlays.

## Shapes

Use small rectangular corners for controls and task surfaces, with the small radius in frontmatter. The middle radius remains on launch-form inputs; dialogs use the large radius. Machine rows and status labels are square and flat. Pills are forbidden. Tiny circular status dots remain status glyphs, not pill-shaped controls. Progress tracks are square-ended and 3px high.

## Components

### Buttons

Outlined actions are transparent with a default border, medium 13px text, and the small radius. Primary actions invert through the theme's accent and ink roles. Standard actions have a 40px minimum height; at 600px and below the general minimum becomes 44px. Catalog and launch actions already have a 44px minimum. Hover uses the neutral hover surface, or accent-hover for filled actions. Disabled buttons use 0.45 opacity and a not-allowed cursor.

Keyboard focus uses a 2px solid text-primary outline with 3px offset; the agent workbench uses accent-primary for the corresponding ring. Preserve native semantics and accessible labels for icon-only controls. The theme button names the theme it will switch to; narrow header actions hide their visible text while retaining the accessible label.

### Inputs / Fields

Message and terminal fields use the small radius, canvas fill, strong border, 14px text, and 44px minimum height. Launch inputs retain the middle radius and 12px padding. Machine selects use the small radius, strong border, 10px padding, and 44px minimum height. Placeholder text explicitly uses text-muted at full opacity. File paths use the mono stack, wrap safely, and retain their form interaction. The file editor has an explicit focus-visible outline; do not remove it when adjusting editor layout.

### Navigation

Desktop active items use bg-active with primary text and medium weight. Bottom navigation indicates the current view with a 2px top inset line; conversation filters use a 2px bottom line. Existing filter/status class names containing “pill” do not authorize pill geometry. State labels remain text-first and square.

### Cards / Containers

The machine inventory is divided rows with 24px vertical padding, transparent backgrounds, no corner treatment, and no shadow. File rows have 16px padding and 64px minimum height. Breadcrumbs use the subtle surface and small radius. The launcher is a broad surface with its own task heading, machine picker, discovery actions, and progressive disclosure for advanced options.

### Messages and overlays

Message bubbles use small corners, 12px 16px padding, and 14px text at 1.65 line height. Role differences come from neutral surface assignments and metadata, not saturated chat colors. Dialogs use large corners, a viewport maximum of calc(100vw - 32px), and 24px section padding, reduced to 20px on phones. Toasts invert text and canvas roles and use small corners.

### ASCII RAIN identity

Preserve the exact preformatted monogram and wordmark in `RainIdentity.tsx`; it is the shared application identity. Do not replace it with a font approximation or use it as a substitute for a task heading.

Component background and shape specimens in the sidecar are static HTML/CSS demonstrations of these patterns. They are not operational launch, navigation, file, or save controls. Motion is limited to short state transitions, with reduced-motion overrides disabling transitions, animations, and smooth scrolling.

## Do's and Don'ts

### Do:

- Do preserve both theme mappings when adding a surface.
- Do keep selected machine and path context legible before launch, edit, save, or terminal actions.
- Do use system sans for task labels and mono for paths, terminal content, and ASCII identity.
- Do retain visible keyboard focus and explicit placeholder contrast in both themes.
- Do keep groups as conversation-routing and workflow records; agent sessions have independent lifecycles.

### Don't:

- Don't introduce gradients, blue hues, or pill controls.
- Don't turn ordinary controls and labels into ASCII art.
- Don't wrap every task section in a raised card.
- Don't hide machine context to gain space.
- Don't treat screenshot review as physical-device, SSH, or comprehensive accessibility certification.


Documentation scope: extracted from the implemented tokens, workbench overrides, Header/RainIdentity components, PRODUCT.md, surface brief, and design contract. Existing review results at `/tmp/rain-ui-review/results.json` record local rendered checks at 320, 390, 768, 1280, and 1440px and both theme mappings; these were read as evidence, not rerun by this documentation pass. The implementation reviewer cleared the dark placeholder and editor focus fixes at fix-list scope. Independent editor verification recorded an active TEXTAREA, focusVisible true, and a computed rgb(32, 32, 32) solid 2px outline. Browser review also sent /help through the populated chat composer to an actual echo process and captured desktop and phone views. These checks do not imply a broader review disposition. Physical phones, Android packaging, and configured SSH destinations are outside this review's evidence. This document records implementation, not user approval of unseen screenshots.
