---
name: Readur
description: A calm, colourful document archive you can read at a glance; every colour carries a job.
colors:
  bg: "#F3F5F8"
  surface: "#FFFFFF"
  surface-2: "#F7F9FB"
  surface-sunken: "#EBEEF2"
  line: "#E0E5EB"
  line-strong: "#7E8895"
  fg: "#141A22"
  fg-2: "#3F4854"
  fg-meta: "#5A6472"
  accent: "#2F62A6"
  accent-hover: "#244F8A"
  accent-soft: "#E4ECF7"
  accent-fg: "#FFFFFF"
  new: "#B23F18"
  new-fill: "#E8633C"
  new-soft: "#FCE9E2"
  new-fg: "#FFFFFF"
  ok: "#1D7337"
  ok-soft: "#E2F2E6"
  warn: "#8F5400"
  warn-fill: "#F8A840"
  warn-soft: "#FDF0D9"
  danger: "#B42318"
  danger-soft: "#FCE8E6"
  danger-fg: "#FFFFFF"
  src-1-teal: "#107064"
  src-2-purple: "#9636BC"
  src-3-green: "#22732C"
  src-4-olive: "#5A6B13"
  src-5-orange: "#9A5111"
  src-6-rose: "#BA2859"
  src-7-magenta: "#A83198"
  src-8-indigo: "#6151CC"
  src-1-soft: "#E3F7F4"
  src-2-soft: "#F1E3F7"
  src-3-soft: "#E3F7E6"
  src-4-soft: "#F3F7E3"
  src-5-soft: "#F7ECE3"
  src-6-soft: "#F7E3EA"
  src-7-soft: "#F7E3F4"
  src-8-soft: "#E6E3F7"
  selection: "#C9DAF1"
  scrollbar: "#C3CAD3"
  dark-bg: "#0F1318"
  dark-surface: "#161B22"
  dark-surface-2: "#1C232C"
  dark-surface-sunken: "#0B0E12"
  dark-line: "#29313C"
  dark-line-strong: "#627080"
  dark-fg: "#E7EBF0"
  dark-fg-2: "#BAC3CE"
  dark-fg-meta: "#96A1AE"
  dark-accent: "#8DB3EA"
  dark-accent-hover: "#A9C6F0"
  dark-accent-soft: "#1D2D44"
  dark-accent-fg: "#0F1318"
  dark-new: "#FF9A76"
  dark-new-soft: "#3A2119"
  dark-new-fg: "#1A0B05"
  dark-ok: "#5BD48A"
  dark-ok-soft: "#15301F"
  dark-warn: "#F8B85A"
  dark-warn-soft: "#33280F"
  dark-danger: "#F97066"
  dark-danger-soft: "#3A1A18"
  dark-src-1-teal: "#36C0AD"
  dark-src-2-purple: "#C79ED7"
  dark-src-3-green: "#66C072"
  dark-src-4-olive: "#9EB739"
  dark-src-5-orange: "#DBA16E"
  dark-src-6-rose: "#DC9AB0"
  dark-src-7-magenta: "#D59ACD"
  dark-src-8-indigo: "#ADA7DA"
  dark-selection: "#27406A"
  dark-scrollbar: "#3A4452"
typography:
  page-title:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "26px"
    fontWeight: 650
    lineHeight: 1.2
    letterSpacing: "-0.015em"
  section-title:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.2
    letterSpacing: "normal"
  body:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  meta:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  reading:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.6
    letterSpacing: "normal"
  table-head:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "11px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "0.06em"
    fontVariation: "'wdth' 75"
  data:
    fontFamily: "Martian Mono Variable, ui-monospace, monospace"
    fontSize: "12px"
    fontWeight: 400
    lineHeight: 1.45
    fontFeature: "'tnum' 1"
rounded:
  sm: "6px"
  md: "8px"
  lg: "12px"
  pill: "999px"
spacing:
  s-1: "4px"
  s-2: "8px"
  s-3: "12px"
  s-4: "16px"
  s-5: "24px"
  s-6: "32px"
  s-7: "48px"
