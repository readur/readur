import React, { useState, useCallback, useEffect } from 'react';
import {
  Box,
  Card,
  CardContent,
  Typography,
  Button,
  LinearProgress,
  Chip,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ListItemSecondaryAction,
  IconButton,
  Alert,
  Paper,
  alpha,
  useTheme,
  Divider,
} from '@mui/material';
import {
  CloudUpload as UploadIcon,
  InsertDriveFile as FileIcon,
  CheckCircle as CheckIcon,
  Error as ErrorIcon,
  Delete as DeleteIcon,
  Refresh as RefreshIcon,
} from '@mui/icons-material';
import { useDropzone, FileRejection, DropzoneOptions } from 'react-dropzone';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { api, ErrorHelper, ErrorCodes } from '../../services/api';
import { useNotifications } from '../../contexts/NotificationContext';
import LabelSelector from '../Labels/LabelSelector';
import { type LabelData } from '../Labels/Label';
import LanguageSelector from '../LanguageSelector';

interface UploadedDocument {
  id: string;
  original_filename: string;
  filename: string;
  file_size: number;
  mime_type: string;
  created_at: string;
}

interface FileItem {
  file: File;
  id: string;
  status: 'pending' | 'uploading' | 'success' | 'error' | 'timeout' | 'cancelled';
  progress: number;
  error: string | null;
  errorCode?: string;
  documentId?: string;
  uploadStartTime?: number;
  retryCount?: number;
}

interface UploadZoneProps {
  onUploadComplete?: (document: UploadedDocument) => void;
}

type FileStatus = 'pending' | 'uploading' | 'success' | 'error' | 'timeout' | 'cancelled';

