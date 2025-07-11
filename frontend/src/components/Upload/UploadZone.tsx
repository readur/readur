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
import api from '../../services/api';
import { useNotifications } from '../../contexts/NotificationContext';
import LabelSelector from '../Labels/LabelSelector';
import { type LabelData } from '../Labels/Label';

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
  status: 'pending' | 'uploading' | 'success' | 'error';
  progress: number;
  error: string | null;
}

interface UploadZoneProps {
  onUploadComplete?: (document: UploadedDocument) => void;
}

type FileStatus = 'pending' | 'uploading' | 'success' | 'error';

const UploadZone: React.FC<UploadZoneProps> = ({ onUploadComplete }) => {
  const theme = useTheme();
  const { addBatchNotification } = useNotifications();
  const [files, setFiles] = useState<FileItem[]>([]);
  const [uploading, setUploading] = useState<boolean>(false);
  const [error, setError] = useState<string>('');
  const [selectedLabels, setSelectedLabels] = useState<LabelData[]>([]);
  const [availableLabels, setAvailableLabels] = useState<LabelData[]>([]);
  const [labelsLoading, setLabelsLoading] = useState<boolean>(false);

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
      throw error;
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

  const uploadFile = async (fileItem: FileItem): Promise<void> => {
    const formData = new FormData();
    formData.append('file', fileItem.file);
    
    // Add selected labels to the form data
    if (selectedLabels.length > 0) {
      const labelIds = selectedLabels.map(label => label.id);
      formData.append('label_ids', JSON.stringify(labelIds));
    }

    try {
      setFiles(prev => prev.map(f => 
        f.id === fileItem.id 
          ? { ...f, status: 'uploading' as FileStatus, progress: 0 }
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
          ? { ...f, status: 'success' as FileStatus, progress: 100 }
          : f
      ));

      if (onUploadComplete) {
        onUploadComplete(response.data);
      }
    } catch (error: any) {
      setFiles(prev => prev.map(f => 
        f.id === fileItem.id 
          ? { 
              ...f, 
              status: 'error' as FileStatus, 
              error: error.response?.data?.message || 'Upload failed',
              progress: 0,
            }
          : f
      ));
    }
  };

  const uploadAllFiles = async (): Promise<void> => {
    setUploading(true);
    setError('');

    const pendingFiles = files.filter(f => f.status === 'pending' || f.status === 'error');
    const results: { name: string; success: boolean }[] = [];
    
    try {
      await Promise.allSettled(pendingFiles.map(async (file) => {
        try {
          await uploadFile(file);
          results.push({ name: file.file.name, success: true });
        } catch (error) {
          results.push({ name: file.file.name, success: false });
        }
      }));
      
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
    } catch (error) {
      setError('Upload failed. Please try again.');
    } finally {
      setUploading(false);
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
      case 'uploading': return theme.palette.primary.main;
      default: return theme.palette.text.secondary;
    }
  };

  const getStatusIcon = (status: FileStatus): React.ReactElement => {
    switch (status) {
      case 'success': return <CheckIcon />;
      case 'error': return <ErrorIcon />;
      case 'uploading': return <UploadIcon />;
      default: return <FileIcon />;
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
              {isDragActive ? 'Drop files here' : 'Drag & drop files here'}
            </Typography>
            
            <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
              or click to browse your computer
            </Typography>
            
            <Button 
              variant="contained" 
              sx={{ 
                mb: 2,
                borderRadius: 2,
                px: 3,
              }}
            >
              Choose Files
            </Button>
            
            <Box sx={{ display: 'flex', justifyContent: 'center', gap: 1, flexWrap: 'wrap' }}>
              <Chip label="PDF" size="small" variant="outlined" />
              <Chip label="Images" size="small" variant="outlined" />
              <Chip label="Text" size="small" variant="outlined" />
              <Chip label="Word" size="small" variant="outlined" />
            </Box>
            
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 2 }}>
              Maximum file size: 50MB per file
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

      {/* Label Selection */}
      <Card elevation={0} sx={{ mb: 3 }}>
        <CardContent>
          <Typography variant="h6" sx={{ fontWeight: 600, mb: 2 }}>
            📋 Label Assignment
          </Typography>
          <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
            Select labels to automatically assign to all uploaded documents
          </Typography>
          <LabelSelector
            selectedLabels={selectedLabels}
            availableLabels={availableLabels}
            onLabelsChange={setSelectedLabels}
            onCreateLabel={handleCreateLabel}
            placeholder="Choose labels for your documents..."
            size="medium"
            disabled={labelsLoading}
          />
          {selectedLabels.length > 0 && (
            <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 1 }}>
              These labels will be applied to all uploaded documents
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
                Files ({files.length})
              </Typography>
              <Box sx={{ display: 'flex', gap: 1 }}>
                <Button
                  size="small"
                  onClick={clearCompleted}
                  disabled={!files.some(f => f.status === 'success')}
                >
                  Clear Completed
                </Button>
                <Button
                  variant="contained"
                  size="small"
                  onClick={uploadAllFiles}
                  disabled={uploading || !files.some(f => f.status === 'pending' || f.status === 'error')}
                  sx={{ borderRadius: 2 }}
                >
                  {uploading ? 'Uploading...' : 'Upload All'}
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
                  }}
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
                          <Typography variant="caption" color="error" sx={{ display: 'block', mt: 0.5 }}>
                            {fileItem.error}
                          </Typography>
                        )}
                      </Box>
                    }
                  />
                  
                  <ListItemSecondaryAction>
                    <Box sx={{ display: 'flex', gap: 0.5 }}>
                      {fileItem.status === 'error' && (
                        <IconButton 
                          size="small" 
                          onClick={() => retryUpload(fileItem)}
                          sx={{ color: 'primary.main' }}
                        >
                          <RefreshIcon fontSize="small" />
                        </IconButton>
                      )}
                      <IconButton 
                        size="small" 
                        onClick={() => removeFile(fileItem.id)}
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