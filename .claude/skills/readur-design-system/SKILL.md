---
name: readur-design-system
description: Use for ANY frontend UI work in readur — new screens, cards, lists, badges, forms, dialogs, restyling, colours, spacing, icons, empty/loading/error states — anything touching frontend/src/**/*.tsx or *.module.css.
---

# Readur design system (Studio)

Every visible object is rendered by a primitive from `frontend/src/ui` (import from `'../../ui'`). Feature CSS only lays primitives out. If nothing fits, add a variant to a primitive — never local look-alike markup.

## Hard rules

1. **Tokens only.** No hex, `rgb()`, px `border-radius`, literal shadows or `z-index` numbers in `*.module.css`. Use the names in `references/tokens.md` exactly; never guess a token or add a fallback like `var(--fs-sm, var(--fs-md))`.
2. **No legacy aliases.** `--ground`, `--signal`, `--focus`, `--btn-bg`, `--danger-bg` exist only for old code. Use `--bg`, `--new`, `--accent`, `--danger-soft`.
3. **Focus rings come from the primitive.** Don't write `:focus-visible` outlines for buttons, links-as-buttons, chips or cards — use `Button`, `IconButton`, `DocumentCard`, `LabelChip`.
4. **Never colour alone, never coloured body text.** A failure is `<StatusMark state="failed" />` plus a neutral reason (`--fg-2`); a source is `SourceBadge`; a label is `LabelChip`. Don't colour paragraphs with `--danger`/`--ok`.
5. **Light and dark are equal.** Check both (`/dev/ui`, theme toggle) before finishing.
6. **Accent is swappable.** Never encode teal outside the accent tokens.
7. **Copy goes through i18n** in en, de, es and fr.

## Which primitive

| Need | Use |
|---|---|
| Action | `Button` (`primary`, `secondary` tonal, `ghost`, `danger` tonal, `danger-solid` only inside confirm dialogs); `IconButton` always has `label` |
| Surface / panel | `Card` (don't hand-roll background + radius + shadow) |
| Rows of records | `BoardTable`; a short list inside a `Card` with hairline rows is fine for ≤5 items |
| Document in a grid | `DocumentCard` with a `DocumentThumbnail` in `thumbnail` |
| Document / source state | `StatusMark` (`progress` for OCR n/m; spinner is automatic) |
| Where it came from | `SourceBadge` (tile); `chip` variant in dense text. Not `SourceDot` for new code |
| Label | `features/labels/Label` (wraps `LabelChip`) |
| Filter | `FilterChip` |
| Inline message | `Notice` (`info`/`ok`/`warning`/`danger`, optional `action`, `onDismiss`) |
| Transient feedback | `useToast().show({ title, tone })` |
| Read-only key/values | `Facts`; header strips `Pass` |
| Nothing here | `EmptyState` (`illustration` for first-run, `icon` otherwise) |
| Loading | `Skeleton` shaped like the content; `Spinner` only inline |
| Progress | `ProgressBar` (one value), `OutcomeBar` (done/failed/queued split) |
| Choose | `Select`, `ComboBox`, `Segmented` (2–3 always-visible options), `Tabs`, `Switch`, `Checkbox`, `ChoiceGroup` + `ChoiceTile` |
| Overlay | `Dialog` (`icon`, `helpLink`), `SlideOver`, `Menu` + `MenuSection`, `Popover`, `BulkActionBar`, `CommandPalette` |
| Person | `Avatar` |

Props, variants and examples: `references/primitives.md`. Page anatomy and states: `references/patterns.md`.

## Before you finish

Run `references/review-checklist.md` and say which checks you ran.
