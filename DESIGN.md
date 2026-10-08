---
name: Readur
description: Studio — calm tinted surfaces, one indigo accent, an amber "new" signal, rounded and soft; every colour carries a job.
colors:
  bg: "#F0F1F7"
  surface: "#FFFFFF"
  surface-2: "#FFFFFF"
  surface-sunken: "#E4E6F0"
  line: "#D8DBE8"
  line-strong: "#737896"
  fg: "#161A2E"
  fg-2: "#474C66"
  fg-meta: "#585D78"
  accent: "#4338CA"
  accent-hover: "#3730A3"
  accent-soft: "#E2E3F9"
  accent-fg: "#FFFFFF"
  new: "#8F5300"
  new-fill: "#BF7A0A"
  new-soft: "#FCEFD6"
  new-fg: "#FFFFFF"
  ok: "#1C7247"
  ok-soft: "#D8F0E2"
  warn: "#A3440C"
  warn-fill: "#E8782E"
  warn-soft: "#FCE5D5"
  danger: "#B4313A"
  danger-soft: "#F8DDE0"
  danger-fg: "#FFFFFF"
  src-1: "#1F6E9E"
  src-2: "#9636BC"
  src-3: "#22732C"
  src-4: "#5A6B13"
  src-5: "#9A5111"
  src-6: "#BA2859"
  src-7: "#A83198"
  src-8: "#12766A"
  src-1-soft: "#DFEEF7"
  src-2-soft: "#F1E3F7"
  src-3-soft: "#E3F7E6"
  src-4-soft: "#F3F7E3"
  src-5-soft: "#F7ECE3"
  src-6-soft: "#F7E3EA"
  src-7-soft: "#F7E3F4"
  src-8-soft: "#DDF2EE"
  selection: "#CDD0F7"
  scrollbar: "#BDC1D6"
  accent-soft-hover: "#D3D5F5"
  dark-bg: "#10121C"
  dark-surface: "#171A28"
  dark-surface-2: "#1D2132"
  dark-surface-sunken: "#0B0D15"
  dark-line: "#272B3D"
  dark-line-strong: "#646A88"
  dark-fg: "#E7E9F5"
  dark-fg-2: "#B3B8CF"
  dark-fg-meta: "#959BB6"
  dark-accent: "#A5B4FC"
  dark-accent-hover: "#C0CBFD"
  dark-accent-soft: "#262B52"
  dark-accent-fg: "#12153A"
  dark-new: "#FBC15E"
  dark-new-fill: "#FBC15E"
  dark-new-soft: "#3A2C10"
  dark-new-fg: "#1F1400"
  dark-ok: "#6FD69F"
  dark-ok-soft: "#15301F"
  dark-warn: "#FFA066"
  dark-warn-fill: "#FFA066"
  dark-warn-soft: "#3A2312"
  dark-danger: "#F28B93"
  dark-danger-soft: "#3A1A1E"
  dark-danger-fg: "#2A0A0E"
  dark-src-1: "#7CC0E8"
  dark-src-2: "#C79ED7"
  dark-src-3: "#66C072"
  dark-src-4: "#9EB739"
  dark-src-5: "#DBA16E"
  dark-src-6: "#DC9AB0"
  dark-src-7: "#D59ACD"
  dark-src-8: "#6FCFBF"
  dark-src-1-soft: "#16303F"
  dark-src-2-soft: "#2D1736"
  dark-src-3-soft: "#17361B"
  dark-src-4-soft: "#2F3617"
  dark-src-5-soft: "#362517"
  dark-src-6-soft: "#361721"
  dark-src-7-soft: "#361731"
  dark-src-8-soft: "#163330"
  dark-selection: "#2E3570"
  dark-scrollbar: "#383D55"
  dark-accent-soft-hover: "#2F3563"
typography:
  page-title:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "24px"
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.02em"
  page-title-large:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 700
    lineHeight: 1.1
    letterSpacing: "-0.02em"
  section:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 700
    lineHeight: 1.3
  body:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "13px"
    fontWeight: 500
    lineHeight: 1.45
  reading:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.6
  meta:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.45
  caps-label:
    fontFamily: "Plus Jakarta Sans Variable, system-ui, sans-serif"
    fontSize: "10.5px"
    fontWeight: 600
    lineHeight: 1
    letterSpacing: "0.07em"
  data:
    fontFamily: "JetBrains Mono Variable, ui-monospace, monospace"
    fontSize: "12px"
    fontWeight: 500
    lineHeight: 1.45
    fontFeature: "'tnum' 1"
rounded:
  sm: "8px"
  md: "10px"
  lg: "14px"
  xl: "20px"
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
    rounded: "{rounded.md}"
    height: "36px"
  button-secondary:
    backgroundColor: "{colors.accent-soft}"
    textColor: "{colors.accent}"
    rounded: "{rounded.md}"
    height: "36px"
  button-danger:
    backgroundColor: "{colors.danger-soft}"
    textColor: "{colors.danger}"
    rounded: "{rounded.md}"
  icon-button:
    backgroundColor: "{colors.surface-sunken}"
    textColor: "{colors.fg-2}"
    rounded: "{rounded.md}"
    size: "34px"
  input:
    backgroundColor: "{colors.surface}"
    borderColor: "{colors.line-strong}"
    rounded: "{rounded.md}"
    height: "40px"
  card:
    backgroundColor: "{colors.surface}"
    rounded: "{rounded.lg}"
  dialog:
    backgroundColor: "{colors.surface-2}"
    rounded: "{rounded.xl}"
---
# Design System: Readur (Studio)

Spec: `docs/superpowers/specs/2026-10-07-readur-design-system-design.md`. Working rules for agents: `.claude/skills/readur-design-system/`. Every primitive in every state: `/dev/ui` (development builds).

