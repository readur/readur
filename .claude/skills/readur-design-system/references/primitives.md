# Primitives

All exported from `frontend/src/ui/index.ts`. Icons are React components from `frontend/src/ui/icons` (lucide glyphs under MUI-style names: `Refresh`, `Delete`, `CheckCircle`, `Upload`…; size with `fontSize="inherit" | "small"`). Every `icon` prop takes an element, never a string: `icon={<CheckCircle fontSize="inherit" />}`. Source and tests live in `frontend/src/ui/<Name>/`. See every state live at `/dev/ui` (dev builds).

## Actions

**Button** — `variant: 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-solid'` (default `secondary`), `size: 'sm' | 'md'` (28 / 36px), `icon`, `isPending` (shows `Spinner`, sets `aria-busy`), plus React Aria props (`onPress`, `isDisabled`, …).
```tsx
<Button variant="primary" icon={<Upload fontSize="inherit" />} onPress={openPicker}>{t('upload')}</Button>
```
Don't: use `danger-solid` outside a confirm dialog; put two primary buttons side by side.

**ButtonLink** — the same looks for an action that goes somewhere: `href`, `variant`, `size`, `icon`. Navigates in-app through `app/RacRouterBridge`.
```tsx
<ButtonLink href="/sources?section=connections&new=1" variant="secondary" icon={<Cloud fontSize="inherit" />}>{t('home.connectSource')}</ButtonLink>
```

**IconButton** — `label` (required: accessible name *and* tooltip), `icon`, `size`, `variant` (`danger` tints the glyph), `disabledReason` (muted, focusable, presses ignored, tooltip says why — use instead of `isDisabled` when the person should learn the reason). Always a visible soft square.

**Avatar** — `name`, `size: 'sm' | 'md'`. **Spinner** — `size`, `label` (with a label it is a status region). Use `Spinner` only inline; loading regions use `Skeleton`.

## Inputs

**TextField** — `label`, `description`, `errorMessage` + `isInvalid`, `multiline`, `type`, `placeholder`. **SearchField** — same chrome, sunken bar, Esc clears. **Select** / **ComboBox** + `SelectItem` / `ComboBoxItem`. **Checkbox** — `label`, `description`, `isIndeterminate`. **Switch** — `label`, `description` (instant settings only). **ChoiceGroup** (`label`, `description`, React Aria `CheckboxGroup` props) + **ChoiceTile** (`value`, `label`, `description`) for options that need explaining.
**Tabs** — `Tabs` / `TabList aria-label` / `Tab id` / `TabPanel id` for page sections. **Segmented** — `label`, `items: {id, label, icon?}[]`, `value`, `onChange`, `iconOnly` for 2–3 view choices (Grid | Table).

## Status and labelling

**StatusMark** — `state: pending | processing | completed | failed | healthy | syncing | warning | error | disabled`, `progress?: {current, total}` (processing only → "OCR 3/12" + bar), `size`, `label` (a more specific word in the same tone, e.g. "Idle", "Quiet"), `reason` (plain-language why; dotted underline + tooltip — this is how a row shows its error). Dot + word; spinner for processing/syncing. Never put it on a coloured background.
**LabelChip** — `name`, `color`, `icon`, `count`, `size`, `onPress`, `onRemove` + `removeLabel`. Feature code with a `LabelData` uses `features/labels/Label`.
**FilterChip** — `label`, `value`, `isActive`, `onPress`, `onClear`, `popover`.
**SourceBadge** — `sourceId`, `kind` (`upload`, `watch`, or the source type), `name`, `type` (icon), `variant: 'tile' | 'chip'`, `showType`. Library rows use `features/library/SourceBadge` (`row`, `name`). **SourceTile** — the icon tile alone (`sourceId`, `kind`, `type`) for rows that print the name themselves (sidebar, lists).
**ChangeTag** — children "New"/"Changed" (amber dot + word). **Kbd** — key text in the data face.
**Notice** — `tone: info | ok | warning | danger`, `title`, children, `action`, `onDismiss`, `live: 'alert' | 'status' | 'off'` (default: danger → alert), `prefix` (hidden word).
```tsx
<Notice tone="danger" title={t('sync.unreachable', { host })} action={<Button variant="ghost" size="sm" onPress={edit}>{t('edit')}</Button>}>
  {t('sync.refused')}
</Notice>
```
**Toast** — `const toast = useToast(); toast.show({ title, description?, tone: 'info' | 'success' | 'danger', timeout? })`. Long-running work: `toast.progress(id, { title, description?, value? })` updates one toast in place (no timeout); `toast.dismiss(id)` when it ends. Hovering pauses timed toasts.

