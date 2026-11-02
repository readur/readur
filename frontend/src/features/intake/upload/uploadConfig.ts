import type { Accept } from 'react-dropzone';

/** File types accepted by the drop area (unchanged from the previous upload page). */
export const ACCEPT: Accept = {
  'application/pdf': ['.pdf'],
  'image/*': ['.png', '.jpg', '.jpeg', '.gif', '.bmp', '.tiff'],
  'text/*': ['.txt', '.rtf'],
  'application/msword': ['.doc'],
  'application/vnd.openxmlformats-officedocument.wordprocessingml.document': ['.docx'],
};

export const MAX_FILE_SIZE_MB = 50;
export const MAX_FILE_SIZE = MAX_FILE_SIZE_MB * 1024 * 1024;

/** Uploads running at once; the rest wait in the queue. */
export const MAX_CONCURRENT_UPLOADS = 3;

/** Extensions for display: "PDF PNG JPG …". */
export const ACCEPTED_EXTENSIONS: string[] = Array.from(
  new Set(Object.values(ACCEPT).flat().map((ext) => ext.replace(/^\./, '').toUpperCase())),
);