components:
  button-primary:
    backgroundColor: "{colors.accent}"
    textColor: "{colors.accent-fg}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "40px"
  button-primary-hover:
    backgroundColor: "{colors.accent-hover}"
    textColor: "{colors.accent-fg}"
  button-secondary:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.fg}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "40px"
  button-secondary-hover:
    backgroundColor: "{colors.surface-sunken}"
  button-ghost:
    textColor: "{colors.fg-2}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "40px"
  button-ghost-hover:
    backgroundColor: "{colors.surface-sunken}"
    textColor: "{colors.fg}"
  button-danger:
    textColor: "{colors.danger}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
    height: "40px"
  button-danger-hover:
    backgroundColor: "{colors.danger-soft}"
  button-danger-solid:
    backgroundColor: "{colors.danger}"
    textColor: "{colors.danger-fg}"
    rounded: "{rounded.sm}"
  button-sm:
    height: "32px"
    padding: "0 12px"
  status-pill-indexed:
    backgroundColor: "{colors.ok-soft}"
    textColor: "{colors.ok}"
    rounded: "{rounded.pill}"
    height: "22px"
    padding: "0 8px"
  status-pill-processing:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.accent}"
    rounded: "{rounded.pill}"
    height: "22px"
  status-pill-pending:
    backgroundColor: "{colors.surface-sunken}"
    textColor: "{colors.fg-2}"
    rounded: "{rounded.pill}"
    height: "22px"
  status-pill-check:
    backgroundColor: "{colors.warn-soft}"
    textColor: "{colors.warn}"
    rounded: "{rounded.pill}"
    height: "22px"
  status-pill-failed:
    backgroundColor: "{colors.danger-soft}"
    textColor: "{colors.danger}"
    rounded: "{rounded.pill}"
    height: "22px"
  new-tag:
    backgroundColor: "{colors.new}"
    textColor: "{colors.new-fg}"
    rounded: "{rounded.pill}"
    height: "18px"
    padding: "0 7px"
  quiet-pill:
    backgroundColor: "{colors.warn-fill}"
    textColor: "{colors.fg}"
    rounded: "{rounded.pill}"
    height: "22px"
    padding: "0 8px"
  source-badge:
    backgroundColor: "{colors.src-1-soft}"
    textColor: "{colors.src-1-teal}"
    rounded: "{rounded.pill}"
    padding: "2px 8px"
  field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.fg}"
    rounded: "{rounded.sm}"
  panel:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.md}"
    padding: "16px"
  dialog:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
  nav-item:
    textColor: "{colors.fg-2}"
    rounded: "{rounded.sm}"
    height: "38px"
  nav-item-active:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.accent}"
  table-row-selected:
    backgroundColor: "{colors.accent-soft}"
---

# Design System: Readur

## Overview

**Creative North Star: "The Colour-Coded Archive"**