## Data

**Card** — `as`, `padding: 'none' | 'sm' | 'md'`, `interactive`. The only way to get a surface panel. Don't nest Cards.
**DocumentCard** — `title`, `thumbnail` (pass `DocumentThumbnail size="fill"`), `meta` (mono "PDF · 412 KB · 2d ago"), `status` (`StatusMark`), `labels` (first two + "+N"), `source` (`SourceBadge`), `flags` (top-right), `href` and/or `onOpen` (both fire on a link), `routerOptions` (history state sent with `href`, e.g. a drawer's `linkState`), `isSelected` + `onSelectionChange` + `selectLabel`, `isChanged`, `quickActions`, `as` (default `li` — wrap in a `ul`).
**BoardTable** — `columns: BoardColumn[]` (`id`, `label`, `render`, `sortable`, `mono`, `isRowHeader`, `hideOnNarrow`, `fold`), `rows`, `getRowId`, `aria-label`, `sort`/`onSortChange`, `selectionMode`/`selectedKeys`/`onSelectionChange`, `onRowAction`, `isLoading`, `emptyState`, `density`.
**Facts** — `items: {label, value, mono?}[]`, `title`. **YesNo** — `value`, `yes`, `no`. **Pass** + **PassCell** (`label`, `mono`, `span`, `wide`) for header strips.
**EmptyState** — `title`, `description`, `action`, `icon` | `illustration`, `headingAs`.
**Skeleton** — `width`, `height`, `lines`, `label`. **Pagination** — `page`, `pageSize`, `total`, `onChange(page, pageSize)`, `pageSizeOptions`.
**ProgressBar** — `value` 0–100, `label`, `showValue`, `tone`. **OutcomeBar** — `label`, `segments: {id, label, value, tone: ok | accent | danger | warn | neutral}[]`.

## Overlays

**Dialog** — `title`, children, `actions`, `isOpen`/`onOpenChange`, `size`, `icon` (accent tile), `helpLink` (footer left), `role="alertdialog"` for destructive confirms, `isDismissable`. Footer order: help link left; ghost Cancel then primary on the right.
**SlideOver** — `title`, `isOpen`, `onOpenChange`, `footer`, `width`, `onNavigate`, `resizable` + `storageKey` (left-edge grab handle; width remembered per key) + `defaultShare` (opening share of the window, default 0.5), `layout="fill"` (no body padding; the content divides the height itself, e.g. a fixed preview above a scrolling tab). ↑/↓ go to `onNavigate` unless focus is in a field, a list or an element marked `data-own-arrows` (a scrolling text pane). Keep the record mounted while it closes (`useLastDefined` from `lib/`) so it can slide out. **SplitHandle** — 8px divider with a three-dot grip between two stacked parts: `label`, `value` (px of the part above), `min`, `max`, `onChange`, `onCommit` (remember it here), `step`. Drag, ↑/↓ or Home/End; owns its arrow keys inside a SlideOver. Its grab area is larger than the bar (24px, 44px on touch, mostly upward) and sits over its neighbours. **Menu** + **MenuItem** (`danger`) + **MenuSection** (`title`) inside **MenuTrigger**. **Popover** + **PopoverTrigger**. **Tooltip** + **TooltipTrigger** (IconButton already has one).
**BulkActionBar** — `count`, `actions: {id, label, icon?, tone?: 'danger', onPress}[]`, `onClear`. **CommandPalette** — `sources: CommandSource[]`; items may carry `preview`.
