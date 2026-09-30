import { useEffect, useId, useState, type CSSProperties, type FormEvent } from 'react';
import { Radio, RadioGroup } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import axios from 'axios';
import { labelService } from '../../../services/api/labels';
import { Button, Dialog, Select, SelectItem, TextField, useToast } from '../../../ui';
import { ColorChoices, LABEL_COLORS, notifyLabelsChanged, toLabelData, type LabelData } from '../../labels';
import { swatchStyle } from '../../labels/labelData';
import styles from './Search.module.css';

type Target = 'new' | 'existing';

export interface SaveCollectionDialogProps {
  isOpen: boolean;
  onOpenChange: (open: boolean) => void;
  /** How many documents will be saved. */
  count: number;
  labels: LabelData[];
  /** Offered as the new collection's name (usually the search text). */
  suggestedName: string;
  /** The ids to save; may fetch (for "all matches"). */
  resolveIds: () => Promise<string[]>;
  /** After the documents are in the collection (new or existing). */
  onSaved: (collection: LabelData) => void;
}

/** "shoulder & injury" → "Shoulder injury": the search text without operators, capitalised. */
export function nameFromQuery(q: string): string {
  const plain = q.replace(/[&|!"()*,]/g, ' ').replace(/\s+/g, ' ').trim();
  return plain ? plain.charAt(0).toLocaleUpperCase() + plain.slice(1) : '';
}

/** The first preset colour no collection uses yet, so a new one stands out in the sidebar. */
function freshColor(labels: readonly LabelData[]): string {
  const used = new Set(labels.map((l) => l.color.toLowerCase()));
  return (LABEL_COLORS.find((c) => !used.has(c.value.toLowerCase())) ?? LABEL_COLORS[0]).value;
}

function serverMessage(error: unknown): string | null {
  if (axios.isAxiosError(error)) {
    const message = (error.response?.data as { error?: unknown } | undefined)?.error;
    if (typeof message === 'string' && message) return message;
  }
  return null;
}

/**
 * Put the chosen documents into a collection (a label): a new one named and coloured here, or
 * one that exists. The sidebar hears about it through `notifyLabelsChanged`.
 */
export function SaveCollectionDialog({ isOpen, onOpenChange, count, labels, suggestedName, resolveIds, onSaved }: SaveCollectionDialogProps) {
  const { t } = useTranslation();
  const toast = useToast();
  const formId = useId();
  const [target, setTarget] = useState<Target>('new');
  const [name, setName] = useState('');
  const [color, setColor] = useState(LABEL_COLORS[0].value);
  const [existingId, setExistingId] = useState<string | null>(null);
  const [nameError, setNameError] = useState('');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!isOpen) return;
    setTarget('new');
    setName(nameFromQuery(suggestedName));
    setColor(freshColor(labels));
    setExistingId(null);
    setNameError('');
    // Reset only when the dialog opens.
  }, [isOpen]);

  const close = () => {
    if (!saving) onOpenChange(false);
  };

  const validName = (): string | null => {
    const trimmed = name.trim();
    if (!trimmed) return t('library.collection.nameRequired', 'Give the collection a name');
    if (trimmed.includes(',')) return t('labels.errors.commaNotAllowed', 'Label names cannot contain commas.');
    if (labels.some((l) => l.name.toLocaleLowerCase() === trimmed.toLocaleLowerCase())) {
      return t('library.collection.nameTaken', 'A collection with this name exists. Pick it under “An existing collection”.');
    }
    return null;
  };

  const save = async (e?: FormEvent) => {
    e?.preventDefault();
    if (target === 'new') {
      const problem = validName();
      if (problem) {
        setNameError(problem);
        return;
      }
    } else if (!existingId) {
      return;
    }
    setSaving(true);
    let collection: LabelData | null = target === 'existing' ? labels.find((l) => l.id === existingId) ?? null : null;
    try {
      const ids = await resolveIds();
      if (!collection) {
        const res = await labelService.create({ name: name.trim(), color });
        collection = toLabelData(res.data);
      }
      await labelService.bulkAssign(ids, [collection.id], 'add');
      notifyLabelsChanged();
      toast.show({
        title: t('library.collection.saved', {
          count: ids.length,
          name: collection.name,
          defaultValue: '{{count}} documents saved to “{{name}}”',
          defaultValue_one: '1 document saved to “{{name}}”',
        }),
        tone: 'success',
      });
      onSaved(collection);
      onOpenChange(false);
    } catch (error) {
      if (collection && target === 'new') {
        // The collection exists now; offer it so trying again does not make a second one.
        notifyLabelsChanged();
        setTarget('existing');
        setExistingId(collection.id);
        toast.show({ title: t('library.collection.assignFailed', 'The collection was made, but the documents could not be added. Try again.'), tone: 'danger' });
      } else if (target === 'new') {
        setNameError(serverMessage(error) ?? t('library.collection.createFailed', 'The collection could not be made'));
      } else {
        toast.show({ title: t('library.collection.assignFailedExisting', 'The documents could not be added'), tone: 'danger' });
      }
    } finally {
      setSaving(false);
    }
  };

  const canSave = target === 'new' ? name.trim().length > 0 : existingId !== null;

  return (
    <Dialog
      isOpen={isOpen}
      onOpenChange={(next) => (next ? undefined : close())}
      isDismissable={!saving}
      title={t('library.collection.title', { count, defaultValue: 'Save {{count}} documents as a collection', defaultValue_one: 'Save 1 document as a collection' })}
      actions={
        <>
          <Button variant="ghost" onPress={close} isDisabled={saving}>
            {t('library.cancel', 'Cancel')}
          </Button>
          <Button variant="primary" type="submit" form={formId} isPending={saving} isDisabled={!canSave}>
            {t('library.collection.save', 'Save')}
          </Button>
        </>
      }
    >
      <form
        id={formId}
        className={styles.saveForm}
        onSubmit={(e) => void save(e)}
        onKeyDown={(e) => {
          // The Save button sits in the dialog footer, outside the form, so Enter submits here.
          if (e.key !== 'Enter' || e.nativeEvent.isComposing || !(e.target instanceof HTMLInputElement)) return;
          e.preventDefault();
          if (canSave && !saving) void save();
        }}
        noValidate
      >
        <RadioGroup
          className={styles.targets}
          aria-label={t('library.collection.target', 'Save to')}
          value={target}
          onChange={(v) => setTarget(v as Target)}
          isDisabled={saving}
        >
          <Radio value="new" className={styles.target}>
            {t('library.collection.new', 'A new collection')}
          </Radio>
          <Radio value="existing" className={styles.target} isDisabled={labels.length === 0}>
            {t('library.collection.existing', 'An existing collection')}
          </Radio>
        </RadioGroup>
        {target === 'new' ? (
          <>
            <TextField
              label={t('library.collection.name', 'Name')}
              value={name}
              onChange={(v) => {
                setName(v);
                if (nameError) setNameError('');
              }}
              isRequired
              validationBehavior="aria"
              isInvalid={Boolean(nameError)}
              errorMessage={nameError}
              isDisabled={saving}
              autoFocus
            />
            <ColorChoices label={t('library.collection.color', 'Colour')} value={color} onChange={setColor} isDisabled={saving} />
          </>
        ) : (
          <Select
            label={t('library.collection.pick', 'Collection')}
            placeholder={t('library.collection.pickPlaceholder', 'Choose a collection')}
            items={labels}
            selectedKey={existingId}
            onSelectionChange={(key) => setExistingId(key === null ? null : String(key))}
            isDisabled={saving}
          >
            {(l) => (
              <SelectItem id={l.id} textValue={l.name}>
                <span className={styles.option}>
                  <span className={styles.optionSwatch} style={swatchStyle(l.color) as CSSProperties} aria-hidden="true" />
                  {l.name}
                </span>
              </SelectItem>
            )}
          </Select>
        )}
      </form>
    </Dialog>
  );
}
