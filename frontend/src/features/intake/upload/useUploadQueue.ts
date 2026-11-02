import { useCallback, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { FileRejection } from 'react-dropzone';
import { api, labelService } from '../../../services/api';
import { useNotifications } from '../../../contexts/NotificationContext';
import { markLit } from '../../board/litStore';
import { categoryOf, ErrorCodes, hasCode, serverMessage } from '../shared/errors';
import { MAX_CONCURRENT_UPLOADS } from './uploadConfig';

export type UploadStatus = 'pending' | 'uploading' | 'success' | 'error';

export interface UploadItem {
  id: string;
  file: File;
  status: UploadStatus;
  progress: number;
  error: string | null;
  documentId?: string;
}

export interface UploadOptions {
  labelIds: string[];
  languages: string[];
}

interface UploadedDocument {
  id: string;
}

let counter = 0;
const nextId = () => `u${Date.now().toString(36)}${(counter++).toString(36)}`;

/**
 * Files waiting, uploading and done. Uploads run three at a time, report progress per file,
 * and a finished upload marks its new document for the Board and Library.
 */
export function useUploadQueue(getOptions: () => UploadOptions, onUploaded?: (documentId: string) => void) {
  const { t } = useTranslation();
  const { addBatchNotification } = useNotifications();
  const [items, setItems] = useState<UploadItem[]>([]);
  const [isUploading, setUploading] = useState(false);
  const [batch, setBatch] = useState({ done: 0, total: 0 });
  const [rejected, setRejected] = useState<string | null>(null);
  const inFlight = useRef(new Set<string>());
  const itemsRef = useRef(items);
  itemsRef.current = items;

  const patch = (id: string, change: Partial<UploadItem>) =>
    setItems((prev) => prev.map((i) => (i.id === id ? { ...i, ...change } : i)));

  const add = useCallback((accepted: File[], rejections: FileRejection[] = []) => {
    setRejected(
      rejections.length > 0
        ? rejections.map((r) => `${r.file.name}: ${r.errors.map((e) => e.message).join(', ')}`).join('; ')
        : null,
    );
    if (accepted.length === 0) return;
    setItems((prev) => [
      ...prev,
      ...accepted.map((file) => ({ id: nextId(), file, status: 'pending' as const, progress: 0, error: null })),
    ]);
  }, []);

  const messageFor = (error: unknown): string => {
    if (hasCode(error, ErrorCodes.DOCUMENT_TOO_LARGE)) return t('intake.upload.errors.tooLarge', 'The file is too large.');
    if (hasCode(error, ErrorCodes.DOCUMENT_INVALID_FORMAT)) return t('intake.upload.errors.format', 'This file type is not supported.');
    if (hasCode(error, ErrorCodes.DOCUMENT_OCR_FAILED)) return t('intake.upload.errors.processing', 'The file could not be processed.');
    if (hasCode(error, ErrorCodes.USER_SESSION_EXPIRED) || hasCode(error, ErrorCodes.USER_TOKEN_EXPIRED)) {
      return t('intake.upload.errors.session', 'Your session expired. Sign in again.');
    }
    if (hasCode(error, ErrorCodes.USER_PERMISSION_DENIED)) return t('intake.upload.errors.permission', 'You are not allowed to upload.');
    const category = categoryOf(error);
    if (category === 'network') return t('intake.upload.errors.network', 'Network error. Check your connection.');
    if (category === 'server') return t('intake.upload.errors.server', 'The server had a problem. Try again.');
    return serverMessage(error) ?? t('intake.upload.errors.generic', 'Upload failed');
  };

  /** Never rejects. 'skipped' means the file is already uploading (e.g. a row Retry during a batch). */
  const uploadOne = async (item: UploadItem): Promise<'ok' | 'failed' | 'skipped'> => {
    if (inFlight.current.has(item.id)) return 'skipped';
    inFlight.current.add(item.id);
    const { labelIds, languages } = getOptions();
    const form = new FormData();
    form.append('file', item.file);
    languages.forEach((lang, i) => form.append(`ocr_languages[${i}]`, lang));

    patch(item.id, { status: 'uploading', progress: 0, error: null });
    try {
      const res = await api.post<UploadedDocument>('/documents', form, {
        headers: { 'Content-Type': 'multipart/form-data' },
        onUploadProgress: (e) => {
          if (e.total) patch(item.id, { progress: Math.round((e.loaded * 100) / e.total) });
        },
      });
      const documentId = res.data?.id;
      patch(item.id, { status: 'success', progress: 100, documentId });
      if (documentId) {
        markLit('document', documentId, 'new');
        onUploaded?.(documentId);
        // The upload endpoint does not take labels; add them to the new document.
        if (labelIds.length > 0) {
          try {
            await labelService.bulkAssign([documentId], labelIds, 'add');
          } catch {
            patch(item.id, { error: t('intake.upload.errors.labels', 'Uploaded, but the labels could not be added.') });
          }
        }
      }
      return 'ok';
    } catch (error) {
      patch(item.id, { status: 'error', progress: 0, error: messageFor(error) });
      return 'failed';
    } finally {
      inFlight.current.delete(item.id);
    }
  };

  const uploadAll = async () => {
    if (isUploading) return;
    const queue = itemsRef.current.filter((i) => i.status === 'pending' || i.status === 'error');
    if (queue.length === 0) return;
    setUploading(true);
    setBatch({ done: 0, total: queue.length });
    const results: { name: string; success: boolean }[] = [];
    const worker = async () => {
      for (let item = queue.shift(); item; item = queue.shift()) {
        const current = itemsRef.current.find((i) => i.id === item.id);
        const stillWaiting = current && (current.status === 'pending' || current.status === 'error');
        const outcome = stillWaiting ? await uploadOne(item) : 'skipped';
        if (outcome === 'skipped') {
          // Removed, or already uploaded/uploading through a row Retry, which reports itself.
          setBatch((b) => ({ ...b, total: Math.max(0, b.total - 1) }));
          continue;
        }
        results.push({ name: item.file.name, success: outcome === 'ok' });
        setBatch((b) => ({ ...b, done: b.done + 1 }));
      }
    };
    await Promise.all(Array.from({ length: Math.min(MAX_CONCURRENT_UPLOADS, queue.length) }, worker));
    const failures = results.filter((r) => !r.success).length;
    if (results.length > 0) addBatchNotification(failures === 0 ? 'success' : failures === results.length ? 'error' : 'warning', 'upload', results);
    setUploading(false);
  };

  const retry = (id: string) => {
    const item = itemsRef.current.find((i) => i.id === id);
    if (item) void uploadOne(item);
  };
  const remove = (id: string) => setItems((prev) => prev.filter((i) => i.id !== id));
  const clearCompleted = () => setItems((prev) => prev.filter((i) => i.status !== 'success'));

  return { items, add, uploadAll, retry, remove, clearCompleted, isUploading, batch, rejected };
}
