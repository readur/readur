---
version: 2
slug: "frontend-src"
primary_target: "frontend/src"
related_targets: []
---

## Scope

The whole Readur web app, every surface. Visitor mode: **Operate**, except the document reading view, which is **Read**.

- **Audience:** self-hosters, and non-technical office staff with one admin. Both sit on an archive of thousands of documents.
- **Jobs:**
  1. Confirm at a glance that documents are still flowing in from every source: watch folder, WebDAV, S3, local folders, uploads.
  2. Find everything about one topic in a huge archive ("my wife's shoulder injury"), see it as a story over time, and keep it together as a collection.
  3. Read a document and its OCR text comfortably.
- **Constraints:** no runtime CDN, WCAG 2.2 AA, light and dark, 4 locales, no source file of 1000 lines or more.
- **Memorable moment:** Home's source lanes, where each source's 14-day arrival strip shows in its own colour and a source that has gone quiet stands out in amber before anything errors.
- **History:** v1 (the monochrome "departure board", all caps and hairlines) was rejected by the user as "very empty and plain … not just blacks and grays". Its IA, slideout, litStore and tests stay. Its visual world does not.

## Direction contract

**THESIS:** Readur is a calm, colourful archive you can read at a glance. Every colour carries a job:
- blue for action;
- coral for new;
- amber for quiet or warning;
- one hue per source;
- your own label colours for collections.

It refuses both the grey wireframe and the stat-tile SaaS dashboard.

**OWN-WORLD:**
- **Palette from the logo** (`frontend/public/readur.png`): steel blue #4878B8 as the accent (deepened or lightened for AA), coral #F07048 for new, amber #F8A840 for warn.
- **Source hues:** 8 source hues at matched lightness.
- **Surfaces:** layered, with raised panels (soft offset shadow in light mode, surface steps in dark) over a tinted ground. Radii are 6px for controls, 8px for panels, 12px for overlays.
- **Type:** Archivo at normal width. Sentence-case titles and filenames in their real case. Condensed caps only for table heads and tiny meta labels. Martian Mono only for numbers in dense data.
- **Status:** tinted pills that keep shape plus word.

**STORY:** You open Home and see every source's lane: its colour, 14-day arrivals, today's count, and the last arrival. One lane is amber "Quiet", so you click through to that connection. Then you search "shoulder" and get 138 matches, grouped by month, with a timeline showing they cluster in spring 2025. You tick eight results and save them as the collection "Shoulder '25". It appears in the sidebar. You open the MRI report and "shoulder" is already highlighted, match 1 of 7, in a readable text column beside a full-width PDF.

**FIRST VIEWPORT (Home):**
- **Left sidebar:** logo; Home / Search / Library / Intake / Settings; Collections with colour dots; Sources with colour and health dots; utilities in the footer.
- **Main area:**
  - a greeting line with this week's count;
  - source lanes;
  - one processing line;
  - one failures line with a Review action;
  - "Just arrived" as real page thumbnails.

**FORM:** the approved "warm it up" concept, recorded in `.superpowers/sdd/sorted-wondering-treasure/warmup-brief.md`, which is the binding token list and component rules.

**SIGNATURE INTERACTION:** the arrival bars grow in once on Home (reduced motion: static). Opening a document from search lands on the first highlighted match.

**FINISH:** unreviewed and undocumented is unfinished. The build ends with the finish review, the verdict, and DESIGN.md plus `.impeccable/design.json` rewritten for v2.
