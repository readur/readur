# Patterns

## Page anatomy
`PageHeader` (breadcrumb, title, subtitle, secondary + primary actions) → filters (`FilterChip` row, `Segmented` view switch on the right) → content: a `BoardTable`, a `ul` grid of `DocumentCard`s, or `Card`s laid out with CSS grid. Feature CSS may only set layout (grid, gap, padding with `--s-*`).

## A card with a short list
```tsx
<Card as="section" aria-labelledby={headingId}>
  <h2 id={headingId} className={styles.title}>{t('home.sources.title')}</h2>
  <ul className={styles.list}>
    {sources.map((source) => (
      <li key={source.id} className={styles.row}>
        <span className={styles.main}>
          <SourceBadge sourceId={source.id} kind={source.type} type={source.type} name={source.name} showType />
          <span className={styles.meta}>{t('home.sources.synced', { age })}</span> {/* --fg-meta */}
        </span>
        <StatusMark state={source.syncing ? 'syncing' : source.errorCount ? 'error' : 'healthy'} size="sm" />
        <IconButton size="sm" label={t('home.sources.sync', { name: source.name })} icon={<Sync fontSize="small" />} onPress={() => sync(source.id)} />
      </li>
    ))}
  </ul>
</Card>
```
Row CSS: `display: flex; align-items: center; gap: var(--s-3); padding: var(--s-3) 0; border-bottom: 1px solid var(--line)`. Meta in `--fg-meta`; times and sizes in `--font-data`. Explanatory text (an error reason, a description) is `--fg-2` — the `StatusMark` carries the tone.

## States
| State | Use |
|---|---|
| Loading | `Skeleton` with the shape of the content (rows → `lines`, cards → card-sized blocks); give one a `label` |
| Empty | `EmptyState` inside the same `Card` (`illustration` only for first-run screens) |
| Error | `Notice tone="danger"` with a retry `action`; keep it inside the region that failed |
| Error on one row | `StatusMark reason="…"` in the status column; never a second line under the name (rows stay one line) |
| Action not allowed | `IconButton disabledReason="…"`; never an explanatory line in the row |
| Success after an action | `useToast().show({ tone: 'success' })`; failure → `tone: 'danger'` |
| In progress per item | `StatusMark state="processing"` (+ `progress`) or `Button isPending` |

## Forms
Stack `TextField`s with `gap: var(--s-4)`. Instant on/off → `Switch`; a set of options with explanations → `ChoiceGroup`; pick one of many → `Select`/`ComboBox`; 2–3 visible options → `Segmented`. Errors go in `errorMessage` + `isInvalid`, never a red paragraph.

## Overlays
Short confirmation or form → `Dialog` (`icon`, `helpLink`, `role="alertdialog"` when destructive). Details beside a list → `SlideOver`.

**A drawer lives in the URL.** Open it with `useDrawerParam(key)` (`lib/useDrawerParam.ts`): `?<key>=<id>` on the current page, so Back closes it, a reload or a shared link reopens it, and ↑/↓ (`step`) replace rather than pile up history. Never hold an open record in `useState`. Records that exist only in a loaded list use `useRecordDrawer` (intake) and drop an id that is not on the list. Keys in use: `document`, `source`, `failure`, `import`.

**A document opens in the document drawer, never a page.** `useDocumentDrawer().open(id)` (or `href(id)` + `linkState` for a real link) puts `?document=<id>` on whatever page is showing; the shell renders the drawer. A page that lists documents registers them with `useRegisterDocumentList` (Library and Search do, via `useLibraryDrawer`) so ↑/↓ walk its rows and changes made in the drawer patch the row. `/documents/:id` only redirects there. Actions on a row → `Menu` from an `IconButton label={t('more')}`. Several selected → `BulkActionBar`.

## Grid vs table
Visual browsing of documents → `DocumentCard` grid. Comparing fields, sorting, bulk selection → `BoardTable`. Offer both with a `Segmented` (Grid | Table) when users need both.
