import type { ComponentType } from 'react';
import type { LabelResponse } from '../../services/api/labels';
import {
  Archive,
  Assignment,
  AttachMoney,
  BugReport,
  Build,
  BusinessCenter,
  Description,
  Folder,
  Label as LabelIcon,
  LocalHospital,
  Person,
  Receipt,
  Scale,
  Schedule,
  Star,
  Work,
} from '../../ui/icons';

/** A label as the label components take it. */
export interface LabelData {
  id: string;
  name: string;
  description?: string;
  color: string;
  background_color?: string;
  icon?: string;
  is_system: boolean;
  created_at: string;
  updated_at: string;
  document_count?: number;
  source_count?: number;
}

/** Fields a create/edit form hands back. */
export type LabelDraft = Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>;

/** The API sends `null` for unset optional fields; the components take `undefined`. */
export function toLabelData(label: LabelResponse | LabelData): LabelData {
  return {
    id: label.id,
    name: label.name,
    description: label.description ?? undefined,
    color: label.color,
    background_color: label.background_color ?? undefined,
    icon: label.icon ?? undefined,
    is_system: label.is_system,
    created_at: label.created_at,
    updated_at: label.updated_at,
    document_count: label.document_count ?? undefined,
    source_count: label.source_count ?? undefined,
  };
}

type IconComponent = ComponentType<{ fontSize?: 'inherit' | 'small' }>;

/** Icons a label may carry. `key` is what the API stores; `nameKey` names it for people. */
export const LABEL_ICONS: { key: string; icon: IconComponent; nameKey: string; fallback: string }[] = [
  { key: 'star', icon: Star, nameKey: 'labels.icons.star', fallback: 'Star' },
  { key: 'archive', icon: Archive, nameKey: 'labels.icons.archive', fallback: 'Archive' },
  { key: 'person', icon: Person, nameKey: 'labels.icons.person', fallback: 'Person' },
  { key: 'work', icon: Work, nameKey: 'labels.icons.work', fallback: 'Work' },
  { key: 'briefcase', icon: BusinessCenter, nameKey: 'labels.icons.briefcase', fallback: 'Briefcase' },
  { key: 'receipt', icon: Receipt, nameKey: 'labels.icons.receipt', fallback: 'Receipt' },
  { key: 'scale', icon: Scale, nameKey: 'labels.icons.scale', fallback: 'Legal' },
  { key: 'medical', icon: LocalHospital, nameKey: 'labels.icons.medical', fallback: 'Medical' },
  { key: 'dollar', icon: AttachMoney, nameKey: 'labels.icons.dollar', fallback: 'Money' },
  { key: 'document', icon: Description, nameKey: 'labels.icons.document', fallback: 'Document' },
  { key: 'label', icon: LabelIcon, nameKey: 'labels.icons.label', fallback: 'Label' },
  { key: 'bug', icon: BugReport, nameKey: 'labels.icons.bug', fallback: 'Bug' },
  { key: 'build', icon: Build, nameKey: 'labels.icons.build', fallback: 'Build' },
  { key: 'folder', icon: Folder, nameKey: 'labels.icons.folder', fallback: 'Folder' },
  { key: 'assignment', icon: Assignment, nameKey: 'labels.icons.assignment', fallback: 'Assignment' },
  { key: 'schedule', icon: Schedule, nameKey: 'labels.icons.schedule', fallback: 'Schedule' },
];

const ICON_BY_KEY: Record<string, IconComponent> = Object.fromEntries(LABEL_ICONS.map((i) => [i.key, i.icon]));
// Older labels stored `user` for the person icon.
ICON_BY_KEY.user = Person;

export function labelIcon(key: string | undefined): IconComponent | null {
  return key ? ICON_BY_KEY[key] ?? null : null;
}

/**
 * Preset label colours. Label colours are user data stored on the label (any hex value is
 * allowed), so these are values offered to pick from, not theme colours.
 */
export const LABEL_COLORS: { value: string; nameKey: string; fallback: string }[] = [
  { value: '#0969da', nameKey: 'labels.colors.blue', fallback: 'Blue' },
  { value: '#d73a49', nameKey: 'labels.colors.red', fallback: 'Red' },
  { value: '#28a745', nameKey: 'labels.colors.green', fallback: 'Green' },
  { value: '#ffd33d', nameKey: 'labels.colors.yellow', fallback: 'Yellow' },
  { value: '#8250df', nameKey: 'labels.colors.purple', fallback: 'Purple' },
  { value: '#fd7e14', nameKey: 'labels.colors.orange', fallback: 'Orange' },
  { value: '#20c997', nameKey: 'labels.colors.teal', fallback: 'Teal' },
  { value: '#6f42c1', nameKey: 'labels.colors.indigo', fallback: 'Indigo' },
  { value: '#e83e8c', nameKey: 'labels.colors.pink', fallback: 'Pink' },
  { value: '#6c757d', nameKey: 'labels.colors.gray', fallback: 'Gray' },
];

export const DEFAULT_LABEL_COLOR = LABEL_COLORS[0].value;

const HEX = /^#(?:[0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/;

export function isHexColor(value: string): boolean {
  return HEX.test(value.trim());
}

/** Label colour as a CSS custom property, or nothing when the stored value is not a hex colour. */
export function swatchStyle(color: string | undefined): Record<string, string> | undefined {
  return color && isHexColor(color) ? { '--label-color': color.trim() } : undefined;
}
