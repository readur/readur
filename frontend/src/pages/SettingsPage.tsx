import React, { useState, useEffect, useCallback } from 'react';
import {
  Box,
  Container,
  Typography,
  Paper,
  Tabs,
  Tab,
  FormControl,
  FormControlLabel,
  InputLabel,
  Select,
  MenuItem,
  Button,
  Snackbar,
  Alert,
  TextField,
  Table,
  TableBody,
  TableCell,
  TableContainer,
  TableHead,
  TableRow,
  IconButton,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Card,
  CardContent,
  Divider,
  Switch,
  SelectChangeEvent,
  Chip,
  LinearProgress,
  CircularProgress,
  Tooltip,
  Divider,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import { Edit as EditIcon, Delete as DeleteIcon, Add as AddIcon, 
         CloudSync as CloudSyncIcon, Folder as FolderIcon,
         Assessment as AssessmentIcon, PlayArrow as PlayArrowIcon,
         Pause as PauseIcon, Stop as StopIcon, CheckCircle as CheckCircleIcon,
         Error as ErrorIcon, Visibility as VisibilityIcon, CreateNewFolder as CreateNewFolderIcon,
         RemoveCircle as RemoveCircleIcon, Warning as WarningIcon } from '@mui/icons-material';
import { useAuth } from '../contexts/AuthContext';
import api, { queueService, ErrorHelper, ErrorCodes, userWatchService, UserWatchDirectoryResponse } from '../services/api';
import OcrLanguageSelector from '../components/OcrLanguageSelector';
import LanguageSelector from '../components/LanguageSelector';

interface User {
  id: string;
  username: string;
  email: string;
  created_at: string;
}

interface Settings {
  ocrLanguage: string;
  preferredLanguages: string[];
  primaryLanguage: string;
  autoDetectLanguageCombination: boolean;
  concurrentOcrJobs: number;
  ocrTimeoutSeconds: number;
  maxFileSizeMb: number;
  allowedFileTypes: string[];
  autoRotateImages: boolean;
  enableImagePreprocessing: boolean;
  searchResultsPerPage: number;
  searchSnippetLength: number;
  fuzzySearchThreshold: number;
  retentionDays: number | null;
  enableAutoCleanup: boolean;
  enableCompression: boolean;
  memoryLimitMb: number;
  cpuPriority: string;
  enableBackgroundOcr: boolean;
  ocrPageSegmentationMode: number;
  ocrEngineMode: number;
  ocrMinConfidence: number;
  ocrDpi: number;
  ocrEnhanceContrast: boolean;
  ocrRemoveNoise: boolean;
  ocrDetectOrientation: boolean;
  ocrWhitelistChars: string;
  ocrBlacklistChars: string;
  ocrBrightnessBoost: number;
  ocrContrastMultiplier: number;
  ocrNoiseReductionLevel: number;
  ocrSharpeningStrength: number;
  ocrMorphologicalOperations: boolean;
  ocrAdaptiveThresholdWindowSize: number;
  ocrHistogramEqualization: boolean;
  ocrUpscaleFactor: number;
  ocrMaxImageWidth: number;
  ocrMaxImageHeight: number;
  saveProcessedImages: boolean;
  ocrQualityThresholdBrightness: number;
  ocrQualityThresholdContrast: number;
  ocrQualityThresholdNoise: number;
  ocrQualityThresholdSharpness: number;
  ocrSkipEnhancement: boolean;
}

interface SnackbarState {
  open: boolean;
  message: string;
  severity: 'success' | 'error' | 'warning' | 'info';
}

interface UserDialogState {
  open: boolean;
  mode: 'create' | 'edit';
  user: User | null;
}

interface UserFormData {
  username: string;
  email: string;
  password: string;
}


interface WebDAVFolderInfo {
  path: string;
  total_files: number;
  supported_files: number;
  estimated_time_hours: number;
  total_size_mb: number;
}

interface WebDAVCrawlEstimate {
  folders: WebDAVFolderInfo[];
  total_files: number;
  total_supported_files: number;
  total_estimated_time_hours: number;
  total_size_mb: number;
}

interface WebDAVConnectionResult {
  success: boolean;
  message: string;
  server_version?: string;
  server_type?: string;
}

interface ServerConfiguration {
  max_file_size_mb: number;
  concurrent_ocr_jobs: number;
  ocr_timeout_seconds: number;
  memory_limit_mb: number;
  cpu_priority: string;
  server_host: string;
  server_port: number;
  jwt_secret_set: boolean;
  upload_path: string;
  watch_folder?: string;
  ocr_language: string;
  allowed_file_types: string[];
  watch_interval_seconds?: number;
  file_stability_check_ms?: number;
  max_file_age_hours?: number;
  enable_background_ocr: boolean;
  version: string;
  build_info?: string;
}


// Debounce utility function
function useDebounce<T extends (...args: any[]) => any>(func: T, delay: number): T {
  const timeoutRef = React.useRef<NodeJS.Timeout | null>(null);

  const debouncedFunc = useCallback((...args: Parameters<T>) => {
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
    timeoutRef.current = setTimeout(() => func(...args), delay);
  }, [func, delay]) as T;

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };
  }, []);

  return debouncedFunc;
}


