import { useState, type ReactNode } from 'react';
import { useThemeMode } from '../../theme/useThemeMode';
import {
  Avatar,
  BoardTable,
  BulkActionBar,
  Button,
  Card,
  ChangeTag,
  Checkbox,
  ChoiceGroup,
  ChoiceTile,
  ComboBox,
  ComboBoxItem,
  CommandPalette,
  Dialog,
  DocumentCard,
  EmptyState,
  Facts,
  FilterChip,
  IconButton,
  Kbd,
  LabelChip,
  Menu,
  MenuItem,
  MenuSection,
  MenuTrigger,
  Notice,
  OutcomeBar,
  Pagination,
  Pass,
  PassCell,
  Popover,
  PopoverTrigger,
  ProgressBar,
  SearchField,
  Segmented,
  Select,
  SelectItem,
  Skeleton,
  SlideOver,
  SourceBadge,
  Spinner,
  STATUS_STATES,
  StatusMark,
  Switch,
  Tab,
  TabList,
  TabPanel,
  Tabs,
  TextField,
  useToast,
  type BoardColumn,
  type CommandSource,
} from '../../ui';
import { Delete, Download, GridView, MoreVert, Refresh, Share, Upload, ViewList, Visibility } from '../../ui/icons';
import styles from './Gallery.module.css';

/**
 * Every primitive in every state, for checking the Studio look in light and dark
 * against the design mockups. Development builds only (`/dev/ui`).
 */

const LABELS = [
  { name: 'Taxes', color: '#C0841A' },
  { name: 'Housing', color: '#3E7CB1' },
  { name: 'Medical', color: '#9B4F96' },
  { name: 'Receipts', color: '#4E8A3E' },
];

interface Row {
  id: string;
  name: string;
  source: ReactNode;
  status: ReactNode;
  added: string;
}

const ROWS: Row[] = [
  { id: '1', name: 'Invoice_ACME_2026-09.pdf', source: <SourceBadge sourceId="nc" kind="webdav" name="Nextcloud" />, status: <StatusMark state="completed" />, added: '2d ago' },
  { id: '2', name: 'Lease_Agreement_Signed.pdf', source: <SourceBadge sourceId="watch" kind="watch" name="Scanner inbox" />, status: <StatusMark state="processing" progress={{ current: 3, total: 14 }} />, added: 'just now' },
  { id: '3', name: 'scan_0042.tiff', source: <SourceBadge sourceId={null} name="Upload" />, status: <StatusMark state="failed" />, added: '5h ago' },
  { id: '4', name: 'Lab_results_Mar.pdf', source: <SourceBadge sourceId="s3" kind="s3" name="Backups bucket" />, status: <StatusMark state="warning" />, added: 'Mar 14' },
];

const COLUMNS: BoardColumn<Row>[] = [
  { id: 'name', label: 'Name', isRowHeader: true, sortable: true, render: (r) => r.name },
  { id: 'source', label: 'Source', render: (r) => r.source },
  { id: 'status', label: 'Status', render: (r) => r.status },
  { id: 'added', label: 'Added', mono: true, render: (r) => r.added },
];

const PALETTE_SOURCES: CommandSource[] = [
  {
    id: 'documents',
    label: 'Documents',
    search: async (q) =>
      ['Lease_Agreement_Signed.pdf', 'Car_lease_2024.pdf']
        .filter((t) => t.toLowerCase().includes(q.toLowerCase()))
        .map((title) => ({
          id: title,
          title,
          subtitle: 'Housing',
          preview: <p>“…the tenant agrees to pay the monthly lease amount on or before the first day…”</p>,
          onSelect: () => {},
        })),
  },
];

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className={styles.section} aria-labelledby={`g-${title}`}>
      <h2 id={`g-${title}`} className={styles.h2}>
        {title}
      </h2>
      {children}
    </section>
  );
}

function Row({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className={styles.row}>
      <span className={styles.rowLabel}>{label}</span>
      <div className={styles.rowBody}>{children}</div>
    </div>
  );
}

