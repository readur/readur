---
version: 1
slug: "frontend-src"
primary_target: "frontend/src"
related_targets: []
---

## Scope

The whole Readur web app, all surfaces. Visitor mode: **Operate**.

- **Audience:** self-hosters, plus non-technical office staff with one admin.
- **Jobs:** (1) quickly find and sort imported documents; (2) bring documents in and check that connections are healthy.
- **Constraints:** no runtime CDN, WCAG 2.2 AA, light and dark mode, 4 locales, no source file of 1000 lines or more.
- **Memorable moment:** a lit row. Something changed (OCR finished, sync failed), and it stays marked until you've seen it.
- **Unresolved:** none.

## Direction contract

**THESIS:** Readur as a live departure board for paperwork. Documents are passes; the library and intake are boards you scan and sort; anything that changed stays lit until seen. Refuses the stat-card-grid SaaS dashboard with an indigo accent.

**OWN-WORLD:**
- **Light:** ground #EEF0F2, surface #FFF, ink #0B0D10, fg-2 #3B424A, meta #5A6169.
- **Night board:** ground #0B0D10, panel #15191E, hairline #262B31.
- **Signals:** signal yellow #FFD400 appears only as a fill with ink text or as a 3px edge bar. Red is only for failed or destructive states.
- **Type:** Archivo variable, condensed, uppercase and tracked for labels and column heads; normal width for body text. Martian Mono with tabular figures for every number.
- **Structure:** passes are segmented cells with small uppercase labels above mono values. Every state is shape plus word (○ ◐ ■ ▲ ◆).
- **Finish:** 1px hairlines, 4px radius, no gradients, shadows only on overlays.

**STORY:** A filer opens the Library, types, and sees matching passes with OCR snippets. They sort any column, open the slideout, and label or download without leaving the board. An admin opens Intake, reads connection health at a glance, clears lit failures in Needs attention, and drops new files in.

**FIRST VIEWPORT (Library):**
- **Header bar:** wordmark, the tabs Board / Library / Intake / Settings, the ⌘K search field, a mono "synced 2m ago" readout, alerts, and the user menu.
- **Filter strip:** one row of facet chips (Type, Label, Status, Source, Added) plus sort.
- **Board table:** full width, uppercase condensed heads NAME, TYPE, PAGES/OCR, SOURCE, LABELS, SIZE, ADDED; mono rows; lit rows have a yellow edge bar.
- **Detail slideout:** a 440px panel from the right on row select, with ↑/↓ to move between rows and Esc to close.
- **Primary action:** "Add documents" at the top right of the page.

**FORM:** Departure-board challenger (vernacular-ephemera-boarding-pass-and-gate-board), seed key 2e64ce3b, chosen over the assigned wayfinding direction by the user's steer. Raises taken from declined challengers:
- **tensegrity:** distinct state shapes.
- **Miura:** settings fold open from a summary.
- **Ikeda:** tabular figures everywhere.
- **Dumbar:** one module grid.

**SIGNATURE INTERACTION:** rows re-rank in place (FLIP) when sort or live updates change order. Lit rows keep the edge bar until acknowledged. Under reduced motion, changes happen instantly and the highlight persists.

**FINISH:** unreviewed and undocumented is unfinished; this build ends with the finish review, the verdict, DESIGN.md, and every shipping raster carrying its provenance.