Readur is a calm, colourful archive you can read at a glance. Every colour does one job. Blue means you can act. Coral means new. Amber means quiet or warning. Each source has its own hue, and your collections keep the colours you gave them. The palette comes from the logo: steel blue (#4878B8), coral (#F07048) and amber (#F8A840), each shifted in lightness until it passes AA. Content sits on raised white panels over a faintly blue-grey ground. In dark mode the panels are lighter surface steps rather than flat black. Type is Archivo at its normal width, in sentence case, with filenames shown exactly as they are named.

Density is operational but unhurried. Home answers "is it still flowing?" in one line, then a strip of real page thumbnails, then the busiest source lanes, each with a 14-day arrival strip in the source's own colour. Search tells a story over time: results come grouped by month under a timeline of every match. The document page is for reading. It has a compact header with one facts line, the viewer and the OCR text side by side, and no nested scrolling.

This system rejects two looks by name. One is the grey wireframe: v1's all-caps monochrome board, which the user called "very empty and plain". The other is the stat-tile SaaS dashboard.

**Key Characteristics:**
- Colour is semantic: accent for action, coral for new, amber for warn and quiet, one hue per source, the user's colours for labels.
- Raised `--surface` panels over a tinted `--bg`. Shadows in light mode, surface steps in dark.
- Archivo at normal width in sentence case. Condensed caps only in table heads and tiny meta labels.
- Martian Mono only for numbers in dense data, plus literal code, paths and tokens.
- Status is always a tinted pill with a shape glyph and a word, never colour alone.
- Radii of 6px for controls, 8px for panels, cards and anchored popovers, and 12px for modal overlays.

**History.** v1, the "departure board", was monochrome, set in condensed caps with hairline rules and a single yellow signal. The user rejected it as "very empty and plain … not just blacks and grays". It also made the document page hard to read. v2, "warm it up", kept v1's information architecture: the destinations, the Library slideout, the litStore new-and-changed semantics (now drawn in coral instead of yellow), and the test suite. v2 replaced v1's visual world completely. Older token names (`--ground`, `--signal`, `--btn-bg` and others) survive only as aliases for code that has not been converted yet. New code does not use them.

## Colors

A cool neutral ground carrying four signal colours from the logo, plus an eight-hue source wheel that deliberately avoids blue.

### Primary
- **Logo Steel Blue** (accent; dark theme: dark-accent): primary buttons, the active nav item, links, the focus ring, the caret, the selected table row (accent-soft), the processing pill and the Search timeline bars (which are clickable). Hover deepens it in light mode and lightens it in dark mode (accent-hover). Blue on screen always means "you can act here" or "this is where you are".

### Secondary
- **Logo Coral** (new, new-fill, new-soft): the new/changed signal driven by litStore. It appears as the pill-shaped "New" tag (white on the deeper coral), as a 6px dot beside changed table rows and unread notifications (new-fill), as a 1px coral ring on changed "Just arrived" thumbnails, and as the notification count badge. It replaces v1's yellow `--signal`.

### Tertiary
- **Logo Amber** (warn, warn-fill, warn-soft): warnings and "Quiet". A quiet source lane is tinted warn-soft and carries a warn-fill "Quiet" pill with ink text. "Check" pills use warn on warn-soft. warn-fill is never used as text colour.

### Status
- **Indexed green** (ok / ok-soft): indexed and healthy.
- **Danger red** (danger / danger-soft / danger-fg): failed, error and destructive actions. It is text-only until the confirm dialog, where it becomes a filled button.

### Source hues
- **Teal, purple, green, olive, orange, rose, magenta, indigo** (src-1 … src-8, each with a -soft tint): one stable hue per source. Uploads always get slot 1 (teal) and the watch folder always gets slot 2 (purple). Every configured source hashes its id (FNV-1a) into slots 3–8, so it keeps the same hue on every screen and across reloads (`lib/sourceColor.ts`). They are used for sidebar source dots, Home lane dots and arrival bars, and source badges (hue on its own tint).

### Label colours
- Collections keep the user's chosen colour. It shows as a sidebar dot, and as a chip tinted 14% toward the label colour with a 40% border, while the chip text stays `--fg`. Label colours are user data. The system does not police their hue.

### Neutral
- **Blue-grey ground** (bg): the page behind the panels.
- **Panel white** (surface): every panel, card, table, popover and dialog.
- **Quiet fill** (surface-2): secondary buttons, table heads, row hover and field lookalikes.
- **Sunken well** (surface-sunken): document viewers, image letterboxes, pending pills and ghost hover.
- **Hairline** (line) and **Control edge** (line-strong): dividers and panel borders, and borders that must reach 3:1 (control edges, scrollbar hover).
- **Ink / Secondary ink / Meta ink** (fg, fg-2, fg-meta): body text, supporting text, and timestamps and counts.
- **Selection** and **Scrollbar**: text selection is tinted blue; scrollbar thumbs are a neutral pill.

### Named Rules
**The Every Colour Has A Job Rule.** Blue for action, coral for new, amber for quiet or warning, green and red for state, a hue per source, and the user's colours for collections. A colour with no job does not ship.

**The Blue Is Taken Rule.** No source hue may sit within 25° of the accent's hue in either theme. `scripts/contrast-check.mjs` enforces this. Today the nearest is indigo, at 32–34°. A source badge must never read as a button.

**The AA Both Ways Rule.** Every text/ground pair must reach 4.5:1 in both themes, and every UI mark (focus ring, control edge, new-fill dot) must reach 3:1. This covers every source hue on every ground and on its own tint. `scripts/contrast-check.mjs` checks all 83 pairs. A new token is not finished until it passes.

## Typography

**Display Font:** none. There is no display face. Page titles are Archivo.
**Body Font:** Archivo Variable (with system-ui, sans-serif), self-hosted via @fontsource.
**Label/Mono Font:** Martian Mono Variable (with ui-monospace, monospace).

**Character:** Archivo at its normal width is sturdy and plain-spoken. It reads like a well-kept office, not a terminal. Martian Mono appears only where digits need to line up.

### Hierarchy
- **Page title** (650, 26px, 1.2, -0.015em, balanced wrap): one per page, in sentence case ("Good afternoon, admin", a collection's name, a document's real filename clamped to two lines). The document title drops to 22px on phones.
- **Section title** (600, 16px): panel heads such as "Just arrived", "Coming in" and "Processing", and Search month headings.
- **Body** (400, 14px, 1.45): the default for rows, forms and lines.
- **Meta** (400, 12px): source kind, "Last arrival 5m ago", counts beside month headings and sidebar list heads (600).
- **Reading** (400, 15px, 1.6, max 72ch): the OCR text pane. The Monospace view option switches to 13px/1.65 within 88ch.
- **Table head** (600, 11px, 0.06em tracking, uppercase, 75% width): only table column heads and tiny meta labels of 12px or less.
- **Data** (Martian Mono, 12px, tabular figures): sizes, counts, dates in dense rows and IDs, plus literal code, paths, URLs and API tokens.

### Named Rules
**The Sentence Case Rule.** Titles, headings, buttons, nav, pills and status words are in sentence case ("Indexed", "Check", "Quiet"). Filenames always show their real case. Caps are reserved for table column heads and tiny meta labels.

**The Numbers-Only Mono Rule.** Martian Mono is for figures that need to align, and for literal machine strings. Never use it for names, sources, headings or prose.

## Layout

The shell is a two-column grid: a fixed 248px sidebar and the content column. The sidebar sits on `--surface` with a hairline right edge and scrolls on its own. Top to bottom it holds the logo and wordmark; a Quick find field lookalike that opens the ⌘K palette; the destinations Home, Search, Library, Intake and Settings; Collections (label dots and counts, top ~8, "All collections"); Sources (source dots with health pills, plus a "synced … ago" readout); and a footer with the user, notifications, language and theme.

Below 900px the sidebar becomes a left drawer (at most 320px or 86vw, with a 12px trailing radius and the overlay shadow). It opens from a slim 56px top bar. A 60px bottom tab bar keeps Home, Search, Library and Intake, with 44px targets. Below 720px, tables that are still too wide scroll inside their own panel and never the page, and some rows restack. At 390px there is no horizontal overflow.

Spacing uses a 4px module (4, 8, 12, 16, 24, 32, 48). Page sections stack 24px apart. Panels pad 16–24px.

Key surface patterns:
- **Home:** the page title is a greeting, with one summary line beneath it: "1,311 arrived this week · nothing waiting · ▲ 190 failed Review". Below it come a "Just arrived" panel of real page thumbnails with name, source dot and relative time; a "Coming in" panel of the five most active source lanes (dot and name, 14-day bar strip, today's count, last arrival, health pill), with the rest behind a more row; and a Processing panel with one line per state and humanised failure reasons (raw text behind a disclosure).
- **Search:** a large search field, then the result count, a month timeline of every match (click a bar to filter the date range), and facets. Results are grouped under month headings with counts, newest first, with relevance order toggleable. Each result shows a thumbnail, filename, source badge, date, highlighted OCR snippet and a checkbox. Selecting results reveals "Save as collection", and the new collection appears in the sidebar immediately.
- **Library:** Grid ⇄ Table (persisted). Grid cards are grouped by month. A collection route (`?label=`) shows the collection's name and colour as the header. Both views share the slideout.
- **Document:** a compact header with breadcrumb, the real-case title (two lines at most), a status pill and one facts line ("PDF · 1 page · 2.1 KB · Scanner inbox · Added … · OCR 85%", with empty values hidden). A filled Download button, secondary Share, a Comments toggle and an overflow menu. Label chips, and a collapsible Details section. The body fills the viewport with a Document | Side by side | Text switch. Side by side is the default from 1200px up (viewer ~60%) and is allowed from 960px. Arriving with `?q=` pre-fills find, highlights every match and lands on the first one.

Relative times come from one formatter (`lib/relativeTime.ts`): "21 min. ago" or "now" in the UI language. Past seven days it shows the calendar date ("30 Sep 2026").

## Elevation & Depth

This is a hybrid system. In light mode, content panels are lifted off the ground by one soft, blurred, offset shadow. Dark mode has no panel shadow (`--shadow-1: none`). Depth there comes from surface steps: the ground (dark-bg), then the panel (dark-surface), then the quiet fill (dark-surface-2), with the sunken well going darker still. Overlays get a heavier shadow and a scrim in both themes.

### Shadow Vocabulary
- **Raised panel** (`box-shadow: 0 1px 2px rgba(20,26,34,0.05), 0 2px 8px rgba(20,26,34,0.06)`): panels, tables, grid cards, thumbnails and the primary button. Dark: none.
- **Overlay** (`box-shadow: 0 12px 32px rgba(20,26,34,0.16), 0 2px 6px rgba(20,26,34,0.08)`; dark `0 16px 40px rgba(0,0,0,0.55), 0 2px 8px rgba(0,0,0,0.4)`): dialogs, slideouts, the drawer, popovers, menus, toasts and the palette.
- **Scrim** (`rgba(20,26,34,0.28)`; dark `rgba(5,7,10,0.6)`): behind modal overlays and the drawer.

### Named Rules
**The One Lift Rule.** A panel is lifted once. Never put a card inside a card. Inside a panel, group with hairlines, surface-2 fills or whitespace.

## Shapes

Corners are gently rounded and tied to role. Controls (buttons, fields, nav items, chips, the Quick find trigger) use 6px. Panels, cards, tables, anchored popovers, menus, listboxes and toasts use 8px. Modal overlays (dialogs, the command palette, the mobile drawer's trailing edge) use 12px. Pills (status, source badges, the New tag, count badges, filter chips, switches, scrollbar thumbs) are fully round. Dots are 8px circles (10px in the medium size), with a 6px circle for the changed-row marker. Arrival bars have 2px top corners and Search timeline bars 3px. Checkboxes use 4px. Borders are 1px hairlines. There are no thick or coloured edges.

## Components

### Buttons
Filled where it matters, quiet elsewhere, so nothing reads as a wall of outline boxes.
- **Shape:** gently rounded (6px). 40px tall by default, 32px small, 600 weight, 14px.
- **Primary:** filled accent with white text and the raised-panel shadow. Use one per view for the main action (Add documents, Download, Save).
- **Secondary:** surface-2 fill with a 1px hairline. On hover the fill sinks and the border strengthens.
- **Ghost:** transparent with fg-2 text. Hover adds a sunken fill.
- **Danger:** red text only. Hover adds a danger-soft fill. The filled red version (dangerSolid) appears only in the confirm dialog.
- **States:** pressed moves the button down 1px (not under reduced motion). The focus ring is 2px accent at a 2px offset. Disabled buttons use meta ink on surface-2. Pending shows a small spinner, which is static under reduced motion.

### Status pills
- **Style:** a 20–22px pill with a shape glyph and a 600-weight word, in sentence case. Tones: Indexed/Healthy ■ ok on ok-soft; OCR/Syncing ◐ accent on accent-soft; Pending ○ and Off — fg-2 on sunken; Check ◆ warn on warn-soft; Failed/Error ▲ danger on danger-soft. Processing can show progress ("OCR 3/12").
- **Quiet:** Home's lane-level pill uses warn-fill with ink text on a warn-soft lane. "Idle" is an outlined neutral pill.

### Chips and badges
- **Source badge:** a pill in the source's tint with its hue as text, led by an 8px dot. The source name uses Archivo, never mono.
- **Label chip:** a 6px-radius chip tinted toward the label's colour with a slightly stronger border. The text stays ink. It comes in 20, 24 and 28px heights.
- **New tag:** an 18px coral pill with the word "New", bold, in white on the deeper coral.
- **Filter chips:** pills in the Search and Library facet strip.

### Cards / Containers
- **Corner Style:** 8px.
- **Background:** surface over bg.
- **Shadow Strategy:** raised panel in light mode, surface step in dark (see Elevation).
- **Border:** a 1px hairline on tables and panels that need an edge.
- **Internal Padding:** 16–24px. Panel heads pair a 16px section title with a right-aligned text link ("Open library", "Manage sources").
- **Grid card:** a thumbnail above the name and meta. Hover strengthens its border to line-strong. Selected gets an accent-soft fill with an accent border.

### Tables (BoardTable)
- A raised panel with a sticky surface-2 head in caps table-head type. Rows are 40px (32px in compact mode) and use sentence-case cells. Hover tints to surface-2 and selected rows use accent-soft. Keyboard focus draws a 2px accent inset outline around the whole row. Changed rows carry a 6px coral dot. Numbers, sizes and dense dates use Martian Mono with tabular figures.

### Inputs / Fields
- **Style:** a 1px border on surface with a 6px radius. The Quick find lookalike uses a surface-2 fill and shows its shortcut in a Kbd chip.
- **Focus:** the 2px accent ring at a 2px offset, the same everywhere. The caret is accent.
- **Error / Disabled:** error text in danger; disabled in meta ink.

### Navigation
- **Sidebar items:** 38px tall, 14px at weight 500, fg-2, with an 18px icon. Hover uses surface-2. The active item is accent-soft with accent text at 600. Collections and Sources rows are 32px, led by a colour dot, with a right-aligned count or health pill.
- **Mobile:** a slim top bar with the drawer button, a drawer that slides in from the left (200ms, expo out), and bottom tabs with 44px targets.

### Overlays
- **Dialog:** surface with a 12px radius, the overlay shadow and a scrim. Widths are 400, 560 and 800px.
- **Slideout (Library detail):** an edge-attached surface panel with the overlay shadow. It holds the real-case title, a status pill with one facts line, a letterboxed preview on the sunken well, and a filled Open button.
- **Command palette:** 12px radius, topmost layer.
- **Popover / Menu / Toast:** surface with an 8px radius and the overlay shadow. Items use 6px.

### Arrival strip (signature)
Each Home lane draws 14 day-bars in its source hue: past days at 55% opacity, today at full strength, and zero days as a 3px stub in the source's soft tint. On first paint the bars grow up from the baseline (480ms, expo out, staggered 18ms per bar). Under reduced motion they are static. This is the one authored motion moment.

### Motion
Durations are 120ms (state changes), 200ms (drawers, overlays) and 480ms (the arrival strip). Easing is `cubic-bezier(.2,.8,.2,1)` for state and `cubic-bezier(.16,1,.3,1)` for entrances. `prefers-reduced-motion` sets every duration to 0 and turns off the spinner, pressed-state translation and bar growth.

## Do's and Don'ts

### Do:
- **Do** use the accent only for things you can act on or where you are: primary buttons, links, active nav, focus, selection and clickable timeline bars.
- **Do** mark new and changed items with coral (the New tag, a 6px dot or a 1px thumbnail ring), driven by litStore.
- **Do** give every source its `sourceHue()` slot, and use the same dot, bar and badge colour for it everywhere.
- **Do** show status as a tinted pill with a shape glyph and a sentence-case word.
- **Do** put content on one raised surface panel over bg: shadow-1 in light mode, a surface step in dark.
- **Do** use 6px radii on controls, 8px on panels, cards and anchored popovers, and 12px on modal overlays.
- **Do** keep titles in sentence case and filenames in their real case, with titles clamped to two lines.
- **Do** run `node scripts/contrast-check.mjs` after any token change. Both themes must pass.
- **Do** humanise failure reasons and put raw error text behind a disclosure.
- **Do** use `formatRelativeTime` for every "ago" and switch to the calendar date after seven days.

### Don't:
- **Don't** use coloured side stripes or thick coloured borders on rows, cards, lanes or callouts. Use a tint, pill or dot instead.
- **Don't** build hero-metric tiles or stat-card rows. Home states its numbers in one summary line.
- **Don't** put eyebrow or kicker labels above headings.
- **Don't** use gradient text.
- **Don't** set titles, headings, buttons or status words in caps, or use condensed caps anywhere except table heads and tiny meta labels.
- **Don't** use Martian Mono for names, sources, headings or prose.
- **Don't** give a source a blue hue, or anything within 25° of the accent.
- **Don't** convey state by colour alone.
- **Don't** nest cards inside cards.
- **Don't** return to v1's monochrome, hairline-only, all-caps board, or to v1's yellow signal.
