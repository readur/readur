import { useId, useMemo, useState, type Key } from 'react';
import {
  Button as RACButton,
  ComboBox as RACComboBox,
  Header,
  Input,
  Label as RACLabel,
  ListBox,
  ListBoxItem,
  ListBoxSection,
  Popover,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Add, ExpandMore } from '../../ui/icons';
import Label from './Label';
import LabelCreateDialog from './LabelCreateDialog';
import type { LabelData, LabelDraft } from './labelData';
import styles from './Labels.module.css';

export interface LabelSelectorProps {
  selectedLabels: LabelData[];
  availableLabels: LabelData[];
  onLabelsChange: (labels: LabelData[]) => void;
  /** Enables "Create label" for text that matches no label. Rejecting keeps the dialog open with the error. */
  onCreateLabel?: (labelData: LabelDraft) => Promise<LabelData>;
  placeholder?: string;
  size?: 'small' | 'medium';
  disabled?: boolean;
  /** `false` keeps at most one label: picking another replaces it. */
  multiple?: boolean;
  showCreateButton?: boolean;
  /** Upper bound on selected labels; further picks are ignored. */
  maxTags?: number;
  /** Visible field label. Without it the field is named "Labels". */
  label?: string;
}

const CREATE_KEY = '__create__';

const matchesQuery = (label: LabelData, query: string) => {
  const q = query.trim().toLocaleLowerCase();
  if (!q) return true;
  return (
    label.name.toLocaleLowerCase().includes(q) ||
    (label.description ?? '').toLocaleLowerCase().includes(q)
  );
};

/** Multi-select label picker: selected labels as removable tags, then a searchable list. */
function LabelSelector({
  selectedLabels,
  availableLabels,
  onLabelsChange,
  onCreateLabel,
  placeholder,
  size = 'medium',
  disabled = false,
  multiple = true,
  showCreateButton = true,
  maxTags,
  label,
}: LabelSelectorProps) {
  const { t } = useTranslation();
  const tagsId = useId();
  const [inputValue, setInputValue] = useState('');
  const [createOpen, setCreateOpen] = useState(false);
  const [prefilledName, setPrefilledName] = useState('');

  const selectedIds = useMemo(() => new Set(selectedLabels.map((l) => l.id)), [selectedLabels]);
  const options = useMemo(
    () => availableLabels.filter((l) => !selectedIds.has(l.id) && matchesQuery(l, inputValue)),
    [availableLabels, selectedIds, inputValue],
  );
  const systemOptions = options.filter((l) => l.is_system);
  const userOptions = options.filter((l) => !l.is_system);

  const trimmed = inputValue.trim();
  const canCreate =
    Boolean(onCreateLabel) &&
    showCreateButton &&
    !disabled &&
    trimmed !== '' &&
    !availableLabels.some((l) => l.name.toLocaleLowerCase() === trimmed.toLocaleLowerCase());

  const add = (next: LabelData) => {
    if (!multiple) {
      onLabelsChange([next]);
      return;
    }
    if (maxTags !== undefined && selectedLabels.length >= maxTags) return;
    onLabelsChange([...selectedLabels, next]);
  };

  const onSelect = (key: Key | null) => {
    if (key === null) return;
    if (key === CREATE_KEY) {
      setPrefilledName(trimmed);
      setCreateOpen(true);
      return;
    }
    const picked = availableLabels.find((l) => l.id === key);
    if (picked) add(picked);
    setInputValue('');
  };

  const handleCreate = async (draft: LabelDraft) => {
    if (!onCreateLabel) return;
    try {
      const created = await onCreateLabel(draft);
      onLabelsChange(multiple ? [...selectedLabels, created] : [created]);
      setInputValue('');
      setPrefilledName('');
    } catch (error) {
      console.error('Failed to create label:', error);
      // Rethrow so the dialog stays open and shows the reason.
      throw error;
    }
  };

  const createText = t('labels.selector.createLabel', { name: trimmed, defaultValue: 'Create label "{{name}}"' });
  const fieldName = label ?? t('labels.selector.label', 'Labels');
  const emptyText = trimmed
    ? t('labels.selector.noLabelsMatch', { query: trimmed, defaultValue: 'No labels match "{{query}}"' })
    : t('labels.selector.noLabelsAvailable', 'No labels available');

  const renderOption = (l: LabelData) => (
    <ListBoxItem key={l.id} id={l.id} textValue={l.name} className={styles.option}>
      <Label label={l} size="small" showCount variant="outlined" />
      {l.description ? <span className={styles.optionDescription}>{l.description}</span> : null}
    </ListBoxItem>
  );

  return (
    <div className={styles.selector} data-size={size}>
      {selectedLabels.length > 0 ? (
        <ul className={styles.selectedTags} id={tagsId} aria-label={t('labels.selector.selected', 'Selected labels')}>
          {selectedLabels.map((l) => (
            <li key={l.id}>
              <Label
                label={l}
                size="small"
                deletable={!disabled}
                onDelete={(id) => onLabelsChange(selectedLabels.filter((s) => s.id !== id))}
              />
            </li>
          ))}
        </ul>
      ) : null}
      <RACComboBox
        className={styles.combo}
        aria-label={label ? undefined : fieldName}
        inputValue={inputValue}
        onInputChange={setInputValue}
        selectedKey={null}
        onSelectionChange={onSelect}
        defaultFilter={() => true}
        menuTrigger="focus"
        allowsEmptyCollection
        isDisabled={disabled}
      >
        {label ? <RACLabel className={styles.fieldLabel}>{label}</RACLabel> : null}
        <div className={styles.comboGroup}>
          <Input
            className={styles.comboInput}
            placeholder={
              selectedLabels.length === 0
                ? placeholder ?? t('labels.selector.placeholder', 'Search or create labels...')
                : undefined
            }
          />
          <RACButton className={styles.comboButton} aria-label={t('ui.showOptions', 'Show options')}>
            <ExpandMore fontSize="small" />
          </RACButton>
        </div>
        <Popover className={styles.popover} placement="bottom start">
          <ListBox className={styles.listbox} renderEmptyState={() => <p className={styles.emptyOptions}>{emptyText}</p>}>
            {systemOptions.length > 0 ? (
              <ListBoxSection className={styles.section}>
                <Header className={styles.sectionHeader}>{t('labels.selector.systemLabels', 'System Labels')}</Header>
                {systemOptions.map(renderOption)}
              </ListBoxSection>
            ) : null}
            {userOptions.length > 0 ? (
              <ListBoxSection className={styles.section}>
                <Header className={styles.sectionHeader}>{t('labels.selector.myLabels', 'My Labels')}</Header>
                {userOptions.map(renderOption)}
              </ListBoxSection>
            ) : null}
            {canCreate ? (
              <ListBoxItem id={CREATE_KEY} textValue={createText} className={`${styles.option} ${styles.createOption}`}>
                <span className={styles.tagIcon} aria-hidden="true">
                  <Add fontSize="inherit" />
                </span>
                {createText}
              </ListBoxItem>
            ) : null}
          </ListBox>
        </Popover>
      </RACComboBox>
      {onCreateLabel ? (
        <LabelCreateDialog
          open={createOpen}
          onClose={() => {
            setCreateOpen(false);
            setPrefilledName('');
          }}
          onSubmit={handleCreate}
          prefilledName={prefilledName}
        />
      ) : null}
    </div>
  );
}

export { LabelSelector };
export default LabelSelector;