const SettingsPage: React.FC = () => {
  const { user: currentUser } = useAuth();
  const [tabValue, setTabValue] = useState<number>(0);
  const [settings, setSettings] = useState<Settings>({
    ocrLanguage: 'eng',
    preferredLanguages: ['eng'],
    primaryLanguage: 'eng',
    autoDetectLanguageCombination: false,
    concurrentOcrJobs: 4,
    ocrTimeoutSeconds: 300,
    maxFileSizeMb: 50,
    allowedFileTypes: ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
    autoRotateImages: true,
    enableImagePreprocessing: false,
    searchResultsPerPage: 25,
    searchSnippetLength: 200,
    fuzzySearchThreshold: 0.8,
    retentionDays: null,
    enableAutoCleanup: false,
    enableCompression: false,
    memoryLimitMb: 512,
    cpuPriority: 'normal',
    enableBackgroundOcr: true,
    ocrPageSegmentationMode: 3,
    ocrEngineMode: 3,
    ocrMinConfidence: 30.0,
    ocrDpi: 300,
    ocrEnhanceContrast: true,
    ocrRemoveNoise: true,
    ocrDetectOrientation: true,
    ocrWhitelistChars: '',
    ocrBlacklistChars: '',
    ocrBrightnessBoost: 0.0,
    ocrContrastMultiplier: 1.0,
    ocrNoiseReductionLevel: 1,
    ocrSharpeningStrength: 0.0,
    ocrMorphologicalOperations: true,
    ocrAdaptiveThresholdWindowSize: 15,
    ocrHistogramEqualization: false,
    ocrUpscaleFactor: 1.0,
    ocrMaxImageWidth: 10000,
    ocrMaxImageHeight: 10000,
    saveProcessedImages: false,
    ocrQualityThresholdBrightness: 40.0,
    ocrQualityThresholdContrast: 0.15,
    ocrQualityThresholdNoise: 0.3,
    ocrQualityThresholdSharpness: 0.15,
    ocrSkipEnhancement: false,
  });
  const [users, setUsers] = useState<User[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [snackbar, setSnackbar] = useState<SnackbarState>({ 
    open: false, 
    message: '', 
    severity: 'success' 
  });
  const [userDialog, setUserDialog] = useState<UserDialogState>({ 
    open: false, 
    mode: 'create', 
    user: null 
  });
  const [userForm, setUserForm] = useState<UserFormData>({ 
    username: '', 
    email: '', 
    password: '' 
  });
  
  // OCR Admin Controls State
  const [ocrStatus, setOcrStatus] = useState<{ is_paused: boolean; status: 'paused' | 'running' } | null>(null);
  const [ocrActionLoading, setOcrActionLoading] = useState(false);
  
  // Server Configuration State
  const [serverConfig, setServerConfig] = useState<ServerConfiguration | null>(null);
  const [configLoading, setConfigLoading] = useState(false);

  // Watch Directory State
  const [userWatchDirectories, setUserWatchDirectories] = useState<Map<string, UserWatchDirectoryResponse>>(new Map());
  const [watchDirLoading, setWatchDirLoading] = useState<Map<string, boolean>>(new Map());
  const [confirmDialog, setConfirmDialog] = useState<{
    open: boolean;
    title: string;
    message: string;
    onConfirm: () => void;
  }>({
    open: false,
    title: '',
    message: '',
    onConfirm: () => {},
  });


  useEffect(() => {
    fetchSettings();
    fetchUsers();
    fetchOcrStatus();
    fetchServerConfiguration();
  }, []);

  // Fetch watch directory information after users are loaded
  useEffect(() => {
    if (users.length > 0) {
      fetchUserWatchDirectories();
    }
  }, [users]);

  const fetchSettings = async (): Promise<void> => {
    try {
      const response = await api.get('/settings');
      setSettings({
        ocrLanguage: response.data.ocr_language || 'eng',
        preferredLanguages: response.data.preferred_languages || ['eng'],
        primaryLanguage: response.data.primary_language || 'eng',
        autoDetectLanguageCombination: response.data.auto_detect_language_combination || false,
        concurrentOcrJobs: response.data.concurrent_ocr_jobs || 4,
        ocrTimeoutSeconds: response.data.ocr_timeout_seconds || 300,
        maxFileSizeMb: response.data.max_file_size_mb || 50,
        allowedFileTypes: response.data.allowed_file_types || ['pdf', 'png', 'jpg', 'jpeg', 'tiff', 'bmp', 'txt'],
        autoRotateImages: response.data.auto_rotate_images !== undefined ? response.data.auto_rotate_images : true,
        enableImagePreprocessing: response.data.enable_image_preprocessing !== undefined ? response.data.enable_image_preprocessing : false,
        searchResultsPerPage: response.data.search_results_per_page || 25,
        searchSnippetLength: response.data.search_snippet_length || 200,
        fuzzySearchThreshold: response.data.fuzzy_search_threshold || 0.8,
        retentionDays: response.data.retention_days,
        enableAutoCleanup: response.data.enable_auto_cleanup || false,
        enableCompression: response.data.enable_compression || false,
        memoryLimitMb: response.data.memory_limit_mb || 512,
        cpuPriority: response.data.cpu_priority || 'normal',
        enableBackgroundOcr: response.data.enable_background_ocr !== undefined ? response.data.enable_background_ocr : true,
        ocrPageSegmentationMode: response.data.ocr_page_segmentation_mode || 3,
        ocrEngineMode: response.data.ocr_engine_mode || 3,
        ocrMinConfidence: response.data.ocr_min_confidence || 30.0,
        ocrDpi: response.data.ocr_dpi || 300,
        ocrEnhanceContrast: response.data.ocr_enhance_contrast !== undefined ? response.data.ocr_enhance_contrast : true,
        ocrRemoveNoise: response.data.ocr_remove_noise !== undefined ? response.data.ocr_remove_noise : true,
        ocrDetectOrientation: response.data.ocr_detect_orientation !== undefined ? response.data.ocr_detect_orientation : true,
        ocrWhitelistChars: response.data.ocr_whitelist_chars || '',
        ocrBlacklistChars: response.data.ocr_blacklist_chars || '',
        ocrBrightnessBoost: response.data.ocr_brightness_boost || 0.0,
        ocrContrastMultiplier: response.data.ocr_contrast_multiplier || 1.0,
        ocrNoiseReductionLevel: response.data.ocr_noise_reduction_level || 1,
        ocrSharpeningStrength: response.data.ocr_sharpening_strength || 0.0,
        ocrMorphologicalOperations: response.data.ocr_morphological_operations !== undefined ? response.data.ocr_morphological_operations : true,
        ocrAdaptiveThresholdWindowSize: response.data.ocr_adaptive_threshold_window_size || 15,
        ocrHistogramEqualization: response.data.ocr_histogram_equalization || false,
        ocrUpscaleFactor: response.data.ocr_upscale_factor || 1.0,
        ocrMaxImageWidth: response.data.ocr_max_image_width || 10000,
        ocrMaxImageHeight: response.data.ocr_max_image_height || 10000,
        saveProcessedImages: response.data.save_processed_images || false,
        ocrQualityThresholdBrightness: response.data.ocr_quality_threshold_brightness || 40.0,
        ocrQualityThresholdContrast: response.data.ocr_quality_threshold_contrast || 0.15,
        ocrQualityThresholdNoise: response.data.ocr_quality_threshold_noise || 0.3,
        ocrQualityThresholdSharpness: response.data.ocr_quality_threshold_sharpness || 0.15,
        ocrSkipEnhancement: response.data.ocr_skip_enhancement || false,
      });
    } catch (error: any) {
      console.error('Error fetching settings:', error);
      if (error.response?.status !== 404) {
        showSnackbar('Failed to load settings', 'error');
      }
    }
  };

  const fetchUsers = async (): Promise<void> => {
    try {
      const response = await api.get<User[]>('/users');
      setUsers(response.data);
    } catch (error: any) {
      console.error('Error fetching users:', error);
      if (error.response?.status !== 404) {
        showSnackbar('Failed to load users', 'error');
      }
    }
  };

  const handleSettingsChange = async (key: keyof Settings, value: any): Promise<void> => {
    try {
      // Convert camelCase to snake_case for API
      const snakeCase = (str: string): string => str.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
      const apiKey = snakeCase(key);
      
      // Build the update payload with only the changed field
      const updatePayload = { [apiKey]: value };
      
      await api.put('/settings', updatePayload);
      
      // Only update state after successful API call
      setSettings(prevSettings => ({ ...prevSettings, [key]: value }));
      
      // Only show success message for non-text inputs to reduce noise
      if (typeof value !== 'string') {
        showSnackbar('Settings updated successfully', 'success');
      }
    } catch (error) {
      console.error('Error updating settings:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific settings errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_INVALID_LANGUAGE)) {
        showSnackbar('Invalid language selected. Please choose from available languages.', 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_VALUE_OUT_OF_RANGE)) {
        showSnackbar(`${errorInfo.message}. ${errorInfo.suggestedAction || ''}`, 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.SETTINGS_CONFLICTING_SETTINGS)) {
        showSnackbar('Conflicting settings detected. Please review your configuration.', 'warning');
      } else {
        showSnackbar(errorInfo.message || 'Failed to update settings', 'error');
      }
    }
  };

  const handleUserSubmit = async (): Promise<void> => {
    setLoading(true);
    try {
      if (userDialog.mode === 'create') {
        await api.post('/users', userForm);
        showSnackbar('User created successfully', 'success');
      } else {
        const { password, ...updateData } = userForm;
        const payload: any = updateData;
        if (password) {
          payload.password = password;
        }
        await api.put(`/users/${userDialog.user?.id}`, payload);
        showSnackbar('User updated successfully', 'success');
      }
      fetchUsers();
      handleCloseUserDialog();
    } catch (error: any) {
      console.error('Error saving user:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific user errors with better messages
      if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_DUPLICATE_USERNAME)) {
        showSnackbar('This username is already taken. Please choose a different username.', 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_DUPLICATE_EMAIL)) {
        showSnackbar('This email address is already in use. Please use a different email.', 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_INVALID_PASSWORD)) {
        showSnackbar('Password must be at least 8 characters with uppercase, lowercase, and numbers.', 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_INVALID_EMAIL)) {
        showSnackbar('Please enter a valid email address.', 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_INVALID_USERNAME)) {
        showSnackbar('Username contains invalid characters. Please use only letters, numbers, and underscores.', 'error');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_PERMISSION_DENIED)) {
        showSnackbar('You do not have permission to perform this action.', 'error');
      } else {
        showSnackbar(errorInfo.message || 'Failed to save user', 'error');
      }
    } finally {
      setLoading(false);
    }
  };

  const handleDeleteUser = async (userId: string): Promise<void> => {
    if (userId === currentUser?.id) {
      showSnackbar('You cannot delete your own account', 'error');
      return;
    }

    if (window.confirm('Are you sure you want to delete this user?')) {
      setLoading(true);
      try {
        await api.delete(`/users/${userId}`);
        showSnackbar('User deleted successfully', 'success');
        fetchUsers();
      } catch (error) {
        console.error('Error deleting user:', error);
        
        const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
        
        // Handle specific delete errors
        if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_DELETE_RESTRICTED)) {
          showSnackbar('Cannot delete this user: They may have associated data or be the last admin.', 'error');
        } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_NOT_FOUND)) {
          showSnackbar('User not found. They may have already been deleted.', 'warning');
          fetchUsers(); // Refresh the list
        } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_PERMISSION_DENIED)) {
          showSnackbar('You do not have permission to delete users.', 'error');
        } else {
          showSnackbar(errorInfo.message || 'Failed to delete user', 'error');
        }
      } finally {
        setLoading(false);
      }
    }
  };

  const handleOpenUserDialog = (mode: 'create' | 'edit', user: User | null = null): void => {
    setUserDialog({ open: true, mode, user });
    if (mode === 'edit' && user) {
      setUserForm({ username: user.username, email: user.email, password: '' });
    } else {
      setUserForm({ username: '', email: '', password: '' });
    }
  };

  const handleCloseUserDialog = (): void => {
    setUserDialog({ open: false, mode: 'create', user: null });
    setUserForm({ username: '', email: '', password: '' });
  };

  const showSnackbar = (message: string, severity: SnackbarState['severity']): void => {
    setSnackbar({ open: true, message, severity });
  };

  const handleTabChange = (event: React.SyntheticEvent, newValue: number): void => {
    setTabValue(newValue);
  };


  const handleCpuPriorityChange = (event: SelectChangeEvent<string>): void => {
    handleSettingsChange('cpuPriority', event.target.value);
  };

  const handleResultsPerPageChange = (event: SelectChangeEvent<number>): void => {
    handleSettingsChange('searchResultsPerPage', event.target.value);
  };

  const handleLanguagesChange = (languages: string[], primary?: string) => {
    // Update multiple fields at once
    const updates = {
      preferredLanguages: languages,
      primaryLanguage: primary || languages[0] || 'eng',
      ocrLanguage: primary || languages[0] || 'eng', // Backward compatibility
    };
    
    // Update all language-related settings
    Object.entries(updates).forEach(([key, value]) => {
      handleSettingsChange(key as keyof Settings, value);
    });
  };

  const fetchOcrStatus = async (): Promise<void> => {
    try {
      const response = await queueService.getOcrStatus();
      setOcrStatus(response.data);
    } catch (error: any) {
      console.error('Error fetching OCR status:', error);
      // Don't show error for OCR status since it might not be available for non-admin users
    }
  };

  const handlePauseOcr = async (): Promise<void> => {
    setOcrActionLoading(true);
    try {
      await queueService.pauseOcr();
      showSnackbar('OCR processing paused successfully', 'success');
      fetchOcrStatus(); // Refresh status
    } catch (error: any) {
      console.error('Error pausing OCR:', error);
      if (error.response?.status === 403) {
        showSnackbar('Admin access required to pause OCR processing', 'error');
      } else {
        showSnackbar('Failed to pause OCR processing', 'error');
      }
    } finally {
      setOcrActionLoading(false);
    }
  };

  const handleResumeOcr = async (): Promise<void> => {
    setOcrActionLoading(true);
    try {
      await queueService.resumeOcr();
      showSnackbar('OCR processing resumed successfully', 'success');
      fetchOcrStatus(); // Refresh status
    } catch (error: any) {
      console.error('Error resuming OCR:', error);
      if (error.response?.status === 403) {
        showSnackbar('Admin access required to resume OCR processing', 'error');
      } else {
        showSnackbar('Failed to resume OCR processing', 'error');
      }
    } finally {
      setOcrActionLoading(false);
    }
  };

  const fetchServerConfiguration = async (): Promise<void> => {
    setConfigLoading(true);
    try {
      const response = await api.get('/settings/config');
      setServerConfig(response.data);
    } catch (error: any) {
      console.error('Error fetching server configuration:', error);
      if (error.response?.status === 403) {
        showSnackbar('Admin access required to view server configuration', 'error');
      } else if (error.response?.status !== 404) {
        showSnackbar('Failed to load server configuration', 'error');
      }
    } finally {
      setConfigLoading(false);
    }
  };

  // Watch Directory Functions
  const fetchUserWatchDirectories = async (): Promise<void> => {
    try {
      const watchDirMap = new Map<string, UserWatchDirectoryResponse>();
      
      // Fetch watch directory info for each user
      await Promise.all(
        users.map(async (user) => {
          try {
            const response = await userWatchService.getUserWatchDirectory(user.id);
            watchDirMap.set(user.id, response.data);
          } catch (error: any) {
            // If watch directory doesn't exist or user doesn't have one, that's okay
            if (error.response?.status === 404) {
              watchDirMap.set(user.id, {
                user_id: user.id,
                username: user.username,
                watch_directory_path: `./user_watch/${user.username}`,
                exists: false,
                enabled: false,
              });
            } else {
              console.error(`Error fetching watch directory for user ${user.username}:`, error);
            }
          }
        })
      );
      
      setUserWatchDirectories(watchDirMap);
    } catch (error: any) {
      console.error('Error fetching user watch directories:', error);
      // Don't show error message as this might not be available for all users
    }
  };

  const setUserWatchDirLoading = (userId: string, loading: boolean): void => {
    setWatchDirLoading(prev => {
      const newMap = new Map(prev);
      if (loading) {
        newMap.set(userId, true);
      } else {
        newMap.delete(userId);
      }
      return newMap;
    });
  };

  const handleCreateWatchDirectory = async (userId: string): Promise<void> => {
    setUserWatchDirLoading(userId, true);
    try {
      const response = await userWatchService.createUserWatchDirectory(userId);
      if (response.data.success) {
        showSnackbar('Watch directory created successfully', 'success');
        // Refresh the watch directory info for this user
        try {
          const updatedResponse = await userWatchService.getUserWatchDirectory(userId);
          setUserWatchDirectories(prev => {
            const newMap = new Map(prev);
            newMap.set(userId, updatedResponse.data);
            return newMap;
          });
        } catch (fetchError) {
          console.error('Error refreshing watch directory info:', fetchError);
        }
      } else {
        showSnackbar(response.data.message || 'Failed to create watch directory', 'error');
      }
    } catch (error: any) {
      console.error('Error creating watch directory:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      if (error.response?.status === 403) {
        showSnackbar('Admin access required to create watch directories', 'error');
      } else if (error.response?.status === 409) {
        showSnackbar('Watch directory already exists for this user', 'warning');
      } else {
        showSnackbar(errorInfo.message || 'Failed to create watch directory', 'error');
      }
    } finally {
      setUserWatchDirLoading(userId, false);
    }
  };

  const handleViewWatchDirectory = (directoryPath: string): void => {
    // For now, just show the path in a snackbar
    // In a real implementation, this could open a file explorer or navigate to a directory view
    showSnackbar(`Watch directory: ${directoryPath}`, 'info');
  };

  const handleRemoveWatchDirectory = (userId: string, username: string): void => {
    setConfirmDialog({
      open: true,
      title: 'Remove Watch Directory',
      message: `Are you sure you want to remove the watch directory for user "${username}"? This action cannot be undone and will stop monitoring their directory for new files.`,
      onConfirm: () => confirmRemoveWatchDirectory(userId),
    });
  };

  const confirmRemoveWatchDirectory = async (userId: string): Promise<void> => {
    setUserWatchDirLoading(userId, true);
    try {
      const response = await userWatchService.deleteUserWatchDirectory(userId);
      if (response.data.success) {
        showSnackbar('Watch directory removed successfully', 'success');
        // Update the watch directory info to reflect removal
        setUserWatchDirectories(prev => {
          const newMap = new Map(prev);
          const current = newMap.get(userId);
          if (current) {
            newMap.set(userId, {
              ...current,
              exists: false,
              enabled: false,
            });
          }
          return newMap;
        });
      } else {
        showSnackbar(response.data.message || 'Failed to remove watch directory', 'error');
      }
    } catch (error: any) {
      console.error('Error removing watch directory:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      if (error.response?.status === 403) {
        showSnackbar('Admin access required to remove watch directories', 'error');
      } else if (error.response?.status === 404) {
        showSnackbar('Watch directory not found or already removed', 'warning');
        // Update state to reflect that it doesn't exist
        setUserWatchDirectories(prev => {
          const newMap = new Map(prev);
          const current = newMap.get(userId);
          if (current) {
            newMap.set(userId, {
              ...current,
              exists: false,
              enabled: false,
            });
          }
          return newMap;
        });
      } else {
        showSnackbar(errorInfo.message || 'Failed to remove watch directory', 'error');
      }
    } finally {
      setUserWatchDirLoading(userId, false);
      setConfirmDialog(prev => ({ ...prev, open: false }));
    }
  };

  const handleCloseConfirmDialog = (): void => {
    setConfirmDialog(prev => ({ ...prev, open: false }));
  };

  // Helper function to render watch directory status
  const renderWatchDirectoryStatus = (userId: string, username: string) => {
    const watchDirInfo = userWatchDirectories.get(userId);
    const isLoading = watchDirLoading.get(userId) || false;

    if (isLoading) {
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <CircularProgress size={16} />
          <Typography variant="body2" color="text.secondary">
            Loading...
          </Typography>
        </Box>
      );
    }

    if (!watchDirInfo) {
      return (
        <Typography variant="body2" color="text.secondary">
          Unknown
        </Typography>
      );
    }

    const getStatusIcon = () => {
      if (watchDirInfo.exists && watchDirInfo.enabled) {
        return <CheckCircleIcon sx={{ color: 'success.main', fontSize: 16 }} />;
      } else if (watchDirInfo.exists && !watchDirInfo.enabled) {
        return <WarningIcon sx={{ color: 'warning.main', fontSize: 16 }} />;
      } else {
        return <ErrorIcon sx={{ color: 'error.main', fontSize: 16 }} />;
      }
    };

    const getStatusText = () => {
      if (watchDirInfo.exists && watchDirInfo.enabled) {
        return 'Active';
      } else if (watchDirInfo.exists && !watchDirInfo.enabled) {
        return 'Disabled';
      } else {
        return 'Not Created';
      }
    };

    const getStatusColor = (): "success" | "warning" | "error" => {
      if (watchDirInfo.exists && watchDirInfo.enabled) {
        return 'success';
      } else if (watchDirInfo.exists && !watchDirInfo.enabled) {
        return 'warning';
      } else {
        return 'error';
      }
    };

    return (
      <Box sx={{ display: 'flex', flexDirection: 'column', gap: 0.5, minWidth: { xs: '120px', sm: '160px' } }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, flexWrap: 'wrap' }}>
          <Tooltip title={`Status: ${getStatusText()}`}>
            {getStatusIcon()}
          </Tooltip>
          <Chip
            label={getStatusText()}
            size="small"
            color={getStatusColor()}
            variant="outlined"
          />
        </Box>
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: 'monospace',
            fontSize: { xs: '0.7rem', sm: '0.75rem' },
            wordBreak: 'break-all',
            display: { xs: 'none', sm: 'block' }
          }}
        >
          {watchDirInfo.watch_directory_path}
        </Typography>
        {/* Show truncated path on mobile */}
        <Typography
          variant="caption"
          color="text.secondary"
          sx={{
            fontFamily: 'monospace',
            fontSize: '0.7rem',
            display: { xs: 'block', sm: 'none' }
          }}
        >
          .../user_watch/{username}
        </Typography>
      </Box>
    );
  };

  return (
    <Container maxWidth="lg" sx={{ mt: 4, mb: 4 }}>
      <Typography variant="h4" sx={{ mb: 4 }}>
        Settings
      </Typography>

      <Paper sx={{ width: '100%' }}>
        <Tabs value={tabValue} onChange={handleTabChange} aria-label="settings tabs">
          <Tab label="General" />
          <Tab label="OCR Settings" />
          <Tab label="User Management" />
          <Tab label="Server Configuration" />
        </Tabs>

        <Box sx={{ p: 3 }}>
          {tabValue === 0 && (
            <Box>
              <Typography variant="h6" sx={{ mb: 3 }}>
                General Settings
              </Typography>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    OCR Configuration
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12}>
                      <Typography variant="body2" color="text.secondary" sx={{ mb: 2 }}>
                        Configure languages for OCR text extraction. Multiple languages help with mixed-language documents.
                      </Typography>
                      <Box sx={{ '& > div': { width: '100%' } }}>
                        <LanguageSelector
                          selectedLanguages={settings.preferredLanguages}
                          primaryLanguage={settings.primaryLanguage}
                          onLanguagesChange={handleLanguagesChange}
                          disabled={loading}
                          showPrimarySelector={true}
                        />
                      </Box>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControlLabel
                        control={
                          <Switch
                            checked={settings.autoDetectLanguageCombination}
                            onChange={(e) => handleSettingsChange('autoDetectLanguageCombination', e.target.checked)}
                            disabled={loading}
                          />
                        }
                        label="Auto-detect language combinations"
                      />
                      <Typography variant="caption" color="text.secondary" sx={{ display: 'block', mt: 0.5 }}>
                        Automatically suggest optimal language combinations based on document content analysis
                      </Typography>
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Concurrent OCR Jobs"
                        value={settings.concurrentOcrJobs}
                        onChange={(e) => handleSettingsChange('concurrentOcrJobs', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 1, max: 16 }}
                        helperText="Number of OCR jobs that can run simultaneously"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="OCR Timeout (seconds)"
                        value={settings.ocrTimeoutSeconds}
                        onChange={(e) => handleSettingsChange('ocrTimeoutSeconds', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 30, max: 3600 }}
                        helperText="Maximum time for OCR processing per file"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>CPU Priority</InputLabel>
                        <Select
                          value={settings.cpuPriority}
                          label="CPU Priority"
                          onChange={handleCpuPriorityChange}
                          disabled={loading}
                        >
                          <MenuItem value="low">Low</MenuItem>
                          <MenuItem value="normal">Normal</MenuItem>
                          <MenuItem value="high">High</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              {/* Admin OCR Controls */}
              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    <StopIcon sx={{ mr: 1, verticalAlign: 'middle' }} />
                    OCR Processing Controls (Admin Only)
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  
                  <Typography variant="body2" sx={{ mb: 2, color: 'text.secondary' }}>
                    Control OCR processing to manage CPU usage and allow users to use the application without performance impact.
                  </Typography>

                  <Grid container spacing={2} alignItems="center">
                    <Grid item xs={12} md={6}>
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
                        <Button
                          variant={ocrStatus?.is_paused ? "outlined" : "contained"}
                          color={ocrStatus?.is_paused ? "success" : "warning"}
                          startIcon={ocrActionLoading ? <CircularProgress size={16} /> : 
                                   (ocrStatus?.is_paused ? <PlayArrowIcon /> : <PauseIcon />)}
                          onClick={ocrStatus?.is_paused ? handleResumeOcr : handlePauseOcr}
                          disabled={ocrActionLoading || loading}
                          size="large"
                        >
                          {ocrActionLoading ? 'Processing...' : 
                           ocrStatus?.is_paused ? 'Resume OCR Processing' : 'Pause OCR Processing'}
                        </Button>
                      </Box>
                    </Grid>
                    
                    <Grid item xs={12} md={6}>
                      {ocrStatus && (
                        <Box>
                          <Chip
                            label={`OCR Status: ${ocrStatus.status.toUpperCase()}`}
                            color={ocrStatus.is_paused ? "warning" : "success"}
                            variant="outlined"
                            icon={ocrStatus.is_paused ? <PauseIcon /> : <PlayArrowIcon />}
                            size="medium"
                          />
                          <Typography variant="caption" sx={{ display: 'block', mt: 1, color: 'text.secondary' }}>
                            {ocrStatus.is_paused 
                              ? 'OCR processing is paused. No new jobs will be processed.' 
                              : 'OCR processing is active. Documents will be processed automatically.'}
                          </Typography>
                        </Box>
                      )}
                    </Grid>
                  </Grid>

                  {ocrStatus?.is_paused && (
                    <Alert severity="warning" sx={{ mt: 2 }}>
                      <Typography variant="body2">
                        <strong>OCR Processing Paused</strong><br />
                        New documents will not be processed for OCR text extraction until processing is resumed.
                        Users can still upload and view documents, but search functionality may be limited.
                      </Typography>
                    </Alert>
                  )}
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    File Processing
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Max File Size (MB)"
                        value={settings.maxFileSizeMb}
                        onChange={(e) => handleSettingsChange('maxFileSizeMb', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 1, max: 500 }}
                        helperText="Maximum allowed file size for uploads"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Memory Limit (MB)"
                        value={settings.memoryLimitMb}
                        onChange={(e) => handleSettingsChange('memoryLimitMb', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 128, max: 4096 }}
                        helperText="Memory limit per OCR job"
                      />
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.autoRotateImages}
                              onChange={(e) => handleSettingsChange('autoRotateImages', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Auto-rotate Images"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Automatically detect and correct image orientation
                        </Typography>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableImagePreprocessing}
                              onChange={(e) => handleSettingsChange('enableImagePreprocessing', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Image Preprocessing"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Enhance images for better OCR accuracy (deskew, denoise, contrast)
                        </Typography>
                        <Typography variant="body2" color="warning.main" sx={{ mt: 1 }}>
                          ⚠️ Warning: Enabling preprocessing can significantly alter OCR text results and may reduce accuracy for some documents
                        </Typography>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableBackgroundOcr}
                              onChange={(e) => handleSettingsChange('enableBackgroundOcr', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Background OCR"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Process OCR in the background after file upload
                        </Typography>
                      </FormControl>
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Search Configuration
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>Results Per Page</InputLabel>
                        <Select
                          value={settings.searchResultsPerPage}
                          label="Results Per Page"
                          onChange={handleResultsPerPageChange}
                          disabled={loading}
                        >
                          <MenuItem value={10}>10</MenuItem>
                          <MenuItem value={25}>25</MenuItem>
                          <MenuItem value={50}>50</MenuItem>
                          <MenuItem value={100}>100</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Snippet Length"
                        value={settings.searchSnippetLength}
                        onChange={(e) => handleSettingsChange('searchSnippetLength', parseInt(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 50, max: 500 }}
                        helperText="Characters to show in search result previews"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Fuzzy Search Threshold"
                        value={settings.fuzzySearchThreshold}
                        onChange={(e) => handleSettingsChange('fuzzySearchThreshold', parseFloat(e.target.value))}
                        disabled={loading}
                        inputProps={{ min: 0, max: 1, step: 0.1 }}
                        helperText="Tolerance for spelling mistakes (0.0-1.0)"
                      />
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Storage Management
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        type="number"
                        label="Retention Days"
                        value={settings.retentionDays || ''}
                        onChange={(e) => handleSettingsChange('retentionDays', e.target.value ? parseInt(e.target.value) : null)}
                        disabled={loading}
                        inputProps={{ min: 1 }}
                        helperText="Auto-delete documents after X days (leave empty to disable)"
                      />
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableAutoCleanup}
                              onChange={(e) => handleSettingsChange('enableAutoCleanup', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Auto Cleanup"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Automatically remove orphaned files and clean up storage
                        </Typography>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12}>
                      <FormControl sx={{ mb: 2 }}>
                        <FormControlLabel
                          control={
                            <Switch
                              checked={settings.enableCompression}
                              onChange={(e) => handleSettingsChange('enableCompression', e.target.checked)}
                              disabled={loading}
                            />
                          }
                          label="Enable Compression"
                        />
                        <Typography variant="body2" color="text.secondary">
                          Compress stored documents to save disk space
                        </Typography>
                      </FormControl>
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>
            </Box>
          )}

          {tabValue === 1 && (
            <Box>
              <Typography variant="h6" sx={{ mb: 3 }}>
                OCR Image Processing Settings
              </Typography>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Enhancement Controls
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  
                  <FormControlLabel
                    control={
                      <Switch
                        checked={settings.ocrSkipEnhancement}
                        onChange={(e) => handleSettingsChange('ocrSkipEnhancement', e.target.checked)}
                      />
                    }
                    label="Skip All Image Enhancement (Use Original Images Only)"
                    sx={{ mb: 2 }}
                  />
                  
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Brightness Boost"
                        type="number"
                        value={settings.ocrBrightnessBoost}
                        onChange={(e) => handleSettingsChange('ocrBrightnessBoost', parseFloat(e.target.value) || 0)}
                        helperText="Manual brightness adjustment (0 = auto, >0 = boost amount)"
                        inputProps={{ step: 0.1, min: 0, max: 100 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Contrast Multiplier"
                        type="number"
                        value={settings.ocrContrastMultiplier}
                        onChange={(e) => handleSettingsChange('ocrContrastMultiplier', parseFloat(e.target.value) || 1)}
                        helperText="Manual contrast adjustment (1.0 = auto, >1.0 = increase)"
                        inputProps={{ step: 0.1, min: 0.1, max: 5 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <FormControl fullWidth>
                        <InputLabel>Noise Reduction Level</InputLabel>
                        <Select
                          value={settings.ocrNoiseReductionLevel}
                          label="Noise Reduction Level"
                          onChange={(e) => handleSettingsChange('ocrNoiseReductionLevel', e.target.value as number)}
                        >
                          <MenuItem value={0}>None</MenuItem>
                          <MenuItem value={1}>Light</MenuItem>
                          <MenuItem value={2}>Moderate</MenuItem>
                          <MenuItem value={3}>Heavy</MenuItem>
                        </Select>
                      </FormControl>
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Sharpening Strength"
                        type="number"
                        value={settings.ocrSharpeningStrength}
                        onChange={(e) => handleSettingsChange('ocrSharpeningStrength', parseFloat(e.target.value) || 0)}
                        helperText="Image sharpening amount (0 = auto, >0 = manual)"
                        inputProps={{ step: 0.1, min: 0, max: 2 }}
                      />
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Quality Thresholds (when to apply enhancements)
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Brightness Threshold"
                        type="number"
                        value={settings.ocrQualityThresholdBrightness}
                        onChange={(e) => handleSettingsChange('ocrQualityThresholdBrightness', parseFloat(e.target.value) || 40)}
                        helperText="Enhance if brightness below this value (0-255)"
                        inputProps={{ step: 1, min: 0, max: 255 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Contrast Threshold"
                        type="number"
                        value={settings.ocrQualityThresholdContrast}
                        onChange={(e) => handleSettingsChange('ocrQualityThresholdContrast', parseFloat(e.target.value) || 0.15)}
                        helperText="Enhance if contrast below this value (0-1)"
                        inputProps={{ step: 0.01, min: 0, max: 1 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Noise Threshold"
                        type="number"
                        value={settings.ocrQualityThresholdNoise}
                        onChange={(e) => handleSettingsChange('ocrQualityThresholdNoise', parseFloat(e.target.value) || 0.3)}
                        helperText="Enhance if noise above this value (0-1)"
                        inputProps={{ step: 0.01, min: 0, max: 1 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Sharpness Threshold"
                        type="number"
                        value={settings.ocrQualityThresholdSharpness}
                        onChange={(e) => handleSettingsChange('ocrQualityThresholdSharpness', parseFloat(e.target.value) || 0.15)}
                        helperText="Enhance if sharpness below this value (0-1)"
                        inputProps={{ step: 0.01, min: 0, max: 1 }}
                      />
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>

              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Advanced Processing Options
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <FormControlLabel
                        control={
                          <Switch
                            checked={settings.ocrMorphologicalOperations}
                            onChange={(e) => handleSettingsChange('ocrMorphologicalOperations', e.target.checked)}
                          />
                        }
                        label="Morphological Operations (text cleanup)"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <FormControlLabel
                        control={
                          <Switch
                            checked={settings.ocrHistogramEqualization}
                            onChange={(e) => handleSettingsChange('ocrHistogramEqualization', e.target.checked)}
                          />
                        }
                        label="Histogram Equalization"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <FormControlLabel
                        control={
                          <Switch
                            checked={settings.saveProcessedImages}
                            onChange={(e) => handleSettingsChange('saveProcessedImages', e.target.checked)}
                          />
                        }
                        label="Save Processed Images for Review"
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Adaptive Threshold Window Size"
                        type="number"
                        value={settings.ocrAdaptiveThresholdWindowSize}
                        onChange={(e) => handleSettingsChange('ocrAdaptiveThresholdWindowSize', parseInt(e.target.value) || 15)}
                        helperText="Window size for contrast enhancement (odd number)"
                        inputProps={{ step: 2, min: 3, max: 101 }}
                      />
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>
              
              <Card sx={{ mb: 3 }}>
                <CardContent>
                  <Typography variant="subtitle1" sx={{ mb: 2 }}>
                    Image Size and Scaling
                  </Typography>
                  <Divider sx={{ mb: 2 }} />
                  
                  <Grid container spacing={2}>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Max Image Width"
                        type="number"
                        value={settings.ocrMaxImageWidth}
                        onChange={(e) => handleSettingsChange('ocrMaxImageWidth', parseInt(e.target.value) || 10000)}
                        helperText="Maximum image width in pixels"
                        inputProps={{ step: 100, min: 100, max: 50000 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Max Image Height"
                        type="number"
                        value={settings.ocrMaxImageHeight}
                        onChange={(e) => handleSettingsChange('ocrMaxImageHeight', parseInt(e.target.value) || 10000)}
                        helperText="Maximum image height in pixels"
                        inputProps={{ step: 100, min: 100, max: 50000 }}
                      />
                    </Grid>
                    <Grid item xs={12} md={6}>
                      <TextField
                        fullWidth
                        label="Upscale Factor"
                        type="number"
                        value={settings.ocrUpscaleFactor}
                        onChange={(e) => handleSettingsChange('ocrUpscaleFactor', parseFloat(e.target.value) || 1.0)}
                        helperText="Image scaling factor (1.0 = no scaling)"
                        inputProps={{ step: 0.1, min: 0.1, max: 5 }}
                      />
                    </Grid>
                  </Grid>
                </CardContent>
              </Card>
            </Box>
          )}

          {tabValue === 2 && (
            <Box>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', mb: 3 }}>
                <Typography variant="h6">
                  User Management
                </Typography>
                <Button
                  variant="contained"
                  startIcon={<AddIcon />}
                  onClick={() => handleOpenUserDialog('create')}
                  disabled={loading}
                >
                  Add User
                </Button>
              </Box>

              <TableContainer component={Paper} sx={{ overflowX: 'auto' }}>
                <Table sx={{ minWidth: 800 }}>
                  <TableHead>
                    <TableRow>
                      <TableCell>Username</TableCell>
                      <TableCell sx={{ display: { xs: 'none', sm: 'table-cell' } }}>Email</TableCell>
                      <TableCell sx={{ display: { xs: 'none', md: 'table-cell' } }}>Created At</TableCell>
                      <TableCell>Watch Directory</TableCell>
                      <TableCell align="right">Actions</TableCell>
                    </TableRow>
                  </TableHead>
                  <TableBody>
                    {users.map((user) => (
                      <TableRow key={user.id}>
                        <TableCell>
                          <Box>
                            <Typography variant="body2" fontWeight="medium">
                              {user.username}
                            </Typography>
                            {/* Show email on mobile */}
                            <Typography 
                              variant="caption" 
                              color="text.secondary"
                              sx={{ display: { xs: 'block', sm: 'none' } }}
                            >
                              {user.email}
                            </Typography>
                            {/* Show created date on mobile */}
                            <Typography 
                              variant="caption" 
                              color="text.secondary"
                              sx={{ display: { xs: 'block', md: 'none' } }}
                            >
                              Created: {new Date(user.created_at).toLocaleDateString()}
                            </Typography>
                          </Box>
                        </TableCell>
                        <TableCell sx={{ display: { xs: 'none', sm: 'table-cell' } }}>
                          {user.email}
                        </TableCell>
                        <TableCell sx={{ display: { xs: 'none', md: 'table-cell' } }}>
                          {new Date(user.created_at).toLocaleDateString()}
                        </TableCell>
                        <TableCell>
                          {renderWatchDirectoryStatus(user.id, user.username)}
                        </TableCell>
                        <TableCell align="right">
                          <Box sx={{ 
                            display: 'flex', 
                            gap: 0.5, 
                            justifyContent: 'flex-end',
                            flexWrap: { xs: 'wrap', sm: 'nowrap' },
                            minWidth: { xs: 'auto', sm: '200px' }
                          }}>
                            {/* Watch Directory Actions */}
                            {(() => {
                              const watchDirInfo = userWatchDirectories.get(user.id);
                              const isWatchDirLoading = watchDirLoading.get(user.id) || false;
                              
                              if (!watchDirInfo || !watchDirInfo.exists) {
                                // Show Create Directory button
                                return (
                                  <Tooltip title="Create watch directory">
                                    <IconButton
                                      onClick={() => handleCreateWatchDirectory(user.id)}
                                      disabled={loading || isWatchDirLoading}
                                      color="primary"
                                      size="small"
                                    >
                                      {isWatchDirLoading ? (
                                        <CircularProgress size={16} />
                                      ) : (
                                        <CreateNewFolderIcon />
                                      )}
                                    </IconButton>
                                  </Tooltip>
                                );
                              } else {
                                // Show View and Remove buttons
                                return (
                                  <>
                                    <Tooltip title="View watch directory">
                                      <IconButton
                                        onClick={() => handleViewWatchDirectory(watchDirInfo.watch_directory_path)}
                                        disabled={loading || isWatchDirLoading}
                                        color="info"
                                        size="small"
                                      >
                                        <VisibilityIcon />
                                      </IconButton>
                                    </Tooltip>
                                    <Tooltip title="Remove watch directory (Admin only)">
                                      <IconButton
                                        onClick={() => handleRemoveWatchDirectory(user.id, user.username)}
                                        disabled={loading || isWatchDirLoading}
                                        color="error"
                                        size="small"
                                      >
                                        {isWatchDirLoading ? (
                                          <CircularProgress size={16} />
                                        ) : (
                                          <RemoveCircleIcon />
                                        )}
                                      </IconButton>
                                    </Tooltip>
                                  </>
                                );
                              }
                            })()}
                            
                            <Divider orientation="vertical" flexItem sx={{ mx: 0.5 }} />
                            
                            {/* User Management Actions */}
                            <Tooltip title="Edit user">
                              <IconButton
                                onClick={() => handleOpenUserDialog('edit', user)}
                                disabled={loading}
                                size="small"
                              >
                                <EditIcon />
                              </IconButton>
                            </Tooltip>
                            <Tooltip title="Delete user">
                              <IconButton
                                onClick={() => handleDeleteUser(user.id)}
                                disabled={loading || user.id === currentUser?.id}
                                color="error"
                                size="small"
                              >
                                <DeleteIcon />
                              </IconButton>
                            </Tooltip>
                          </Box>
                        </TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </TableContainer>
            </Box>
          )}

          {tabValue === 3 && (
            <Box>
              <Typography variant="h6" sx={{ mb: 3 }}>
                Server Configuration (Admin Only)
              </Typography>

              {configLoading ? (
                <Box sx={{ display: 'flex', justifyContent: 'center', p: 4 }}>
                  <CircularProgress />
                </Box>
              ) : serverConfig ? (
                <>
                  <Card sx={{ mb: 3 }}>
                    <CardContent>
                      <Typography variant="subtitle1" sx={{ mb: 2 }}>
                        File Upload Configuration
                      </Typography>
                      <Divider sx={{ mb: 2 }} />
                      <Grid container spacing={2}>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Max File Size</Typography>
                          <Typography variant="h6">{serverConfig.max_file_size_mb} MB</Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Upload Path</Typography>
                          <Typography variant="body1" sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                            {serverConfig.upload_path}
                          </Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Allowed File Types</Typography>
                          <Box sx={{ mt: 1 }}>
                            {serverConfig.allowed_file_types.map((type) => (
                              <Chip key={type} label={type} size="small" sx={{ mr: 0.5, mb: 0.5 }} />
                            ))}
                          </Box>
                        </Grid>
                        {serverConfig.watch_folder && (
                          <Grid item xs={12} md={6}>
                            <Typography variant="body2" color="text.secondary">Watch Folder</Typography>
                            <Typography variant="body1" sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                              {serverConfig.watch_folder}
                            </Typography>
                          </Grid>
                        )}
                      </Grid>
                    </CardContent>
                  </Card>

                  <Card sx={{ mb: 3 }}>
                    <CardContent>
                      <Typography variant="subtitle1" sx={{ mb: 2 }}>
                        OCR Processing Configuration
                      </Typography>
                      <Divider sx={{ mb: 2 }} />
                      <Grid container spacing={2}>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Concurrent OCR Jobs</Typography>
                          <Typography variant="h6">{serverConfig.concurrent_ocr_jobs}</Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">OCR Timeout</Typography>
                          <Typography variant="h6">{serverConfig.ocr_timeout_seconds}s</Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Memory Limit</Typography>
                          <Typography variant="h6">{serverConfig.memory_limit_mb} MB</Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">OCR Language</Typography>
                          <Typography variant="h6">{serverConfig.ocr_language}</Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">CPU Priority</Typography>
                          <Typography variant="h6" sx={{ textTransform: 'capitalize' }}>
                            {serverConfig.cpu_priority}
                          </Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Background OCR</Typography>
                          <Chip 
                            label={serverConfig.enable_background_ocr ? 'Enabled' : 'Disabled'}
                            color={serverConfig.enable_background_ocr ? 'success' : 'warning'}
                            size="small"
                          />
                        </Grid>
                      </Grid>
                    </CardContent>
                  </Card>

                  <Card sx={{ mb: 3 }}>
                    <CardContent>
                      <Typography variant="subtitle1" sx={{ mb: 2 }}>
                        Server Information
                      </Typography>
                      <Divider sx={{ mb: 2 }} />
                      <Grid container spacing={2}>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Server Host</Typography>
                          <Typography variant="body1" sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                            {serverConfig.server_host}
                          </Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Server Port</Typography>
                          <Typography variant="h6">{serverConfig.server_port}</Typography>
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">JWT Secret</Typography>
                          <Chip 
                            label={serverConfig.jwt_secret_set ? 'Configured' : 'Not Set'}
                            color={serverConfig.jwt_secret_set ? 'success' : 'error'}
                            size="small"
                          />
                        </Grid>
                        <Grid item xs={12} md={6}>
                          <Typography variant="body2" color="text.secondary">Version</Typography>
                          <Typography variant="h6">{serverConfig.version}</Typography>
                        </Grid>
                        {serverConfig.build_info && (
                          <Grid item xs={12}>
                            <Typography variant="body2" color="text.secondary">Build Information</Typography>
                            <Typography variant="body1" sx={{ fontFamily: 'monospace', fontSize: '0.875rem' }}>
                              {serverConfig.build_info}
                            </Typography>
                          </Grid>
                        )}
                      </Grid>
                    </CardContent>
                  </Card>

                  {serverConfig.watch_interval_seconds && (
                    <Card sx={{ mb: 3 }}>
                      <CardContent>
                        <Typography variant="subtitle1" sx={{ mb: 2 }}>
                          Watch Folder Configuration
                        </Typography>
                        <Divider sx={{ mb: 2 }} />
                        <Grid container spacing={2}>
                          <Grid item xs={12} md={6}>
                            <Typography variant="body2" color="text.secondary">Watch Interval</Typography>
                            <Typography variant="h6">{serverConfig.watch_interval_seconds}s</Typography>
                          </Grid>
                          {serverConfig.file_stability_check_ms && (
                            <Grid item xs={12} md={6}>
                              <Typography variant="body2" color="text.secondary">File Stability Check</Typography>
                              <Typography variant="h6">{serverConfig.file_stability_check_ms}ms</Typography>
                            </Grid>
                          )}
                          {serverConfig.max_file_age_hours && (
                            <Grid item xs={12} md={6}>
                              <Typography variant="body2" color="text.secondary">Max File Age</Typography>
                              <Typography variant="h6">{serverConfig.max_file_age_hours}h</Typography>
                            </Grid>
                          )}
                        </Grid>
                      </CardContent>
                    </Card>
                  )}

                  <Box sx={{ mt: 2 }}>
                    <Button
                      variant="outlined"
                      onClick={fetchServerConfiguration}
                      startIcon={<CloudSyncIcon />}
                      disabled={configLoading}
                    >
                      Refresh Configuration
                    </Button>
                  </Box>
                </>
              ) : (
                <Alert severity="error">
                  Failed to load server configuration. Admin access may be required.
                </Alert>
              )}
            </Box>
          )}
        </Box>
      </Paper>

      <Dialog open={userDialog.open} onClose={handleCloseUserDialog} maxWidth="sm" fullWidth>
        <DialogTitle>
          {userDialog.mode === 'create' ? 'Create New User' : 'Edit User'}
        </DialogTitle>
        <DialogContent>
          <Grid container spacing={2} sx={{ mt: 1 }}>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Username"
                value={userForm.username}
                onChange={(e) => setUserForm({ ...userForm, username: e.target.value })}
                required
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label="Email"
                type="email"
                value={userForm.email}
                onChange={(e) => setUserForm({ ...userForm, email: e.target.value })}
                required
              />
            </Grid>
            <Grid item xs={12}>
              <TextField
                fullWidth
                label={userDialog.mode === 'create' ? 'Password' : 'New Password (leave empty to keep current)'}
                type="password"
                value={userForm.password}
                onChange={(e) => setUserForm({ ...userForm, password: e.target.value })}
                required={userDialog.mode === 'create'}
              />
            </Grid>
          </Grid>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleCloseUserDialog} disabled={loading}>
            Cancel
          </Button>
          <Button onClick={handleUserSubmit} variant="contained" disabled={loading}>
            {userDialog.mode === 'create' ? 'Create' : 'Update'}
          </Button>
        </DialogActions>
      </Dialog>

      {/* Confirmation Dialog for Watch Directory Actions */}
      <Dialog
        open={confirmDialog.open}
        onClose={handleCloseConfirmDialog}
        maxWidth="sm"
        fullWidth
      >
        <DialogTitle sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <WarningIcon color="warning" />
          {confirmDialog.title}
        </DialogTitle>
        <DialogContent>
          <Typography variant="body1">
            {confirmDialog.message}
          </Typography>
        </DialogContent>
        <DialogActions>
          <Button onClick={handleCloseConfirmDialog} variant="outlined">
            Cancel
          </Button>
          <Button
            onClick={() => {
              confirmDialog.onConfirm();
              handleCloseConfirmDialog();
            }}
            variant="contained"
            color="error"
            startIcon={<RemoveCircleIcon />}
          >
            Remove Directory
          </Button>
        </DialogActions>
      </Dialog>

      <Snackbar
        open={snackbar.open}
        autoHideDuration={6000}
        onClose={() => setSnackbar({ ...snackbar, open: false })}
      >
        <Alert
          onClose={() => setSnackbar({ ...snackbar, open: false })}
          severity={snackbar.severity}
          sx={{ width: '100%' }}
        >
          {snackbar.message}
        </Alert>
      </Snackbar>
    </Container>
  );
};

export default SettingsPage;