## Overview

Studio is calm and soft. Pages sit on a tinted background (`--bg`), content sits on white (or deep indigo-grey in dark mode) cards with 14px corners and a gentle shadow, and one indigo accent marks what you can act on. Light and dark are designed as equals. Density is comfortable: 36px buttons, 40px inputs, 40px table rows.

Colour is never the only signal. Status is a dot or spinner plus a word; a source is a hue tile plus a type icon plus its name; a label is a dot plus its name.

## Colors

All colours live in `frontend/src/styles/tokens.css` and nowhere else. Primitives are checked by `src/ui/__tests__/tokens-only.test.ts`; both themes are checked by `scripts/contrast-check.mjs` (4.5:1 for text, 3:1 for marks and control borders).

### Accent
`--accent` (indigo) is the brand and the action colour: primary buttons, focus rings, selected rows, active nav, current page. `--accent-soft` is its tint for secondary buttons, selection and highlights. The accent is one swappable group: `--accent`, `--accent-hover`, `--accent-soft`, `--accent-soft-hover`, `--accent-fg`, `--focus-halo` and `--selection`. Changing those in the light block, the dark block and the pre-theme media block rebrands the app; nothing else may encode the accent hue.

### Status
`--ok`, `--warn`, `--danger` and `--new` (amber, for things that changed since you last looked), each with a `-soft` tint.

### Source hues
`--src-1` … `--src-8` give each source a stable hue (uploads blue, the watch folder purple, other sources hashed). They stay at least 25° of hue away from the accent so a source never reads as an action.

### Neutral
`--bg`, `--surface`, `--surface-2` (raised: popovers, dialogs, toasts), `--surface-sunken` (wells, tracks, search), `--line`, `--line-strong` (control borders), `--fg`, `--fg-2`, `--fg-meta`, `--hover`, `--press`.

## Typography

Plus Jakarta Sans for everything you read; JetBrains Mono (tabular numbers) for data: sizes, counts, dates, page numbers, fractions, shortcuts. Both are self-hosted through `@fontsource-variable`; nothing loads from a CDN.

Page titles are 24/700 with −0.02em tracking; section headings 14/700; body 13; meta 12; small uppercase caps labels (10.5/600, +0.07em) only for table heads, menu groups and gallery rows.

## Layout

4px spacing scale (`--s-1` … `--s-7`). A page is: page header (breadcrumb, title, subtitle, actions) → filters → content (cards, a contained table or a document grid). The shell is a full-width top bar with the global search aligned to the content column and a classic sidebar beneath it (Plan 2).

## Elevation & Depth

Two shadows: `--shadow-1` for resting cards, `--shadow-overlay` for anything floating (popovers, dialogs, slide-overs, toasts, the bulk bar). Raised buttons add `--bevel`, a 1px inner highlight. Layers: `--z-local` (inside a component) < `--z-shell` < `--z-dock` < `--z-overlay` < `--z-toast` < `--z-palette`.

## Shapes

`--radius-sm` 8px (small buttons, menu rows), `--radius` 10px (buttons, inputs, icon buttons, nav items), `--radius-lg` 14px (cards, tables, notices), `--radius-xl` 20px (dialogs, slide-over, command palette), `--radius-pill` (chips, labels, segmented controls, switches).

## Components

All in `frontend/src/ui`, exported from `frontend/src/ui/index.ts`.

- **Buttons**: raised, 10px corners. Primary is solid accent; secondary is a tonal accent tint; ghost is neutral text; danger is a tonal danger tint; `danger-solid` only confirms inside a dialog. `IconButton` is a 34px soft square that is always visible.
- **Fields**: outlined, 40px, accent border plus a 3px halo on focus, danger border plus halo when invalid. The search field is a sunken bar with no border. Select lists tint the chosen row and show a check.
- **Toggles**: rounded-square checkboxes; `ChoiceTile` rows for options with descriptions; M3-style switch with a check in the thumb.
- **Tabs and Segmented**: underline tabs; a pill track with a raised thumb for two or three view choices.
- **StatusMark**: a haloed dot plus a quiet word; in-progress states spin and can show `OCR 3/12` with a mini bar.
- **LabelChip, FilterChip, SourceBadge**: dot pills; outline filter pills that tint and check when active; hue icon tiles.
- **Notice and Toast**: white cards with a coloured left stripe; toasts add a timer line.
- **Card, DocumentCard, BoardTable, Facts, Pass**: 14px cards; thumbnail-first document cards; a contained table with a sunken header, zebra rows and an accent edge on selection; definition-list facts.
- **EmptyState, Skeleton, Pagination, Progress**: an illustrated first-run state; shimmer placeholders shaped like content; numbered pages; `ProgressBar` and the segmented `OutcomeBar`.
- **Overlays**: top-anchored dialogs with an icon tile and help link; a floating slide-over; grouped menus; a spotlight command palette with a preview pane; a floating surface bulk-action bar.

### Motion
Short and functional: `--dur-1` 120ms for hovers, `--dur-2` 200ms for panels. Every keyframe animation (spinners, shimmer, toast timer) stops under `prefers-reduced-motion`.

## Do's and Don'ts

### Do
- Build from `src/ui` primitives and tokens; extend a primitive with a variant when nothing fits.
- Check light and dark, desktop and phone widths, and the keyboard path.
- Put new copy through i18n in en, de, es and fr.

### Don't
- Write hex colours, px radii, literal shadows or z-index numbers in feature CSS.
- Recreate a badge, chip, card or spinner locally.
- Use colour as the only way to tell states apart.
- Hard-code the accent colour (indigo) anywhere outside the accent tokens.