const UploadZone: React.FC<UploadZoneProps> = ({ onUploadComplete }) => {
  const theme = useTheme();
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { addBatchNotification } = useNotifications();
  const [files, setFiles] = useState<FileItem[]>([]);
  const [uploading, setUploading] = useState<boolean>(false);
  const [uploadProgress, setUploadProgress] = useState<{ completed: number; total: number; failed: number }>({ completed: 0, total: 0, failed: 0 });
  const [concurrentUploads, setConcurrentUploads] = useState<Set<string>>(new Set());
  const [error, setError] = useState<string>('');
  const [selectedLabels, setSelectedLabels] = useState<LabelData[]>([]);
  const [availableLabels, setAvailableLabels] = useState<LabelData[]>([]);
  const [labelsLoading, setLabelsLoading] = useState<boolean>(false);
  const [selectedLanguages, setSelectedLanguages] = useState<string[]>(['eng']);
  const [primaryLanguage, setPrimaryLanguage] = useState<string>('eng');

  useEffect(() => {
    fetchLabels();
  }, []);

  const fetchLabels = async () => {
    try {
      setLabelsLoading(true);
      const response = await api.get('/labels?include_counts=false');
      
      if (response.status === 200 && Array.isArray(response.data)) {
        setAvailableLabels(response.data);
      } else {
        console.error('Failed to fetch labels:', response);
      }
    } catch (error) {
      console.error('Failed to fetch labels:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific label fetch errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_SESSION_EXPIRED) ||
          ErrorHelper.isErrorCode(error, ErrorCodes.USER_TOKEN_EXPIRED)) {
        setError(t('upload.errors.sessionExpired'));
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_PERMISSION_DENIED)) {
        setError(t('upload.errors.labelPermissionDenied'));
      } else if (errorInfo.category === 'network') {
        setError(t('upload.errors.labelNetworkError'));
      } else {
        // Don't show error for label loading failures as it's not critical
        console.warn('Label loading failed:', errorInfo.message);
      }
    } finally {
      setLabelsLoading(false);
    }
  };

  const handleCreateLabel = async (labelData: Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>) => {
    try {
      const response = await api.post('/labels', labelData);
      const newLabel = response.data;
      setAvailableLabels(prev => [...prev, newLabel]);
      return newLabel;
    } catch (error) {
      console.error('Failed to create label:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific label creation errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_DUPLICATE_NAME)) {
        throw new Error(t('labels.errors.duplicateName'));
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_INVALID_NAME)) {
        throw new Error(t('labels.errors.invalidName'));
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_INVALID_COLOR)) {
        throw new Error(t('labels.errors.invalidColor'));
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_MAX_LABELS_REACHED)) {
        throw new Error(t('labels.errors.maxLabelsReached'));
      } else {
        throw new Error(errorInfo.message || t('labels.errors.loadFailed'));
      }
    }
  };

  const handleLanguagesChange = (languages: string[], primary?: string) => {
    setSelectedLanguages(languages);
    if (primary) {
      setPrimaryLanguage(primary);
    } else if (languages.length > 0) {
      setPrimaryLanguage(languages[0]);
    }
  };

  const onDrop = useCallback((acceptedFiles: File[], rejectedFiles: FileRejection[]) => {
    setError('');
    
    // Handle rejected files
    if (rejectedFiles.length > 0) {
      const errors = rejectedFiles.map(file => 
        `${file.file.name}: ${file.errors.map(e => e.message).join(', ')}`
      );
      setError(`Some files were rejected: ${errors.join('; ')}`);
    }

    // Add accepted files to the list
    const newFiles: FileItem[] = acceptedFiles.map(file => ({
      file,
      id: Math.random().toString(36).substr(2, 9),
      status: 'pending' as FileStatus,
      progress: 0,
      error: null,
      retryCount: 0,
    }));

    setFiles(prev => [...prev, ...newFiles]);
  }, []);

  const dropzoneOptions: DropzoneOptions = {
    onDrop,
    accept: {
      'application/pdf': ['.pdf'],
      'image/*': ['.png', '.jpg', '.jpeg', '.gif', '.bmp', '.tiff'],
      'text/*': ['.txt', '.rtf'],
      'application/msword': ['.doc'],
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document': ['.docx'],
    },
    maxSize: 50 * 1024 * 1024, // 50MB
    multiple: true,
  };

  const { getRootProps, getInputProps, isDragActive } = useDropzone(dropzoneOptions);

  const removeFile = (fileId: string): void => {
    setFiles(prev => prev.filter(f => f.id !== fileId));
  };

  const uploadFile = async (fileItem: FileItem, isRetry: boolean = false): Promise<void> => {
    const uploadId = fileItem.id;
    const maxRetries = 3;
    const uploadTimeout = 60000; // 60 seconds timeout
    
    // Prevent concurrent uploads of the same file
    if (concurrentUploads.has(uploadId)) {
      console.warn(`Upload already in progress for file: ${fileItem.file.name}`);
      return;
    }
    
    setConcurrentUploads(prev => new Set(prev.add(uploadId)));
    const formData = new FormData();
    formData.append('file', fileItem.file);
    
    // Add selected labels to the form data
    if (selectedLabels.length > 0) {
      const labelIds = selectedLabels.map(label => label.id);
      formData.append('label_ids', JSON.stringify(labelIds));
    }

    // Add selected languages to the form data
    if (selectedLanguages.length > 0) {
      selectedLanguages.forEach((lang, index) => {
        formData.append(`ocr_languages[${index}]`, lang);
      });
    }

    try {
      const uploadStartTime = Date.now();
      
      setFiles(prev => prev.map(f => 
        f.id === fileItem.id 
          ? { ...f, status: 'uploading' as FileStatus, progress: 0, uploadStartTime, error: null, errorCode: undefined }
          : f
      ));

      const response = await api.post<UploadedDocument>('/documents', formData, {
        headers: {
          'Content-Type': 'multipart/form-data',
        },
        onUploadProgress: (progressEvent) => {
          if (progressEvent.total) {
            const progress = Math.round((progressEvent.loaded * 100) / progressEvent.total);
            setFiles(prev => prev.map(f => 
              f.id === fileItem.id 
                ? { ...f, progress }
                : f
            ));
          }
        },
      });

      setFiles(prev => prev.map(f => 
        f.id === fileItem.id 
          ? { ...f, status: 'success' as FileStatus, progress: 100, documentId: response.data.id }
          : f
      ));

      if (onUploadComplete) {
        onUploadComplete(response.data);
      }
    } catch (error: any) {
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      let errorMessage = 'Upload failed';
      let errorCode = 'UNKNOWN_ERROR';
      let newStatus: FileStatus = 'error';
      
      // Handle specific document upload errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.DOCUMENT_TOO_LARGE)) {
        errorMessage = t('upload.errors.fileTooLarge');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.DOCUMENT_INVALID_FORMAT)) {
        errorMessage = t('upload.errors.unsupportedFormat');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.DOCUMENT_OCR_FAILED)) {
        errorMessage = t('upload.errors.processingFailed');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_SESSION_EXPIRED) ||
                 ErrorHelper.isErrorCode(error, ErrorCodes.USER_TOKEN_EXPIRED)) {
        errorMessage = t('upload.errors.sessionExpired');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_PERMISSION_DENIED)) {
        errorMessage = t('upload.errors.permissionDenied');
      } else if (errorInfo.category === 'network') {
        errorMessage = t('upload.errors.networkError');
      } else if (errorInfo.category === 'server') {
        errorMessage = t('upload.errors.serverError');
      } else {
        errorMessage = errorInfo.message || t('common.status.failed');
      }
      
      setFiles(prev => prev.map(f => 
        f.id === fileItem.id 
          ? { 
              ...f, 
              status: newStatus, 
              error: errorMessage,
              errorCode,
              progress: 0,
            }
          : f
      ));
    } finally {
      setConcurrentUploads(prev => {
        const newSet = new Set(prev);
        newSet.delete(uploadId);
        return newSet;
      });
    }
  };

  const uploadAllFiles = async (): Promise<void> => {
    if (uploading) {
      console.warn('[UPLOAD_DEBUG] Upload already in progress, ignoring request');
      return;
    }
    
    setUploading(true);
    setError('');

    const pendingFiles = files.filter(f => f.status === 'pending' || f.status === 'error' || f.status === 'timeout');
    const results: { name: string; success: boolean; errorCode?: string }[] = [];
    
    if (pendingFiles.length === 0) {
      setUploading(false);
      return;
    }
    
    console.log(`[UPLOAD_DEBUG] Starting upload of ${pendingFiles.length} files`);
    
    // Initialize progress tracking
    setUploadProgress({ completed: 0, total: pendingFiles.length, failed: 0 });
    
    // Limit concurrent uploads to prevent overwhelming the server
    const maxConcurrentUploads = 3;
    const uploadQueue: FileItem[] = [...pendingFiles];
    const activeUploads: Promise<void>[] = [];
    
    const processNextUpload = async (): Promise<void> => {
      if (uploadQueue.length === 0) return;
      
      const fileItem = uploadQueue.shift()!;
      
      try {
        await uploadFile(fileItem);
        results.push({ name: fileItem.file.name, success: true });
        setUploadProgress(prev => ({ ...prev, completed: prev.completed + 1 }));
        console.log(`[UPLOAD_DEBUG] Upload succeeded for ${fileItem.file.name}`);
      } catch (error) {
        const errorCode = error?.response?.data?.error_code || 'UNKNOWN_ERROR';
        results.push({ name: fileItem.file.name, success: false, errorCode });
        setUploadProgress(prev => ({ ...prev, completed: prev.completed + 1, failed: prev.failed + 1 }));
        console.error(`[UPLOAD_DEBUG] Upload failed for ${fileItem.file.name}:`, error);
      }
      
      // Process next file in queue
      if (uploadQueue.length > 0) {
        return processNextUpload();
      }
    };
    
    try {
      // Start initial batch of concurrent uploads
      for (let i = 0; i < Math.min(maxConcurrentUploads, uploadQueue.length); i++) {
        activeUploads.push(processNextUpload());
      }
      
      // Wait for all uploads to complete
      await Promise.allSettled(activeUploads);
      
      console.log(`[UPLOAD_DEBUG] Batch upload completed. ${results.filter(r => r.success).length} succeeded, ${results.filter(r => !r.success).length} failed`);
      
      // Trigger notification based on results
      const hasFailures = results.some(r => !r.success);
      const hasSuccesses = results.some(r => r.success);
      
      if (!hasFailures) {
        addBatchNotification('success', 'upload', results);
      } else if (!hasSuccesses) {
        addBatchNotification('error', 'upload', results);
      } else {
        addBatchNotification('warning', 'upload', results);
      }
      
      // Show detailed error information if there were failures
      if (hasFailures) {
        const errorsByType = results
          .filter(r => !r.success)
          .reduce((acc, r) => {
            const key = r.errorCode || 'UNKNOWN_ERROR';
            acc[key] = (acc[key] || 0) + 1;
            return acc;
          }, {} as Record<string, number>);
        
        console.warn('[UPLOAD_DEBUG] Upload failures by error type:', errorsByType);
      }
    } catch (error) {
      console.error('[UPLOAD_DEBUG] Critical error during batch upload:', error);
      setError('Upload failed due to an unexpected error. Please refresh the page and try again.');
    } finally {
      setUploading(false);
      // Reset concurrent uploads tracking
      setConcurrentUploads(new Set());
    }
  };

  const retryUpload = (fileItem: FileItem): void => {
    uploadFile(fileItem);
  };

  const clearCompleted = (): void => {
    setFiles(prev => prev.filter(f => f.status !== 'success'));
  };

  const formatFileSize = (bytes: number): string => {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  };

  const getStatusColor = (status: FileStatus): string => {
    switch (status) {
      case 'success': return theme.palette.success.main;
      case 'error': return theme.palette.error.main;
      case 'timeout': return theme.palette.warning.main;
      case 'cancelled': return theme.palette.text.disabled;
      case 'uploading': return theme.palette.primary.main;
      default: return theme.palette.text.secondary;
    }
  };

  const getStatusIcon = (status: FileStatus): React.ReactElement => {
    switch (status) {
      case 'success': return <CheckIcon />;
      case 'error': return <ErrorIcon />;
      case 'timeout': return <ErrorIcon />;
      case 'cancelled': return <DeleteIcon />;
      case 'uploading': return <UploadIcon />;
      default: return <FileIcon />;
    }
  };

  const handleFileClick = (fileItem: FileItem) => {
    if (fileItem.status === 'success' && fileItem.documentId) {
      navigate(`/documents/${fileItem.documentId}`);
    }
  };

  return (
    <Box>
      {/* Upload Drop Zone */}
      <Card 
        elevation={0}
        sx={{ 
          mb: 3,
          border: `2px dashed ${isDragActive ? theme.palette.primary.main : theme.palette.divider}`,
          backgroundColor: isDragActive ? alpha(theme.palette.primary.main, 0.04) : 'transparent',
          transition: 'all 0.2s ease-in-out',
        }}
      >
        <CardContent>
          <Box
            {...getRootProps()}
            sx={{
              textAlign: 'center',
              py: 6,
              cursor: 'pointer',
              outline: 'none',
            }}
          >
            <input {...getInputProps()} />
            
            <UploadIcon 
              sx={{ 
                fontSize: 64, 
                color: isDragActive ? 'primary.main' : 'text.secondary',
                mb: 2,
              }} 
            />
            
            <Typography variant="h6" sx={{ mb: 1, fontWeight: 600 }}>
              {isDragActive ? t('upload.dropzone.dropHere') : t('upload.dropzone.dragDrop')}
            </Typography>

            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              {t('upload.dropzone.browse')}
            </Typography>

            <Button
              variant="contained"
              sx={{
                mb: 2,
                borderRadius: 2,
                px: 3,
              }}
            >
              {t('upload.dropzone.chooseFiles')}
            </Button>

            <Box sx={{ display: 'flex', justifyContent: 'center', gap: 1, flexWrap: 'wrap' }}>
              <Chip label={t('upload.dropzone.fileTypes.pdf')} size="small" variant="outlined" />
              <Chip label={t('upload.dropzone.fileTypes.images')} size="small" variant="outlined" />
              <Chip label={t('upload.dropzone.fileTypes.text')} size="small" variant="outlined" />
              <Chip label={t('upload.dropzone.fileTypes.word')} size="small" variant="outlined" />
            </Box>

            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 2 }}>
              {t('upload.dropzone.maxFileSize')}
            </Typography>
          </Box>
        </CardContent>
      </Card>

      {/* Error Alert */}
      {error && (
        <Alert severity="error" sx={{ mb: 3, borderRadius: 2 }}>
          {error}
        </Alert>
      )}

      {/* Language Selection */}
      <Card elevation={0} sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" sx={{ fontWeight: 600, mb: 2 }}>
            {t('upload.languageSettings.title')}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            {t('upload.languageSettings.description')}
          </Typography>
          <Box sx={{ '& > div': { width: '100%' } }}>
            <LanguageSelector
              selectedLanguages={selectedLanguages}
              primaryLanguage={primaryLanguage}
              onLanguagesChange={handleLanguagesChange}
              disabled={uploading}
            />
          </Box>
        </CardContent>
      </Card>

      {/* Label Selection */}
      <Card elevation={0} sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" sx={{ fontWeight: 600, mb: 2 }}>
            {t('upload.labelAssignment.title')}
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            {t('upload.labelAssignment.description')}
          </Typography>
          <LabelSelector
            selectedLabels={selectedLabels}
            availableLabels={availableLabels}
            onLabelsChange={setSelectedLabels}
            onCreateLabel={handleCreateLabel}
            placeholder={t('upload.labelAssignment.placeholder')}
            size="medium"
            disabled={labelsLoading}
          />
          {selectedLabels.length > 0 && (
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
              {t('upload.labelAssignment.helperText')}
            </Typography>
          )}
        </CardContent>
      </Card>

      {/* File List */}
      {files.length > 0 && (
        <Card elevation={0}>
          <CardContent>
            <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
              <Typography variant="h6" sx={{ fontWeight: 600 }}>
                {t('upload.fileList.title', { count: files.length })}
              </Typography>
              <Box sx={{ display: 'flex', gap: 1 }}>
                <Button
                  size="small"
                  onClick={clearCompleted}
                  disabled={!files.some(f => f.status === 'success')}
                >
                  {t('upload.fileList.clearCompleted')}
                </Button>
                <Button
                  variant="contained"
                  size="small"
                  onClick={uploadAllFiles}
                  disabled={uploading || !files.some(f => f.status === 'pending' || f.status === 'error' || f.status === 'timeout')}
                  sx={{ borderRadius: 2 }}
                >
                  {uploading ? (
                    uploadProgress.total > 0 ?
                      t('upload.fileList.uploading', { completed: uploadProgress.completed, total: uploadProgress.total }) :
                      t('upload.fileList.uploadingSimple')
                  ) : t('upload.fileList.uploadAll')}
                </Button>
              </Box>
            </Box>

            <List sx={{ p: 0 }}>
              {files.map((fileItem, index) => (
                <ListItem 
                  key={fileItem.id}
                  sx={{ 
                    px: 0,
                    py: 2,
                    borderBottom: index < files.length - 1 ? 1 : 0,
                    borderColor: 'divider',
                    cursor: fileItem.status === 'success' && fileItem.documentId ? 'pointer' : 'default',
                    '&:hover': fileItem.status === 'success' && fileItem.documentId ? {
                      backgroundColor: alpha(theme.palette.primary.main, 0.04),
                    } : {},
                  }}
                  onClick={() => handleFileClick(fileItem)}
                >
                  <ListItemIcon>
                    <Box sx={{ color: getStatusColor(fileItem.status) }}>
                      {getStatusIcon(fileItem.status)}
                    </Box>
                  </ListItemIcon>
                  
                  <ListItemText
                    sx={{ 
                      pr: 6, // Add padding-right to prevent overlap with secondary action
                    }}
                    primary={
                      <Typography 
                        variant="subtitle2" 
                        sx={{ 
                          fontWeight: 500,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                          maxWidth: '100%',
                        }}
                        title={fileItem.file.name}
                      >
                        {fileItem.file.name}
                      </Typography>
                    }
                    secondary={
                      <Box>
                        <Typography variant="caption" color="text.secondary">
                          {formatFileSize(fileItem.file.size)}
                        </Typography>
                        {fileItem.status === 'uploading' && (
                          <Box sx={{ mt: 1 }}>
                            <LinearProgress 
                              variant="determinate" 
                              value={fileItem.progress}
                              sx={{ height: 4, borderRadius: 2 }}
                            />
                            <Typography variant="caption" color="text.secondary">
                              {fileItem.progress}%
                            </Typography>
                          </Box>
                        )}
                        {fileItem.error && (
                          <Typography 
                            variant="caption" 
                            color={fileItem.status === 'timeout' ? 'warning' : 'error'} 
                            sx={{ display: 'block', mt: 0.5 }}
                            title={fileItem.errorCode ? `Error Code: ${fileItem.errorCode}` : undefined}
                          >
                            {fileItem.error}
                            {fileItem.retryCount && fileItem.retryCount > 0 && (
                              <span style={{ marginLeft: '8px', fontSize: '0.8em' }}>
                                (Attempt {fileItem.retryCount + 1})
                              </span>
                            )}
                          </Typography>
                        )}
                      </Box>
                    }
                  />
                  
                  <ListItemSecondaryAction>
                    <Box sx={{ display: 'flex', gap: 0.5 }}>
                      {(fileItem.status === 'error' || fileItem.status === 'timeout') && (
                        <IconButton 
                          size="small" 
                          onClick={(e) => {
                            e.stopPropagation();
                            retryUpload(fileItem);
                          }}
                          sx={{ color: 'primary.main' }}
                          title={`Retry upload${fileItem.errorCode ? ` (Error: ${fileItem.errorCode})` : ''}`}
                        >
                          <RefreshIcon fontSize="small" />
                        </IconButton>
                      )}
                      <IconButton 
                        size="small" 
                        onClick={(e) => {
                          e.stopPropagation();
                          removeFile(fileItem.id);
                        }}
                        disabled={fileItem.status === 'uploading'}
                      >
                        <DeleteIcon fontSize="small" />
                      </IconButton>
                    </Box>
                  </ListItemSecondaryAction>
                </ListItem>
              ))}
            </List>
          </CardContent>
        </Card>
      )}
    </Box>
  );
};

export default UploadZone;