import { useEffect, useId, useRef, useState, type CSSProperties, type FormEvent, type KeyboardEvent } from 'react';
import { Label as RACLabel, Radio, RadioGroup } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import axios from 'axios';
import { Button, Dialog, TextField } from '../../ui';
import Label from './Label';
import {
  DEFAULT_LABEL_COLOR,
  LABEL_COLORS,
  LABEL_ICONS,
  isHexColor,
  swatchStyle,
  type LabelData,
  type LabelDraft,
} from './labelData';
import styles from './Labels.module.css';

export interface LabelCreateDialogProps {
  open: boolean;
  onClose: () => void;
  /** Saves the label. A rejection keeps the dialog open and shows the error under the name. */
  onSubmit: (labelData: LabelDraft) => Promise<void>;
  prefilledName?: string;
  /** When set, the dialog edits this label instead of creating one. */
  editingLabel?: LabelData;
}

interface FormState {
  name: string;
  description: string;
  color: string;
  background_color: string;
  icon: string;
}

const NO_ICON = 'none';

function initialState(editing: LabelData | undefined, prefilledName: string): FormState {
  if (editing) {
    return {
      name: editing.name,
      description: editing.description ?? '',
      color: editing.color,
      background_color: editing.background_color ?? '',
      icon: editing.icon ?? '',
    };
  }
  return { name: prefilledName, description: '', color: DEFAULT_LABEL_COLOR, background_color: '', icon: '' };
}

function errorText(error: unknown): string | null {
  if (axios.isAxiosError(error)) {
    const message = (error.response?.data as { error?: unknown } | undefined)?.error;
    if (typeof message === 'string' && message) return message;
  }
  if (error instanceof Error && error.message) return error.message;
  return null;
}

