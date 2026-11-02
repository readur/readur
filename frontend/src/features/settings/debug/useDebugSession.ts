import { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { api } from '../../../services/api';
import type { DebugInfo } from './types';

const POLL_MS = 2000;
const MONITOR_LIMIT_MS = 300000;
const MAX_NOT_FOUND_RETRIES = 3;

type Err = { response?: { status?: number; data?: { message?: string } }; message?: string };

/**
 * Debug state: fetch diagnostics for a document id (retrying 404s while a new upload settles),
 * upload a file and poll its OCR status until it completes, fails or times out.
 */
export function useDebugSession() {
  const { t } = useTranslation();
  const [documentId, setDocumentId] = useState('');
  const [debugInfo, setDebugInfo] = useState<DebugInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [selectedFile, setSelectedFile] = useState<File | null>(null);
  const [uploading, setUploading] = useState(false);
  const [uploadProgress, setUploadProgress] = useState(0);
  const [uploadedDocumentId, setUploadedDocumentId] = useState('');
  const [processingStatus, setProcessingStatus] = useState('');
  const [monitoring, setMonitoring] = useState(false);
  const timers = useRef<{ poll?: ReturnType<typeof setInterval>; stop?: ReturnType<typeof setTimeout> }>({});

  const stopMonitor = useCallback(() => {
    if (timers.current.poll) clearInterval(timers.current.poll);
    if (timers.current.stop) clearTimeout(timers.current.stop);
    timers.current = {};
    setMonitoring(false);
  }, []);

  useEffect(() => stopMonitor, [stopMonitor]);

  const fetchDebugInfo = useCallback(
    async (docId?: string, retryCount = 0): Promise<void> => {
      const target = (docId ?? documentId).trim();
      if (!target) {
        setError(t('debug.errors.enterDocumentId'));
        return;
      }
      setLoading(true);
      if (retryCount === 0) setError('');
      try {
        const response = await api.get(`/documents/${target}/debug`);
        setDebugInfo(response.data);
        setError('');
        setLoading(false);
      } catch (e) {
        const err = e as Err;
        if (err.response?.status === 404 && retryCount < MAX_NOT_FOUND_RETRIES) {
          setTimeout(() => void fetchDebugInfo(target, retryCount + 1), (retryCount + 1) * 1000);
          return;
        }
        setError(
          err.response?.status === 404
            ? t('debug.errors.documentNotFound', { documentId: target })
            : err.response?.data?.message || t('debug.errors.fetchFailed', { message: err.message }),
        );
        setDebugInfo(null);
        setLoading(false);
      }
    },
    [documentId, t],
  );

  const startMonitor = useCallback(
    (docId: string) => {
      stopMonitor();
      setMonitoring(true);
      timers.current.poll = setInterval(async () => {
        try {
          const doc = (await api.get(`/documents/${docId}`)).data;
          if (doc.ocr_status === 'completed' || doc.ocr_status === 'failed') {
            setProcessingStatus(t('debug.monitoring.processingComplete', { status: doc.ocr_status }));
            stopMonitor();
            // Give the server a moment to persist the result before reading diagnostics.
            setTimeout(() => void fetchDebugInfo(docId), POLL_MS);
          } else if (doc.ocr_status === 'processing') setProcessingStatus(t('debug.monitoring.ocrInProgress'));
          else if (doc.ocr_status === 'pending') setProcessingStatus(t('debug.monitoring.queuedForOcr'));
          else setProcessingStatus(t('debug.monitoring.checkingStatus'));
        } catch {
          /* keep polling; a transient failure should not end monitoring */
        }
      }, POLL_MS);
      timers.current.stop = setTimeout(() => {
        stopMonitor();
        setProcessingStatus(t('debug.monitoring.monitoringTimeout'));
      }, MONITOR_LIMIT_MS);
    },
    [fetchDebugInfo, stopMonitor, t],
  );

  const selectFile = (file: File | null) => {
    if (!file) return;
    setSelectedFile(file);
    setError('');
  };

  const upload = useCallback(async () => {
    if (!selectedFile) {
      setError(t('debug.upload.selectFile'));
      return;
    }
    setUploading(true);
    setUploadProgress(0);
    setError('');
    setProcessingStatus(t('debug.upload.uploading'));
    try {
      const formData = new FormData();
      formData.append('file', selectedFile);
      const response = await api.post('/documents', formData, {
        headers: { 'Content-Type': 'multipart/form-data' },
        onUploadProgress: (e) => setUploadProgress(e.total ? Math.round((e.loaded * 100) / e.total) : 0),
      });
      const id = response.data.id as string;
      setUploadedDocumentId(id);
      setDocumentId(id);
      setProcessingStatus(t('debug.upload.uploadedStartingOcr'));
      startMonitor(id);
    } catch (e) {
      setError((e as Err).response?.data?.message || t('debug.upload.uploadFailed'));
      setProcessingStatus(t('debug.upload.uploadFailedStatus'));
    } finally {
      setUploading(false);
      setUploadProgress(0);
    }
  }, [selectedFile, startMonitor, t]);

  return {
    documentId,
    setDocumentId,
    debugInfo,
    loading,
    error,
    fetchDebugInfo,
    selectedFile,
    selectFile,
    upload,
    uploading,
    uploadProgress,
    uploadedDocumentId,
    processingStatus,
    monitoring,
  };
}

export type DebugSession = ReturnType<typeof useDebugSession>;
