---
name: Readur
description: A self-hosted document archive that reads like a live board you scan, sort, and clear.
colors:
  ground: "#EEF0F2"
  surface: "#FFFFFF"
  surface-2: "#F6F7F8"
  line: "#D5D9DE"
  ink: "#0B0D10"
  fg-2: "#3B424A"
  fg-meta: "#5A6169"
  signal: "#FFD400"
  danger: "#B42318"
  danger-bg: "#FDECEA"
  ok: "#1F7A3A"
  night-ground: "#0B0D10"
  night-surface: "#15191E"
  night-surface-2: "#1C2127"
  night-line: "#262B31"
  night-fg: "#E8EAED"
  night-fg-2: "#B5BBC2"
  night-fg-meta: "#9AA0A6"
  night-danger: "#F97066"
  night-danger-bg: "#2A1414"
  night-ok: "#4ADE80"
typography:
  display:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "40px"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.005em"
    fontVariation: "'wdth' 68"
  headline:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "28px"
    fontWeight: 600
    lineHeight: 1.1
    letterSpacing: "0.06em"
    fontVariation: "'wdth' 75"
  title:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 600
    lineHeight: 1.1
    letterSpacing: "0.06em"
    fontVariation: "'wdth' 75"
  body:
    fontFamily: "Archivo Variable, system-ui, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.45
    letterSpacing: "normal"
  label:
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
  figure:
    fontFamily: "Martian Mono Variable, ui-monospace, monospace"
    fontSize: "20px"
    fontWeight: 600
    lineHeight: 1
    fontFeature: "'tnum' 1"
rounded:
  tag: "2px"
  base: "4px"
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
    backgroundColor: "{colors.ink}"
    textColor: "{colors.surface}"
    typography: "{typography.label}"
    rounded: "{rounded.base}"
    padding: "0 16px"
    height: "40px"
  button-primary-hover:
    backgroundColor: "{colors.fg-2}"
    textColor: "{colors.surface}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.base}"
    padding: "0 16px"
    height: "40px"
  button-secondary-hover:
    backgroundColor: "{colors.surface-2}"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.base}"
    padding: "0 16px"
    height: "40px"
  button-danger:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.danger}"
    typography: "{typography.label}"
    rounded: "{rounded.base}"
    padding: "0 16px"
    height: "40px"
  button-sm:
    padding: "0 12px"
    height: "32px"
  input-field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    rounded: "{rounded.base}"
    padding: "0 12px"
    height: "40px"
  filter-chip:
    backgroundColor: "transparent"
    textColor: "{colors.ink}"
    rounded: "{rounded.base}"
    padding: "0 8px"
    height: "28px"
  filter-chip-active:
    backgroundColor: "{colors.surface}"
  change-tag:
    backgroundColor: "{colors.signal}"
    textColor: "{colors.ink}"
    typography: "{typography.label}"
    rounded: "{rounded.tag}"
    padding: "0 4px"
  board-table-head:
    backgroundColor: "{colors.surface-2}"
    textColor: "{colors.fg-meta}"
    typography: "{typography.label}"
    padding: "0 12px"
    height: "32px"
  board-table-row:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    typography: "{typography.body}"
    padding: "0 12px"
    height: "40px"
  board-table-row-compact:
    height: "32px"
  pass-cell:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    padding: "8px 12px"
  slide-over:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    width: "440px"
    padding: "24px"
  command-palette:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ink}"
    rounded: "{rounded.base}"
    width: "600px"
  bulk-action-bar:
    backgroundColor: "{colors.ink}"
    textColor: "{colors.surface}"
    rounded: "{rounded.base}"
    padding: "8px 12px"
  top-bar:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.fg-2}"
    height: "56px"
    padding: "0 16px"
---

# Design System: Readur

## Overview

**Creative North Star: "The Paperwork Board"**

Readur reads like a live departure board for documents. Every surface is a board you scan top to bottom: condensed uppercase column heads, one hairline per row, numbers in a monospaced data face that never jitters. A document's facts are laid out as a segmented strip of labelled cells (internally, a "pass"). When something changes (OCR finishes, a sync fails) the row is marked and stays marked until the person has seen it. That persistent mark is the one moment of colour the system allows itself.