/** Create or edit a label: name, description, colour and an optional icon, with a live preview. */
function LabelCreateDialog({ open, onClose, onSubmit, prefilledName = '', editingLabel }: LabelCreateDialogProps) {
  const { t } = useTranslation();
  const formId = useId();
  const formRef = useRef<HTMLFormElement>(null);
  const [form, setForm] = useState<FormState>(() => initialState(editingLabel, prefilledName));
  const [saving, setSaving] = useState(false);
  const [nameError, setNameError] = useState('');

  useEffect(() => {
    setForm(initialState(editingLabel, prefilledName));
    setNameError('');
  }, [editingLabel, prefilledName, open]);

  const colorValid = isHexColor(form.color);
  const update = (patch: Partial<FormState>) => setForm((f) => ({ ...f, ...patch }));

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    const name = form.name.trim();
    if (!name) {
      setNameError(t('labels.create.nameRequired', 'Name is required'));
      return;
    }
    if (name.includes(',') || name.toLowerCase().includes('%2c')) {
      setNameError(t('labels.errors.commaNotAllowed', 'Label names cannot contain commas.'));
      return;
    }
    if (!colorValid) return;
    setSaving(true);
    try {
      await onSubmit({
        name,
        description: form.description.trim() || undefined,
        color: form.color.trim(),
        background_color: form.background_color || undefined,
        icon: form.icon || undefined,
      });
      onClose();
    } catch (error) {
      console.error('Failed to save label:', error);
      setNameError(errorText(error) ?? t('labels.errors.serverError', 'Server error. Please try again later.'));
    } finally {
      setSaving(false);
    }
  };

  // The submit button sits in the dialog footer (outside the form), so Enter in a single-line
  // field submits explicitly rather than relying on implicit submission.
  const submitOnEnter = (e: KeyboardEvent<HTMLFormElement>) => {
    if (e.key !== 'Enter' || e.nativeEvent.isComposing) return;
    if (!(e.target instanceof HTMLInputElement)) return;
    e.preventDefault();
    formRef.current?.requestSubmit();
  };

  const close = () => {
    if (!saving) onClose();
  };

  const preview: LabelData = {
    id: 'preview',
    name: form.name || t('labels.create.previewName', 'Label Preview'),
    description: form.description,
    color: colorValid ? form.color : DEFAULT_LABEL_COLOR,
    icon: form.icon || undefined,
    is_system: false,
    created_at: '',
    updated_at: '',
  };

  const isEditing = Boolean(editingLabel);
  const submitText = saving
    ? t('labels.create.saving', 'Saving...')
    : isEditing
      ? t('labels.create.update', 'Update')
      : t('labels.create.create', 'Create');

  return (
    <Dialog
      isOpen={open}
      onOpenChange={(next) => {
        if (!next) close();
      }}
      isDismissable={!saving}
      title={isEditing ? t('labels.create.editTitle', 'Edit Label') : t('labels.create.title', 'Create New Label')}
      actions={
        <>
          <Button variant="ghost" onPress={close} isDisabled={saving}>
            {t('labels.create.cancel', 'Cancel')}
          </Button>
          <Button
            variant="primary"
            type="submit"
            form={formId}
            isPending={saving}
            isDisabled={!form.name.trim() || !colorValid}
          >
            {submitText}
          </Button>
        </>
      }
    >
      <form ref={formRef} id={formId} className={styles.form} onSubmit={handleSubmit} onKeyDown={submitOnEnter} noValidate>
        <TextField
          label={t('labels.create.nameLabel', 'Label Name')}
          value={form.name}
          onChange={(name) => {
            update({ name });
            if (nameError) setNameError('');
          }}
          isRequired
          validationBehavior="aria"
          isInvalid={Boolean(nameError)}
          errorMessage={nameError}
          isDisabled={saving}
          autoFocus
        />
        <TextField
          label={t('labels.create.descriptionLabel', 'Description (optional)')}
          value={form.description}
          onChange={(description) => update({ description })}
          multiline
          rows={2}
          isDisabled={saving}
        />
        <RadioGroup
          className={styles.choiceGroup}
          value={LABEL_COLORS.some((c) => c.value === form.color) ? form.color : null}
          onChange={(color) => update({ color })}
          isDisabled={saving}
        >
          <RACLabel className={styles.fieldLabel}>{t('labels.create.colorLabel', 'Color')}</RACLabel>
          <div className={styles.choices}>
            {LABEL_COLORS.map((c) => (
              <Radio key={c.value} value={c.value} className={styles.colorChoice} aria-label={t(c.nameKey, c.fallback)}>
                <span className={styles.colorDot} style={swatchStyle(c.value) as CSSProperties} aria-hidden="true" />
              </Radio>
            ))}
          </div>
        </RadioGroup>
        <TextField
          label={t('labels.create.customColorLabel', 'Custom Color (hex)')}
          value={form.color}
          onChange={(color) => update({ color })}
          isDisabled={saving}
          validationBehavior="aria"
          isInvalid={!colorValid}
          errorMessage={t('labels.errors.invalidColor', 'Invalid color format. Please use a valid hex color like #0969da.')}
        />
        <RadioGroup
          className={styles.choiceGroup}
          value={form.icon || NO_ICON}
          onChange={(icon) => update({ icon: icon === NO_ICON ? '' : icon })}
          isDisabled={saving}
        >
          <RACLabel className={styles.fieldLabel}>{t('labels.create.iconLabel', 'Icon (optional)')}</RACLabel>
          <div className={styles.choices}>
            <Radio value={NO_ICON} className={styles.iconChoice}>
              {t('labels.create.iconNone', 'None')}
            </Radio>
            {LABEL_ICONS.map(({ key, icon: Icon, nameKey, fallback }) => (
              <Radio key={key} value={key} className={styles.iconChoice} aria-label={t(nameKey, fallback)}>
                <Icon fontSize="small" />
              </Radio>
            ))}
          </div>
        </RadioGroup>
        <div className={styles.preview} role="group" aria-label={t('labels.create.previewLabel', 'Preview')}>
          <span className={styles.fieldLabel} aria-hidden="true">
            {t('labels.create.previewLabel', 'Preview')}
          </span>
          <div className={styles.previewTags}>
            <Label label={preview} variant="filled" />
            <Label label={preview} variant="outlined" />
          </div>
        </div>
      </form>
    </Dialog>
  );
}

export { LabelCreateDialog };
export default LabelCreateDialog;
