/** Friendly file-type groups for the Type filter. */
export const TYPE_GROUPS = ['pdf', 'image', 'office', 'text', 'other'] as const;
export type TypeGroup = (typeof TYPE_GROUPS)[number];

export const TYPE_GROUP_LABELS: Record<TypeGroup, { key: string; fallback: string }> = {
  pdf: { key: 'library.type.pdf', fallback: 'PDF' },
  image: { key: 'library.type.image', fallback: 'Images' },
  office: { key: 'library.type.office', fallback: 'Office' },
  text: { key: 'library.type.text', fallback: 'Text' },
  other: { key: 'library.type.other', fallback: 'Other' },
};

const OFFICE_PREFIXES = [
  'application/msword',
  'application/vnd.openxmlformats-officedocument.',
  'application/vnd.ms-',
  'application/vnd.oasis.opendocument.',
  'application/rtf',
];

export function groupOf(mime: string): TypeGroup {
  const m = mime.toLowerCase();
  if (m === 'application/pdf') return 'pdf';
  if (m.startsWith('image/')) return 'image';
  if (OFFICE_PREFIXES.some((p) => m.startsWith(p))) return 'office';
  if (m.startsWith('text/') || m === 'application/json' || m === 'application/xml') return 'text';
  return 'other';
}

/** Used when the facet list is unavailable, so a Type filter still narrows the list. */
const FALLBACK_MIMES: Record<TypeGroup, string[]> = {
  pdf: ['application/pdf'],
  image: ['image/jpeg', 'image/png', 'image/tiff', 'image/gif', 'image/webp', 'image/bmp'],
  office: [
    'application/msword',
    'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
    'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet',
    'application/vnd.openxmlformats-officedocument.presentationml.presentation',
    'application/rtf',
  ],
  text: ['text/plain', 'text/csv', 'text/markdown', 'text/html'],
  other: [],
};

/** Matches no document; sent when the chosen groups hold no known type. */
export const NO_MIME_MATCH = 'application/x-no-match';

/**
 * The MIME types to filter on for the chosen groups. `known` is every MIME type in the library
 * (from the search facets); without it a built-in list stands in.
 */
export function mimeTypesFor(groups: readonly TypeGroup[], known: readonly string[] | null): string[] {
  if (groups.length === 0) return [];
  const source = known && known.length > 0 ? known : Object.values(FALLBACK_MIMES).flat();
  const result = source.filter((m) => groups.includes(groupOf(m)));
  return result.length > 0 ? result : [NO_MIME_MATCH];
}

const SHORT: Record<string, string> = {
  'application/pdf': 'PDF',
  'image/jpeg': 'JPEG',
  'image/png': 'PNG',
  'image/tiff': 'TIFF',
  'image/gif': 'GIF',
  'image/webp': 'WEBP',
  'image/bmp': 'BMP',
  'image/svg+xml': 'SVG',
  'text/plain': 'TXT',
  'text/csv': 'CSV',
  'text/markdown': 'MD',
  'text/html': 'HTML',
  'application/json': 'JSON',
  'application/xml': 'XML',
  'application/rtf': 'RTF',
  'application/msword': 'DOC',
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'DOCX',
  'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'XLSX',
  'application/vnd.openxmlformats-officedocument.presentationml.presentation': 'PPTX',
  'application/vnd.ms-excel': 'XLS',
  'application/vnd.ms-powerpoint': 'PPT',
  'application/vnd.oasis.opendocument.text': 'ODT',
  'application/vnd.oasis.opendocument.spreadsheet': 'ODS',
};

/** Short uppercase type for the TYPE column, e.g. PDF, PNG, DOCX. */
export function shortType(mime: string | null | undefined): string {
  if (!mime) return '—';
  const known = SHORT[mime.toLowerCase()];
  if (known) return known;
  const sub = mime.split('/')[1] ?? mime;
  const tail = sub.split(/[.+-]/).filter(Boolean).pop() ?? sub;
  return tail.slice(0, 6).toUpperCase();
}