The palette is ink on cool paper by day and pale type on a night board after dark, with a single signal yellow and a red kept for failure. Depth is flat: hairlines and tonal steps separate things, and shadows appear only on elements that float above the page. Density is high but calm. Rows are 40px (32px compact), labels are small and tracked, and whitespace comes from a 4px module rather than from padded cards. The system turns down the stat-card-grid SaaS dashboard with an indigo accent. Figures sit in strips and tables, never in isolated tiles.

The metaphor words (departure board, pass, lit, gate, boarding) belong to design documents and code comments only. User copy says what things are: "Needs attention", "Mark all seen", "New", "Changed", "Indexed", "Failed". The shipped navigation tab "Board" and the region title "Arrivals" are the only exceptions, and neither should become a precedent for more metaphor in copy.

**Key Characteristics:**
- Condensed, uppercase, tracked Archivo for every label, head, tab, and button. Normal-width Archivo for reading text.
- Martian Mono with tabular figures for every number, size, date, count, and type code.
- 1px hairlines and a 4px radius everywhere. No gradients.
- Signal yellow only as a filled tag with ink text, a 3px edge bar, a search-match highlight, or a count badge.
- Every state is a shape plus a word (○ ◐ ■ ▲ ◆ —). Colour alone never carries meaning.
- Light and night themes share one token set. Only values change, never structure.

## Colors

The system is ink and cool greys, with one signal yellow and one functional red and green for outcomes.

### Primary
- **Board Ink** (ink): the text, the strong hairline under table heads and around overlays, the primary button fill, and the active-tab underline. On the night board the ink pair inverts: primary buttons and the bulk bar become pale (night-fg) with ink text.

### Secondary
- **Signal Yellow** (signal): the "something changed" colour. It appears in four places only: the NEW / CHANGED tag (filled, ink text), the 3px left edge bar on a changed row, the search-match highlight inside OCR snippets, and the unread count badge on the alerts bell. It is also the text-selection colour and the night-mode focus ring. It never tints a surface, never carries body text, and never marks success.

### Tertiary
- **Failure Red** (danger / night-danger): failed OCR, sync errors, destructive buttons (outlined red on surface), invalid fields, and danger toasts on the danger-bg tint. Red never decorates.
- **Indexed Green** (ok / night-ok): the ■ INDEXED and ■ HEALTHY marks and success toast icons. It is used as text colour only, never as a fill.

### Neutral
- **Cool Paper** (ground): the page behind every board. Night: night-ground, which is the same value as ink.
- **Panel White** (surface): tables, strips, regions, fields, and overlays. Night: night-surface.
- **Row Tint** (surface-2): table heads, hover and selected rows, the search trigger, disabled fields, and the key-cap background. Night: night-surface-2.
- **Hairline** (line): every 1px divider between rows, cells, and regions. Night: night-line.
- **Second Ink** (fg-2): secondary text, snippets, field labels, and the primary-button hover fill.
- **Meta Grey** (fg-meta): column heads, small labels, placeholders, and mono meta lines. It passes AA on both surface and ground in both themes.

### Named Rules
**The Lit Until Seen Rule.** Yellow means "changed since you last looked", and nothing else. A marked row keeps its edge bar through sorting, paging, and reduced motion until the person opens it or marks everything seen.

**The Two Placements Rule.** Signal yellow is either a fill with ink text on top or a 3px bar. It is never yellow text, a yellow border, or a yellow surface.

**The Red Is Failure Rule.** Red is only for failed, error, invalid, or destructive states.

## Typography

**Display Font:** Archivo Variable at 68% width (with system-ui)
**Body Font:** Archivo Variable at normal width (with system-ui, sans-serif)
**Label/Mono Font:** Archivo Variable at 75% width for labels; Martian Mono Variable (with ui-monospace) for data

**Character:** A condensed grotesk shouting in caps at gate-board scale, paired with a wide, even monospace that makes every figure sit in a column. Body text stays normal-width Archivo so OCR snippets and descriptions read comfortably. Both families are self-hosted through @fontsource-variable, with no CDN.

