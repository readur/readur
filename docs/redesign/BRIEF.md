# Readur redesign — shared brief for every implementer

Branch `feat/ui-departure-board`. Frontend lives in `frontend/`, backend in `src/`. Product truth is in `PRODUCT.md` at the repo root; read it once.

## 1. The world: "Departure board"

- Readur works like a live departure board for paperwork.
  - A document is a **pass**: a segmented card with labelled cells (NAME · TYPE · PAGES/OCR · SOURCE · LABELS · SIZE · ADDED).
  - The Library and Intake are **boards**: dense, sortable rows.
  - Anything that changed is **lit** until the user has seen it. Examples: OCR finished, a sync failed, a new import.
- This is an Operate-mode app. Clarity, scanability, and familiar affordances beat decoration.
- The airline metaphor lives in *structure and grammar only*. No planes, barcodes, airport codes, or tickets. No gradients, no glassmorphism, no glow.
- Never write the words "departure board", "pass", "lit", or anything else from this brief into user-visible copy, code comments, `data-*` attributes, or shipped files. The metaphor is internal.
  - Component names such as `Pass`, `LitRow`, and `BoardTable` are fine.
  - User copy uses plain product words: "Documents", "Needs attention", "New".

## 2. Tokens (the only source of colour, type, space)

`frontend/src/styles/tokens.css` defines all of these as CSS custom properties. Components use `var(--…)` only: no hex literals outside tokens.css, and no inline colour styles. Theme is set by `data-theme="light|dark"` on `<html>`.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--ground` | #EEF0F2 | #0B0D10 | page background |
| `--surface` | #FFFFFF | #15191E | panels, board, passes |
| `--surface-2` | #F6F7F8 | #1C2127 | row hover, insets, table head |
| `--line` | #D5D9DE | #262B31 | 1px hairlines |
| `--line-strong` | #0B0D10 | #E8EAED | emphasis rules, active tab underline |
| `--fg` | #0B0D10 | #E8EAED | primary text |
| `--fg-2` | #3B424A | #B5BBC2 | secondary text |
| `--fg-meta` | #5A6169 | #9AA0A6 | metadata, column heads (AA-verified) |
| `--signal` | #FFD400 | #FFD400 | "changed / needs you": FILL ONLY or 3px edge bar, never text colour |
| `--signal-ink` | #0B0D10 | #0B0D10 | text on signal fill |
| `--danger` | #B42318 | #F97066 | failed / destructive (text + icon + border) |
| `--danger-bg` | #FDECEA | #2A1414 | danger tag/alert fill |
| `--ok` | #1F7A3A | #4ADE80 | indexed/healthy mark |
| `--focus` | #0B0D10 | #FFD400 | 2px focus outline, 2px offset |
| `--btn-bg` / `--btn-fg` | #0B0D10 / #FFFFFF | #E8EAED / #0B0D10 | primary button |

Every text/background pair has been checked at ≥4.5:1. Don't invent new colours. If you need one, ask the orchestrator in your report.

**Type** (self-hosted with `@fontsource-variable/archivo` and `@fontsource-variable/martian-mono`; no CDN, ever):

- `--font-ui: "Archivo Variable", system-ui, sans-serif` for body text at width 100.
- `--font-label`: the same family at `font-stretch: 75%` (condensed), UPPERCASE, `letter-spacing: 0.06em`, weight 600. Use it for column heads, cell labels, nav, and tags.
- `--font-data: "Martian Mono Variable", ui-monospace, monospace` with `font-variant-numeric: tabular-nums`. Use it for every count, size, date, ID, and page number.
- Scale: `--fs-xs 11px`, `--fs-sm 12px`, `--fs-md 14px` (body), `--fs-lg 16px`, `--fs-xl 20px`, `--fs-2xl 28px`, `--fs-3xl 40px` (condensed display). Line-height 1.45 for body, 1.1 for display.

**Space and shape:**

- The module is 4px. Use `--s-1 4px`, `--s-2 8px`, `--s-3 12px`, `--s-4 16px`, `--s-5 24px`, `--s-6 32px`, `--s-7 48px`.
- `--radius 4px`. Pills only for count badges.
- Borders are 1px `var(--line)`.
- Shadows only on overlays: `--shadow-overlay`.
- Row height is 40px (comfortable) or 32px (compact). Minimum touch target is 40px, or 44px on the mobile bottom nav.

**Motion:**

- `--dur-1 120ms`, `--dur-2 200ms`, `--ease: cubic-bezier(.2,.8,.2,1)`.
- Rows re-rank in place with FLIP (`useFlip`).
- Under `prefers-reduced-motion: reduce`, changes happen instantly. Never hide state behind animation, and the lit edge bar stays either way.

## 3. State vocabulary (shape + word, never colour alone)

`StatusMark` renders these. Use it everywhere a document or connection state is shown.

| State | Mark | Word (i18n key `status.*`) |
|---|---|---|
| pending | ○ | PENDING |
| processing | ◐ + `n/m` when known | OCR 3/12 |
| completed/indexed | ■ (`--ok`) | INDEXED |
| failed | ▲ (`--danger`) | FAILED |
| healthy connection | ■ | HEALTHY |
| syncing | ◐ | SYNCING |
| warning/degraded | ◆ | CHECK |
| error connection | ▲ | ERROR |
| disabled | — | OFF |

"Changed / new" is a separate orthogonal flag. It is drawn as the `--signal` 3px left edge bar on the row plus a `NEW` or `CHANGED` tag filled `--signal` with `--signal-ink` text. It clears when acknowledged via `litStore`.

## 4. Architecture rules

- **Behaviour:** `react-aria-components` (RAC). **Styling:** CSS Modules co-located (`X.module.css`). No MUI, Emotion, Tailwind, or styled-components in new code.
- **Layout:**
  - `frontend/src/ui/<Component>/{Component.tsx, Component.module.css, Component.test.tsx, index.ts}` for primitives.
  - `frontend/src/features/<feature>/…` for surfaces.
  - `frontend/src/styles/` and `frontend/src/theme/` for the foundation.
- **Icons:** import from `frontend/src/ui/icons` (the Lucide adapter; it keeps `data-testid="${Name}Icon"`). Until W2 moves it, the adapter lives at `src/design/icons.tsx`.
- **Hard cap: no source file ≥ 1000 lines.** Target ≤ 400. Split by responsibility.
- **API:** go through `frontend/src/services/api` (barrel). Never call axios directly from components.
- **i18n:** all user copy goes through `useTranslation()`.
  - Add keys only under your feature's namespace: `board.*`, `library.*`, `intake.*`, `document.*`, `settings.*`, `auth.*`, `shell.*`, `ui.*`, `status.*`.
  - Put your new keys for **all four** locales (en, de, es, fr) in `.superpowers/sdd/sorted-wondering-treasure/i18n/<task>-<lng>.json`. The orchestrator merges them into `frontend/public/locales/<lng>/translation.json`. Don't edit the translation files yourself.
  - German runs long: cells use `min-width:0` plus ellipsis, and a Tooltip shows the full text.
- **Accessibility floor (WCAG 2.2 AA):**
  - Every control has an accessible name.
  - `:focus-visible` uses `outline: 2px solid var(--focus); outline-offset: 2px`.
  - Headings nest correctly: one `h1` per page.
  - Landmarks: `header`, `nav`, `main`.
  - Nothing is conveyed by colour alone.
- **Shared files belong to the orchestrator.** Don't edit these; list what you need in your report instead:
  - `frontend/package.json` and the lockfile
  - `frontend/src/App.tsx` and `frontend/src/app/routes.tsx`
  - `frontend/public/locales/**`
  - `frontend/src/services/api/index.ts`
  - Exception: tasks that explicitly own one of these files may edit it.
- **Generated API types:** `frontend/src/types/generated/**` is generated from the Rust models by [ts-rs](https://github.com/Aleph-Alpha/ts-rs). Import them with `import type` from the `index.ts` barrel in that folder, and never edit them by hand.
  - Regenerate after changing any Rust type that derives `TS`: `scripts/generate-ts-bindings.sh`, then commit the result. On a host without leptonica/tesseract/libclang/openssl, run `READUR_USE_NIX=1 scripts/generate-ts-bindings.sh` to build inside nix-shell.
  - `scripts/check-ts-bindings.sh` regenerates and fails if the committed files drift (modified, deleted or new). It takes `READUR_USE_NIX=1` too.
  - Adding an API type in Rust: derive `TS` next to `ToSchema` and add `#[ts(export)]`. If the TS name would clash with another exported type, add `rename = "..."`.
  - Mapping rules:
    - Integers (`i64`, `u64`, `usize`, …) are `number` (`TS_RS_LARGE_INT = "number"` in `.cargo/config.toml`); `bigint` never appears. Dates and UUIDs are `string`. `serde_json::Value` is `JsonValue`.
    - Response types: a plain `Option<T>` is `T | null` (the key is always present). An `Option<T>` with `skip_serializing_if = "Option::is_none"` gets `#[ts(optional)]` and becomes `field?: T` (the key is omitted, never `null`). A `#[serde(skip_serializing)]` field gets `#[ts(skip)]`.
    - Request types (JSON bodies and query strings) carry `#[ts(optional_fields)]`, so every `Option<T>` is `field?: T`: serde accepts a missing key. A non-`Option` field with a serde default gets `#[ts(optional = nullable)]`, which makes it `field?: T`.
    - Comma-separated query params (`deserialize_comma_separated*`) are typed as the wire value, `field?: string` (`#[ts(optional = nullable, type = "string")]`), not the parsed `Vec`. Join the values with `,` before sending.
    - Enums follow their serde representation, so the string unions match the JSON exactly.

## 5. Primitive API contracts (built in W2-B; later waves consume them)

All primitives are exported from `frontend/src/ui` (barrel `index.ts`).

- `Button({variant:'primary'|'secondary'|'ghost'|'danger', size:'sm'|'md', icon?, ...RAC ButtonProps})`
- `IconButton({label, icon, ...})`. `label` is required: it becomes aria-label and the Tooltip text.
- `TextField`, `SearchField`, `Select<T>`, `ComboBox<T>`, `Checkbox`, `Switch`, `Tabs`/`TabList`/`Tab`/`TabPanel`: thin RAC wrappers with `label` and `description` and `errorMessage`.
- `Dialog({title, children, actions, isOpen, onOpenChange, size:'sm'|'md'|'lg'})`
- `SlideOver({title, isOpen, onOpenChange, children, footer})`: right-anchored, 440px on desktop and full width below 720px, Esc closes, focus returns to the trigger.
- `Menu`/`MenuItem`, `Popover`, `Tooltip`, `useToast()` returning `{show({title, description?, tone:'info'|'success'|'danger'})}`
- `BoardTable<T>`:
  - props: `columns: {id, label, sortable?, width?, align?, render(row)}[]`, `rows: T[]`, `getRowId`
  - `sort?: {column:string, direction:'ascending'|'descending'}`, `onSortChange`
  - `selectionMode?: 'none'|'multiple'`, `selectedKeys`, `onSelectionChange`
  - `onRowAction?(id)`, `isRowLit?(row)`, `renderRowDetail?(row)` for an optional sub-line such as search snippets
  - `density?: 'comfortable'|'compact'`, `isLoading`, `emptyState`
  - Built on RAC `Table`. Paginated, not virtualized.
- `Pass` / `PassCell({label, children, mono?})`: a segmented labelled cell grid.
- `StatusMark({state, progress?:{current,total}})`: see §3.
- `LitRow` behaviour lives inside BoardTable through `isRowLit`. `useFlip(keys)` is a hook in `ui/motion`.
- `EmptyState({title, description, action?})`, `Skeleton`, `Pagination({page, pageSize, total, onChange})`, `Kbd`.
- `CommandPalette({isOpen, onOpenChange, sources})`. A source is `{id, label, search(q) => Promise<Item[]>}`.
- `BulkActionBar({count, actions, onClear})`: a floating bar at the bottom centre.

## 6. litStore (owned by Board, frozen API)

`frontend/src/features/board/litStore.ts`:

- `markLit(kind:'document'|'source'|'attention', id:string, reason:'new'|'changed'|'failed')`
- `acknowledge(kind, id)`
- `acknowledgeAll(kind?)`
- `isLit(kind, id): boolean`
- `useLit(kind, id): {lit:boolean, reason?:string}`
- `useLitCount(kind?): number`

It persists the currently lit (unseen) entries with their reasons in localStorage under the key `readur.lit.v1`; acknowledging removes an entry and is capped at 2000 entries. It is fed by NotificationContext events and the sync-progress WebSocket. It's safe to call before any lit entry exists.

## 7. Testing conventions

- vitest + Testing Library.
- Query by **role / label / text** only. No `.Mui*` or CSS-module class selectors, and no snapshot tests.
- Every primitive gets:
  - a render test
  - a keyboard interaction test
  - an a11y-name test
- Every surface gets tests for:
  - its happy path
  - the empty state
  - the error state
  - its key interactions: sort, filter, select, and open the slideout
- Use `src/test/test-utils.tsx` `renderWithProviders`. Mock `services/api` the way the existing tests do.
- When you delete an old page, delete its old tests too, and replace them with tests that cover **at least the same behaviours**. Keep the test count up.
- Commands run from `frontend/`:
  - `npm run type-check`
  - `npm run test:unit`
  - `npm run test:integration`
  - `npm run build`
- **Backend:** every cargo command goes through the nix-shell wrapper. The recipe is in the orchestrator's dispatch.

## 8. Commits

- Commit only your owned paths: `git add <paths>` then `git commit -m "…"`.
- Use conventional prefixes: `feat(ui):`, `feat(api):`, `test:`, `chore:`, `refactor:`.
- If you hit `index.lock` because a parallel agent is committing, wait a few seconds and retry.
- Never push, never force.
