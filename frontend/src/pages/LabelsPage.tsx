import React, { useState, useEffect } from 'react';
import { useTranslation } from 'react-i18next';
import {
  Container,
  Typography,
  Button,
  Box,
  Paper,
  IconButton,
  Tooltip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  DialogContentText,
  TextField,
  InputAdornment,
  Chip,
  Alert,
  Card,
  CardContent,
  CardActions,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  Add as AddIcon,
  Edit as EditIcon,
  Delete as DeleteIcon,
  Search as SearchIcon,
  FilterList as FilterIcon,
} from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import Label, { type LabelData } from '../components/Labels/Label';
import LabelCreateDialog from '../components/Labels/LabelCreateDialog';
import { useApi } from '../hooks/useApi';
import { ErrorHelper, ErrorCodes } from '../services/api';

const LabelsPage: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const api = useApi();
  
  const [labels, setLabels] = useState<LabelData[]>([]);
  const [loading, setLoading] = useState(true);
  const [searchTerm, setSearchTerm] = useState('');
  const [showSystemLabels, setShowSystemLabels] = useState(true);
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [editingLabel, setEditingLabel] = useState<LabelData | null>(null);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [labelToDelete, setLabelToDelete] = useState<LabelData | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Fetch labels
  const fetchLabels = async () => {
    try {
      setLoading(true);
      const response = await api.get('/labels?include_counts=true');
      
      // Validate response status and data format
      if (response.status === 200 && Array.isArray(response.data)) {
        setLabels(response.data);
        setError(null);
      } else {
        console.error('Invalid response - Status:', response.status, 'Data:', response.data);
        if (!Array.isArray(response.data)) {
          setError('Received invalid data format from server');
        } else {
          setError(`Server returned unexpected response (${response.status})`);
        }
        setLabels([]); // Reset to empty array to prevent filter errors
      }
    } catch (error: any) {
      console.error('Failed to fetch labels:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific label errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_SESSION_EXPIRED) || 
          ErrorHelper.isErrorCode(error, ErrorCodes.USER_TOKEN_EXPIRED)) {
        setError('Your session has expired. Please log in again.');
        // Could trigger a redirect to login here
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.USER_PERMISSION_DENIED)) {
        setError('You do not have permission to view labels.');
      } else if (errorInfo.category === 'server') {
        setError('Server error. Please try again later.');
      } else if (errorInfo.category === 'network') {
        setError('Network error. Please check your connection and try again.');
      } else {
        setError(errorInfo.message || 'Failed to load labels. Please check your connection.');
      }
      
      setLabels([]); // Reset to empty array to prevent filter errors
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchLabels();
  }, []);

  // Filter labels based on search and system label preference
  const filteredLabels = Array.isArray(labels) ? labels.filter(label => {
    const matchesSearch = label.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
                         (label.description || '').toLowerCase().includes(searchTerm.toLowerCase());
    const matchesFilter = showSystemLabels || !label.is_system;
    return matchesSearch && matchesFilter;
  }) : [];

  // Group labels
  const systemLabels = filteredLabels.filter(label => label.is_system);
  const userLabels = filteredLabels.filter(label => !label.is_system);

  const handleCreateLabel = async (labelData: Omit<LabelData, 'id' | 'is_system'>) => {
    try {
      const response = await api.post('/labels', labelData);
      await fetchLabels(); // Refresh the list
    } catch (error) {
      console.error('Failed to create label:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific label creation errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_DUPLICATE_NAME)) {
        throw new Error('A label with this name already exists. Please choose a different name.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_INVALID_NAME)) {
        throw new Error('Label name contains invalid characters. Please use only letters, numbers, and basic punctuation.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_INVALID_COLOR)) {
        throw new Error('Invalid color format. Please use a valid hex color like #0969da.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_MAX_LABELS_REACHED)) {
        throw new Error('Maximum number of labels reached. Please delete some labels before creating new ones.');
      } else {
        throw new Error(errorInfo.message || 'Failed to create label');
      }
    }
  };

  const handleUpdateLabel = async (labelData: Omit<LabelData, 'id' | 'is_system'>) => {
    if (!editingLabel) return;
    
    try {
      await api.put(`/labels/${editingLabel.id}`, labelData);
      await fetchLabels(); // Refresh the list
      setEditingLabel(null);
    } catch (error) {
      console.error('Failed to update label:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific label update errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_NOT_FOUND)) {
        throw new Error('Label not found. It may have been deleted by another user.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_DUPLICATE_NAME)) {
        throw new Error('A label with this name already exists. Please choose a different name.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_SYSTEM_MODIFICATION)) {
        throw new Error('System labels cannot be modified. Only user-created labels can be edited.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_INVALID_NAME)) {
        throw new Error('Label name contains invalid characters. Please use only letters, numbers, and basic punctuation.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_INVALID_COLOR)) {
        throw new Error('Invalid color format. Please use a valid hex color like #0969da.');
      } else {
        throw new Error(errorInfo.message || 'Failed to update label');
      }
    }
  };

  const handleDeleteLabel = async (labelId: string) => {
    try {
      await api.delete(`/labels/${labelId}`);
      await fetchLabels(); // Refresh the list
      setDeleteDialogOpen(false);
      setLabelToDelete(null);
    } catch (error) {
      console.error('Failed to delete label:', error);
      
      const errorInfo = ErrorHelper.formatErrorForDisplay(error, true);
      
      // Handle specific label deletion errors
      if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_NOT_FOUND)) {
        setError('Label not found. It may have already been deleted.');
        await fetchLabels(); // Refresh the list to sync state
        setDeleteDialogOpen(false);
        setLabelToDelete(null);
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_IN_USE)) {
        setError('Cannot delete label because it is currently assigned to documents. Please remove the label from all documents first.');
      } else if (ErrorHelper.isErrorCode(error, ErrorCodes.LABEL_SYSTEM_MODIFICATION)) {
        setError('System labels cannot be deleted. Only user-created labels can be removed.');
      } else {
        setError(errorInfo.message || 'Failed to delete label');
      }
    }
  };

  const openDeleteDialog = (label: LabelData) => {
    setLabelToDelete(label);
    setDeleteDialogOpen(true);
  };

  const openEditDialog = (label: LabelData) => {
    setEditingLabel(label);
  };

  if (loading) {
    return (
      <Container maxWidth="lg" sx={{ py: 4 }}>
        <Typography>{t('labels.loading')}</Typography>
      </Container>
    );
  }

  return (
    <Container maxWidth="lg" sx={{ py: 4 }}>
      {/* Header */}
      <Box display="flex" justifyContent="space-between" alignItems="center" mb={4}>
        <Typography variant="h4" component="h1">
          {t('labels.title')}
        </Typography>
        <Button
          variant="contained"
          startIcon={<AddIcon />}
          onClick={() => setCreateDialogOpen(true)}
        >
          {t('labels.actions.createLabel')}
        </Button>
      </Box>

      {/* Error Alert */}
      {error && (
        <Alert severity="error" onClose={() => setError(null)} sx={{ mb: 3 }}>
          {error}
        </Alert>
      )}

      {/* Search and Filters */}
      <Paper sx={{ p: 3, mb: 3 }}>
        <Grid container spacing={2} alignItems="center">
          <Grid item xs={12} md={6}>
            <TextField
              fullWidth
              placeholder={t('labels.search.placeholder')}
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start">
                    <SearchIcon />
                  </InputAdornment>
                ),
              }}
            />
          </Grid>
          <Grid item xs={12} md={6}>
            <Box display="flex" gap={1} flexWrap="wrap">
              <Chip
                label={t('labels.filters.systemLabels')}
                color={showSystemLabels ? 'primary' : 'default'}
                onClick={() => setShowSystemLabels(!showSystemLabels)}
                variant={showSystemLabels ? 'filled' : 'outlined'}
              />
            </Box>
          </Grid>
        </Grid>
      </Paper>

      {/* Labels List */}
      <Box>
        {/* System Labels */}
        {systemLabels.length > 0 && (
          <Box mb={4}>
            <Typography variant="h6" gutterBottom color="text.secondary">
              {t('labels.sections.systemLabels')}
            </Typography>
            <Grid container spacing={2}>
              {systemLabels.map((label) => (
                <Grid item xs={12} sm={6} md={4} key={label.id}>
                  <Card>
                    <CardContent>
                      <Box display="flex" justifyContent="space-between" alignItems="flex-start" mb={2}>
                        <Label label={label} showCount />
                        <Typography variant="caption" color="text.secondary">
                          {t('labels.badge.system')}
                        </Typography>
                      </Box>
                      {label.description && (
                        <Typography variant="body2" color="text.secondary">
                          {label.description}
                        </Typography>
                      )}
                      <Box mt={2} display="flex" gap={2}>
                        <Typography variant="caption" color="text.secondary">
                          {t('labels.stats.documents', { count: label.document_count || 0 })}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {t('labels.stats.sources', { count: label.source_count || 0 })}
                        </Typography>
                      </Box>
                    </CardContent>
                  </Card>
                </Grid>
              ))}
            </Grid>
          </Box>
        )}

        {/* User Labels */}
        {userLabels.length > 0 && (
          <Box>
            <Typography variant="h6" gutterBottom>
              {t('labels.sections.myLabels')}
            </Typography>
            <Grid container spacing={2}>
              {userLabels.map((label) => (
                <Grid item xs={12} sm={6} md={4} key={label.id}>
                  <Card>
                    <CardContent>
                      <Box display="flex" justifyContent="space-between" alignItems="flex-start" mb={2}>
                        <Label label={label} showCount />
                        <Box>
                          <Tooltip title={t('labels.actions.editLabel')}>
                            <IconButton
                              size="small"
                              onClick={() => openEditDialog(label)}
                            >
                              <EditIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title={t('labels.actions.deleteLabel')}>
                            <IconButton
                              size="small"
                              onClick={() => openDeleteDialog(label)}
                              color="error"
                            >
                              <DeleteIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      </Box>
                      {label.description && (
                        <Typography variant="body2" color="text.secondary">
                          {label.description}
                        </Typography>
                      )}
                      <Box mt={2} display="flex" gap={2}>
                        <Typography variant="caption" color="text.secondary">
                          {t('labels.stats.documents', { count: label.document_count || 0 })}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {t('labels.stats.sources', { count: label.source_count || 0 })}
                        </Typography>
                      </Box>
                    </CardContent>
                  </Card>
                </Grid>
              ))}
            </Grid>
          </Box>
        )}

        {/* Empty State */}
        {filteredLabels.length === 0 && (
          <Paper sx={{ p: 4, textAlign: 'center' }}>
            <Typography variant="h6" color="text.secondary" gutterBottom>
              {t('labels.empty.title')}
            </Typography>
            <Typography variant="body2" color="text.secondary" mb={3}>
              {searchTerm
                ? t('labels.empty.noMatch', { query: searchTerm })
                : t('labels.empty.noLabels')
              }
            </Typography>
            {!searchTerm && (
              <Button
                variant="contained"
                startIcon={<AddIcon />}
                onClick={() => setCreateDialogOpen(true)}
              >
                {t('labels.empty.createFirst')}
              </Button>
            )}
          </Paper>
        )}
      </Box>

      {/* Create/Edit Label Dialog */}
      <LabelCreateDialog
        open={createDialogOpen || !!editingLabel}
        onClose={() => {
          setCreateDialogOpen(false);
          setEditingLabel(null);
        }}
        onSubmit={editingLabel ? handleUpdateLabel : handleCreateLabel}
        editingLabel={editingLabel || undefined}
      />

      {/* Delete Confirmation Dialog */}
      <Dialog
        open={deleteDialogOpen}
        onClose={() => {
          setDeleteDialogOpen(false);
          setLabelToDelete(null);
        }}
      >
        <DialogTitle>{t('labels.dialogs.delete.title')}</DialogTitle>
        <DialogContent>
          <DialogContentText>
            {t('labels.dialogs.delete.message', { name: labelToDelete?.name })}
            {(labelToDelete?.document_count || 0) > 0 && (
              <>{t('labels.dialogs.delete.inUseWarning', { count: labelToDelete?.document_count })}</>
            )}
          </DialogContentText>
        </DialogContent>
        <DialogActions>
          <Button onClick={() => {
            setDeleteDialogOpen(false);
            setLabelToDelete(null);
          }}>
            {t('common.actions.cancel')}
          </Button>
          <Button
            onClick={() => labelToDelete && handleDeleteLabel(labelToDelete.id)}
            color="error"
            variant="contained"
          >
            {t('common.actions.delete')}
          </Button>
        </DialogActions>
      </Dialog>
    </Container>
  );
};

export default LabelsPage;