### Hierarchy
- **Display** (800, 40px, line-height 1, 68% width, uppercase, -0.005em): the page title in the page header (BOARD, LIBRARY, a document's filename). There is one per page. It wraps anywhere rather than truncating.
- **Headline** (600, 28px, 1.1, 75% width, uppercase, 0.06em): standalone titles such as the sign-in panel, the upload drop zone, and the shared-document title.
- **Title** (600, 16px, 75% width, uppercase, tracked): slide-over and dialog titles and the wordmark (at 700).
- **Body** (400, 14px, 1.45): reading text, row names (at 500), OCR excerpts, and field values. Secondary copy is 12px.
- **Label** (600, 11–12px, 75% width, uppercase, 0.06em): column heads, pass-cell labels, chip labels, tabs, buttons (12px), field labels (12px, fg-2), region titles (12px, 700), and status words.
- **Data** (Martian Mono, 11–12px, tabular): sizes, dates, relative ages, counts, type codes, the "synced" readout, key caps, and meta lines. Pass-cell values in a document header step up to 20px.
- **Figure** (Martian Mono 600, 20px, line-height 1): the count set beside the display title ("892 documents", "0 in queue"). In board strips, figures run at 16px.

### Named Rules
**The Every Number Is Mono Rule.** Numbers, sizes, dates, durations, counts, and file-type codes are set in Martian Mono with tabular figures. This applies in tables, chips, badges, pass cells, and the bulk-bar count.

**The Width Carries Rank Rule.** Hierarchy comes from width and case before size. Condensed caps mean structure (heads, labels, controls). Normal width means content. Don't set reading text in condensed caps, and don't set labels in normal-width sentence case.

## Layout

Content sits in a 1440px maximum column with 24px padding (16px on phones), under a sticky 56px top bar. Spacing follows a 4px module (4 / 8 / 12 / 16 / 24 / 32 / 48). Most gaps between regions are 16px. Cell padding is 8×12. Overlays and dialogs pad by 24px.

The board uses one 12-column grid with a 16px gap. The attention list and the totals strip span all 12 columns. Recent documents take 8 columns and the side stack (processing, connections) takes 4, and all of them go to full width below 900px. The library is one full-width table under a single-row filter strip (search, facet chips, view controls pushed to the end).

**Narrow-screen folding** (breakpoint 720px, with sub-steps at 600, 480, and 400):
- The top nav becomes a fixed 56px bottom tab bar with icon-over-label items (44px minimum targets, safe-area inset). The search field collapses to a 20px icon, and the "synced" readout hides.
- A board table stacks each row instead of scrolling sideways. Line 1 is the checkbox and the name at full width (two lines at most). Line 2 is the change tag, the status mark, and one mono meta line of the folded columns separated by "·". Long text and any kept action column follow. Heads of folded columns stay in the grid for assistive tech only. The changed-row edge bar moves from the first cell to the row.
- Column sorting moves from the heads to a sort select. The density toggle hides.
- A pass (container query) drops to two columns below 480px, with hairlines drawn as 1px gaps over the line colour. A cell left alone on the last row spans both columns, and cells marked wide (timestamps) take a full row below 400px.
- Board strips turn each segment vertical below 600px and set figures in two columns below 480px.
- The slide-over goes full width. The bulk bar docks above the bottom tab bar.

### Named Rules
**The Stack, Never Scroll Rule.** On phones a table row folds into lines; the page never scrolls sideways. A table only scrolls inside its own box if it is still wider than the screen after folding.

## Elevation & Depth

The system is flat. Depth at rest comes from tonal steps (ground, then surface, then surface-2) and 1px hairlines. The strong ink hairline marks structure that matters: the line under table heads, selected rows, the active tab, and overlay edges. There is one shadow, and it belongs only to things that float above the page.

### Shadow Vocabulary
- **Overlay** (`box-shadow: 0 8px 24px rgba(11, 13, 16, 0.16)`; night `0 8px 24px rgba(0, 0, 0, 0.6)`): slide-over, dialog, command palette, popover, menu, toast, and the bulk action bar.

Focus is not a shadow. It is a 2px outline with a 2px offset in ink by day and signal yellow by night. Inside tables it is inset (a -2px offset, or 2px inset strokes along a focused row).

Stacking uses named layers, and nothing invents its own z-index: shell 1100 < dock 1150 < overlay 1200 < toast 1300 < palette 1400 < skip link 1500. Sticky table heads use a local z-index of 1.

### Named Rules
**The Only Floaters Cast Rule.** A shadow means "this sits above the page". Tables, regions, strips, and cards at rest have none.

## Shapes

Corners are small and consistent: 4px on every button, field, chip, table container, region, pass, overlay, key cap, and label tag. The NEW / CHANGED tag in library rows and search-match highlights use a tighter 2px. Label colour swatches are 8px squares with a 2px radius. Circles are reserved for things that are round by function: the spinner, radio and status dots, and the 16px count badge on the alerts bell. Borders are always 1px. The heavier strokes are the 2px active-tab underline, the 2px bottom edge on key caps, and the 3px signal edge bar. Segmented surfaces (passes, board strips) draw their dividers as 1px gaps over the line colour, so hairlines stay continuous however the cells reflow.

## Components

### Buttons
Condensed caps on flat blocks, with no drop shadows.
- **Shape:** 4px radius, 1px border, 40px tall (32px small), 16px horizontal padding (12px small), 8px icon gap.
- **Primary:** ink fill with white label on light, and the inverse on night. There is one per view: "Add documents" at the top right of the page header.
- **Hover / Focus:** primary hover shifts to fg-2. Secondary and ghost hover to surface-2. Pressed moves down 1px (none under reduced motion). Focus is the 2px outline token. Transitions are 120ms on colour only.
- **Secondary:** surface fill with an ink border. **Ghost:** transparent (for example "Mark all seen"). **Danger:** surface fill with a red border and red text, tinting danger-bg on hover. **Disabled:** meta text on surface-2 with a hairline border.
- **Icon buttons** are square (32 or 40px). Top-bar tool icons are 20px. Inside table rows, controls shrink to 28px tall so they sit inside the hairlines.

### Chips
- **Filter chip:** 28px tall, transparent with a hairline border. A condensed caps facet label (TYPE, LABEL, STATUS, SOURCE, ADDED), then a mono value, a chevron, and, once active, a clear button separated by a hairline. Active chips move to surface with an ink border.
- **Label tag:** 20/24/28px tall on surface-2 with a hairline border, an 8px colour swatch, a normal-width name, and a mono count. The user-assigned label colour appears only in the swatch.

### Cards / Containers
- **Region:** surface with a hairline border and 4px radius. The head is 40px tall with a condensed caps title (12px, 700) and one text action ("View all", underlined caps). The body pads 12px or is flush for tables.
- **Background:** surface on ground. There are no nested cards; sub-structure is drawn with hairlines.
- **Shadow Strategy:** none at rest (see Elevation & Depth).

### Inputs / Fields
- **Style:** 40px minimum height, surface fill, hairline border, 4px radius, 12px horizontal padding. The label sits above in 12px condensed caps (fg-2). Placeholders are fg-meta.
- **Focus:** hover darkens the border to ink. Keyboard focus adds the 2px outline token.
- **Error / Disabled:** invalid fields take a red border with red 12px error text below. Disabled fields are surface-2 with meta text.
- **Search trigger (top bar):** a 32px field-shaped button on surface-2 with a mono key cap (Ctrl K / ⌘K). It opens the command palette.

### Navigation
- **Top bar:** 56px, surface, with a hairline bottom border. Left to right: the wordmark (logo plus READUR in 16px condensed caps at 700), then tabs in 14px condensed caps (fg-2, turning ink on hover), then the centred search trigger (420px maximum), then the mono "synced" readout, then alerts, language, theme, and user icons. The active tab shows ink text and a 2px ink underline.
- **Mobile:** a fixed bottom tab bar with 56px icon-over-label items in 11px caps. The active tab carries a 2px ink bar along its top edge.
- **Page header:** the display title, with a mono figure beside it on the same baseline, actions pushed right, and a hairline beneath. Document pages add a small caps breadcrumb above the title.

### Board Table (signature)
The sortable, selectable grid that every list uses.
- **Head:** a sticky 32px row on surface-2 in 11px condensed caps (fg-meta) with an ink hairline beneath. The sort arrow (▲/▼) shows on hover and stays on the sorted column, which is ink.
- **Rows:** 40px (32px compact) on surface with a hairline between rows. The name is in body at 500, beside a 40×28 type stub or thumbnail. Type, size, and age are mono, with sizes right-aligned. Hover turns the row surface-2. Selected rows are surface-2 with inset ink hairlines above and below. Keyboard focus draws a 2px inset frame around the whole row.
- **Changed rows:** a 3px signal bar on the left edge plus a NEW or CHANGED tag beside the name. Both persist until seen. When sorting or live updates change the order, rows re-rank in place (FLIP), and under reduced motion they reorder instantly while the mark stays.
- **Detail rows:** an optional second line with an OCR snippet (12px, fg-2, clamped to two lines), with matches highlighted in signal.
- **Loading / empty:** skeleton rows at row height. An empty state pads 32×16.

### Pass (signature)
A document's facts as a segmented strip: surface with a hairline border and 4px radius, and cells divided by 1px hairlines. Each cell is an 11px condensed caps label above a value. Values are mono when they are data and normal-width body when they are words. In the document page header, cells pad 12×16 and values step up to 20px. Long values truncate or clamp and reveal the full text only when clipped. Board totals use the same construction: a segment head followed by figure cells with labels above 16px mono values.

### Status Mark (signature)
A shape plus a condensed caps word, never colour alone: ○ PENDING, ◐ OCR (or OCR 3/12 with known progress), ■ INDEXED, ▲ FAILED, ■ HEALTHY, ◐ SYNCING, ◆ CHECK, ▲ ERROR, — OFF. The glyph is set in the mono face. Green is for INDEXED and HEALTHY, red for FAILED and ERROR, and fg-2 for everything else. It comes in two sizes, 11px and 12px, and its words are translated.

### NEW / CHANGED Tag
An inline tag in condensed caps with signal fill and ink text. NEW marks a document added since last seen. CHANGED marks a state change since last seen, such as OCR finishing or a failure. On phones it leads the row's meta line.

### Slide-Over
A 440px panel from the right on a light scrim (ink at 20%, or 50% at night) so the board stays readable behind it. It has an ink left edge and the overlay shadow, and slides in over 200ms (instant under reduced motion). The sticky header is 56px with a title in condensed caps and the status mark beside it. The body pads 24px. The sticky footer holds right-aligned actions. ↑/↓ move between rows and Esc closes it. On phones it takes the full width and the status mark wraps under the name.

### Command Palette
Opened with ⌘K / Ctrl K. It is a 600px surface panel 12vh from the top with an ink border, 4px radius, and the overlay shadow, on the palette layer. A search field sits above sections with 11px caps headings. Items are 40px with an icon, title, and fg-2 subtitle. The focused item turns surface-2 with a 3px ink left edge.

### Bulk Action Bar
A floating ink bar (inverted at night) docked 24px above the bottom edge, centred, with the overlay shadow. It shows a mono count and a caps word ("3 SELECTED"), then actions and a clear button. Focus rings on the bar use the bar's text colour. On phones it docks above the bottom tab bar.

### Overlays: Dialog, Popover, Menu, Toast
All four are surface panels with an ink border, 4px radius, and the overlay shadow. Dialogs are 400, 560, or 800px wide, with a title in condensed caps and a hairline under it, a 24px body, and right-aligned actions above a hairline. Toasts are 360px, bottom right, and move aside or up when a slide-over is open. A danger toast takes a red border on danger-bg.

## Do's and Don'ts

### Do:
- **Do** set every number, size, date, count, and type code in Martian Mono with tabular figures.
- **Do** pair every state with its shape and word (○ ◐ ■ ▲ ◆ —) through the status mark.
- **Do** mark changes with the 3px signal edge bar plus a NEW / CHANGED tag, and keep both until the person has seen the item.
- **Do** build facts as passes and totals as segmented strips: labels above values, hairlines drawn as 1px gaps.
- **Do** use condensed (75%) uppercase tracked Archivo for heads, labels, tabs, and buttons, and 68% width at 800 for the one page title.
- **Do** take every layer from the z-index tokens and every value from tokens.css. Surfaces never hard-code a colour.
- **Do** fold table rows into lines on phones rather than scrolling sideways.
- **Do** keep the metaphor vocabulary (departure board, pass, lit, gate) out of user-facing copy in all four locales.

### Don't:
- **Don't** build a stat-card grid or tint an accent indigo. Figures belong in strips and tables.
- **Don't** use signal yellow as text, a border, a surface tint, or a success colour.
- **Don't** use red for anything but failure, error, invalid, or destructive states.
- **Don't** add shadows to anything that rests on the page, and don't use gradients.
- **Don't** convey state by colour alone.
- **Don't** put a small caps kicker or eyebrow above a page title. The breadcrumb on document pages is navigation, not a kicker.
- **Don't** set reading text in condensed caps.