function Thumb() {
  return (
    <span className={styles.thumb} aria-hidden="true">
      <i />
      <i />
      <i />
      <i />
      <i />
    </span>
  );
}

export default function Gallery() {
  const { mode, toggle } = useThemeMode();
  const toast = useToast();
  const [view, setView] = useState<'grid' | 'table'>('grid');
  const [page, setPage] = useState(2);
  const [selected, setSelected] = useState<Set<string>>(new Set(['1']));
  const [dialogOpen, setDialogOpen] = useState(false);
  const [slideOpen, setSlideOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [bulk, setBulk] = useState(false);

  return (
    <main className={styles.page}>
      <header className={styles.header}>
        <h1 className={styles.h1}>Studio primitives</h1>
        <Button variant="secondary" onPress={toggle}>
          {mode === 'dark' ? 'Switch to light' : 'Switch to dark'}
        </Button>
      </header>

      <Section title="Actions">
        <Row label="Variants">
          <Button variant="primary" icon={<Upload fontSize="inherit" />}>Upload</Button>
          <Button variant="secondary">Export</Button>
          <Button variant="ghost">Cancel</Button>
          <Button variant="danger" icon={<Delete fontSize="inherit" />}>Delete</Button>
          <Button variant="danger-solid">Delete forever</Button>
        </Row>
        <Row label="States">
          <Button variant="primary" isPending>Processing</Button>
          <Button variant="primary" isDisabled>Disabled</Button>
          <Button variant="primary" size="sm" icon={<Refresh fontSize="inherit" />}>Retry OCR</Button>
          <Button variant="secondary" size="sm">Add label</Button>
        </Row>
        <Row label="Icon buttons">
          <IconButton label="Retry OCR" icon={<Refresh fontSize="small" />} />
          <IconButton label="Download" icon={<Download fontSize="small" />} />
          <IconButton label="Share" icon={<Share fontSize="small" />} />
          <IconButton label="Preview" icon={<Visibility fontSize="small" />} />
          <IconButton label="Delete" variant="danger" icon={<Delete fontSize="small" />} />
          <IconButton label="More" size="sm" icon={<MoreVert fontSize="small" />} />
        </Row>
        <Row label="People & loading">
          <Avatar name="Alex Lindqvist" />
          <Avatar name="sam" size="sm" />
          <Spinner label="Loading" />
        </Row>
      </Section>

      <Section title="Inputs">
        <div className={styles.grid2}>
          <TextField label="Document title" defaultValue="Lease Agreement 2026" description="Shown in the library and search results" />
          <TextField label="Port" defaultValue="80800" isInvalid errorMessage="Must be between 1 and 65535" />
          <SearchField aria-label="Search documents" placeholder="Search documents, labels, text…" />
          <Select label="OCR language" defaultSelectedKey="en">
            <SelectItem id="en">English</SelectItem>
            <SelectItem id="de">German</SelectItem>
            <SelectItem id="fr">French</SelectItem>
          </Select>
          <ComboBox label="Source" placeholder="Pick a source">
            <ComboBoxItem id="nc">Nextcloud</ComboBoxItem>
            <ComboBoxItem id="s3">Backups bucket</ComboBoxItem>
          </ComboBox>
          <div className={styles.stack}>
            <Checkbox defaultSelected label="Run OCR on upload" />
            <Checkbox isIndeterminate label="Apply to all sources" />
            <Switch defaultSelected label="Watch folder for new files" />
            <Switch label="Email me when OCR fails" />
          </div>
        </div>
        <ChoiceGroup label="OCR options" defaultValue={['ocr']}>
          <ChoiceTile value="ocr" label="Run OCR on upload" description="Extract text as soon as a file arrives" />
          <ChoiceTile value="rotate" label="Auto-rotate pages" description="Fix sideways scans before OCR" />
        </ChoiceGroup>
        <Tabs>
          <TabList aria-label="Document sections">
            <Tab id="overview">Overview</Tab>
            <Tab id="text">Text</Tab>
            <Tab id="history">History</Tab>
          </TabList>
          <TabPanel id="overview">Overview panel</TabPanel>
          <TabPanel id="text">Text panel</TabPanel>
          <TabPanel id="history">History panel</TabPanel>
        </Tabs>
        <Segmented<'grid' | 'table'>
          label="Layout"
          value={view}
          onChange={setView}
          iconOnly
          items={[
            { id: 'grid', label: 'Grid', icon: <GridView fontSize="inherit" /> },
            { id: 'table', label: 'Table', icon: <ViewList fontSize="inherit" /> },
          ]}
        />
      </Section>

      <Section title="Status">
        <Row label="Status marks">
          {STATUS_STATES.map((s) => (
            <StatusMark key={s} state={s} />
          ))}
          <StatusMark state="processing" progress={{ current: 3, total: 12 }} />
        </Row>
        <Row label="Labels">
          {LABELS.map((l) => (
            <LabelChip key={l.name} name={l.name} color={l.color} />
          ))}
          <LabelChip name="Insurance" color="#C2506B" onRemove={() => {}} removeLabel="Remove Insurance" />
        </Row>
        <Row label="Filters">
          <FilterChip label="All" isActive={false} onPress={() => {}} />
          <FilterChip label="Needs review" value="18" isActive onPress={() => {}} onClear={() => {}} />
          <FilterChip label="Failed OCR" value="3" isActive={false} onPress={() => {}} />
        </Row>
        <Row label="Sources">
          <SourceBadge sourceId="nc" kind="webdav" type="webdav" name="Nextcloud" showType />
          <SourceBadge sourceId="s3" kind="s3" type="s3" name="Backups bucket" showType />
          <SourceBadge sourceId="watch" kind="watch" name="Scanner inbox" variant="chip" />
        </Row>
        <Row label="Marks">
          <ChangeTag>New</ChangeTag>
          <Kbd>⌘K</Kbd>
        </Row>
        <div className={styles.stack}>
          <Notice tone="info" title="Watch folder active">New files in /mnt/scans are imported every 5 minutes.</Notice>
          <Notice tone="ok" title="Sync complete" action={<Button variant="ghost" size="sm">View</Button>}>
            412 files checked, 6 new documents imported.
          </Notice>
          <Notice tone="warning" title="Low OCR confidence">3 documents scored below 60%.</Notice>
          <Notice tone="danger" title="Can’t reach dav.home.lan" onDismiss={() => {}}>
            Connection refused on port 443.
          </Notice>
        </div>
        <Row label="Toasts">
          <Button onPress={() => toast.show({ title: '3 documents uploaded', description: 'Queued for OCR', tone: 'success' })}>Success</Button>
          <Button onPress={() => toast.show({ title: 'OCR failed', description: 'scan_0042.tiff', tone: 'danger' })}>Danger</Button>
          <Button onPress={() => toast.show({ title: 'Syncing Nextcloud', tone: 'info' })}>Info</Button>
        </Row>
      </Section>

      <Section title="Data">
        <ul className={styles.cards}>
          {ROWS.map((r) => (
            <DocumentCard
              key={r.id}
              title={r.name}
              thumbnail={<Thumb />}
              onOpen={() => {}}
              meta={`PDF · 3 pp · ${r.added}`}
              status={r.status}
              labels={<LabelChip name={LABELS[Number(r.id) - 1].name} color={LABELS[Number(r.id) - 1].color} size="small" />}
              source={r.source}
              selectLabel={`Select ${r.name}`}
              isSelected={selected.has(r.id)}
              onSelectionChange={(on) =>
                setSelected((prev) => {
                  const next = new Set(prev);
                  if (on) next.add(r.id);
                  else next.delete(r.id);
                  return next;
                })
              }
              isChanged={r.id === '2'}
              quickActions={<IconButton label="Download" size="sm" icon={<Download fontSize="small" />} />}
            />
          ))}
        </ul>
        <BoardTable
          aria-label="Documents"
          columns={COLUMNS}
          rows={ROWS}
          getRowId={(r) => r.id}
          selectionMode="multiple"
          selectedKeys={selected}
          onSelectionChange={(keys) => setSelected(keys === 'all' ? new Set(ROWS.map((r) => r.id)) : new Set([...keys].map(String)))}
        />
        <div className={styles.grid2}>
          <Facts
            title="Details"
            items={[
              { label: 'Pages', value: '14', mono: true },
              { label: 'Language', value: 'English' },
              { label: 'Added', value: 'Oct 7, 2026 · 09:14', mono: true },
            ]}
          />
          <Pass aria-label="Document facts">
            <PassCell label="Pages" mono>14</PassCell>
            <PassCell label="Size" mono>2.1 MB</PassCell>
            <PassCell label="Language">English</PassCell>
          </Pass>
        </div>
        <Card>
          <OutcomeBar
            label="Processing pipeline"
            segments={[
              { id: 'done', label: 'Done', value: 214, tone: 'ok' },
              { id: 'processing', label: 'Processing', value: 24, tone: 'accent' },
              { id: 'failed', label: 'Failed', value: 2, tone: 'danger' },
              { id: 'low', label: 'Low confidence', value: 12, tone: 'warn' },
              { id: 'queued', label: 'Queued', value: 160, tone: 'neutral' },
            ]}
          />
          <ProgressBar value={58} label="Files progress" showValue />
        </Card>
        <Card>
          <EmptyState
            illustration
            title="No documents yet"
            description="Upload files or connect a WebDAV, S3 or local folder source."
            action={<Button variant="primary">Upload files</Button>}
          />
        </Card>
        <Skeleton lines={3} />
        <Pagination page={page} pageSize={50} total={1284} onChange={(p) => setPage(p)} pageSizeOptions={[25, 50, 100]} />
      </Section>

      <Section title="Overlays">
        <Row label="Open">
          <Button onPress={() => setDialogOpen(true)}>Dialog</Button>
          <MenuTrigger>
            <Button>Menu</Button>
            <Menu aria-label="Document actions">
              <MenuSection title="Document">
                <MenuItem>Open</MenuItem>
                <MenuItem>Download</MenuItem>
              </MenuSection>
              <MenuSection title="OCR">
                <MenuItem>Retry OCR</MenuItem>
                <MenuItem danger>Delete</MenuItem>
              </MenuSection>
            </Menu>
          </MenuTrigger>
          <PopoverTrigger>
            <Button>Popover</Button>
            <Popover>Details here</Popover>
          </PopoverTrigger>
          <Button onPress={() => setSlideOpen(true)}>Slide-over</Button>
          <Button onPress={() => setPaletteOpen(true)}>Command palette</Button>
          <Button onPress={() => setBulk((b) => !b)}>Bulk bar</Button>
        </Row>
        <Dialog
          isOpen={dialogOpen}
          onOpenChange={setDialogOpen}
          title="Retry OCR for 3 documents"
          icon={<Refresh fontSize="small" />}
          helpLink={<a href="#help">What does this do?</a>}
          actions={
            <>
              <Button variant="ghost" onPress={() => setDialogOpen(false)}>Cancel</Button>
              <Button variant="primary" onPress={() => setDialogOpen(false)}>Retry OCR</Button>
            </>
          }
        >
          They’ll be re-queued. Existing text is kept until the new run succeeds.
        </Dialog>
        <SlideOver isOpen={slideOpen} onOpenChange={setSlideOpen} title="Lease_Agreement_Signed.pdf">
          <Facts items={[{ label: 'Status', value: <StatusMark state="completed" /> }, { label: 'Pages', value: '14', mono: true }]} />
        </SlideOver>
        <CommandPalette isOpen={paletteOpen} onOpenChange={setPaletteOpen} sources={PALETTE_SOURCES} />
        {bulk ? (
          <BulkActionBar
            count={3}
            onClear={() => setBulk(false)}
            actions={[
              { id: 'label', label: 'Label', onPress: () => {} },
              { id: 'retry', label: 'Retry OCR', onPress: () => {} },
              { id: 'delete', label: 'Delete', tone: 'danger', onPress: () => {} },
            ]}
          />
        ) : null}
      </Section>
    </main>
  );
}
