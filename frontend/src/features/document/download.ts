/** Hands a downloaded blob to the browser as a file save. */
export function saveBlob(data: BlobPart, filename: string, type?: string): void {
  const url = URL.createObjectURL(new Blob([data], type ? { type } : undefined));
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.rel = 'noopener';
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

/** Filename from a Content-Disposition header, if it names one. */
export function filenameFromDisposition(header: string | undefined): string | null {
  const match = header?.match(/filename\*?=(?:UTF-8'')?"?([^";\n]+)"?/i);
  if (!match) return null;
  try {
    return decodeURIComponent(match[1]);
  } catch {
    return match[1];
  }
}
