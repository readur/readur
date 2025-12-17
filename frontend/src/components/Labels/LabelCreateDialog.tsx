import React, { useState, useEffect } from 'react';
import {
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
  TextField,
  Box,
  Typography,
  IconButton,
  Paper,
  Tooltip,
} from '@mui/material';
import axios from 'axios';
import Grid from '@mui/material/GridLegacy';
import {
  Star as StarIcon,
  Archive as ArchiveIcon,
  Person as PersonIcon,
  Work as WorkIcon,
  Receipt as ReceiptIcon,
  Scale as ScaleIcon,
  LocalHospital as MedicalIcon,
  AttachMoney as DollarIcon,
  BusinessCenter as BriefcaseIcon,
  Description as DocumentIcon,
  Label as LabelIcon,
  BugReport as BugIcon,
  Build as BuildIcon,
  Folder as FolderIcon,
  Assignment as AssignmentIcon,
  Schedule as ScheduleIcon,
} from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import Label, { type LabelData } from './Label';

interface LabelCreateDialogProps {
  open: boolean;
  onClose: () => void;
  onSubmit: (labelData: Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>) => Promise<void>;
  prefilledName?: string;
  editingLabel?: LabelData;
}

const availableIcons = [
  { name: 'star', icon: StarIcon, label: 'Star' },
  { name: 'archive', icon: ArchiveIcon, label: 'Archive' },
  { name: 'person', icon: PersonIcon, label: 'Person' },
  { name: 'work', icon: WorkIcon, label: 'Work' },
  { name: 'briefcase', icon: BriefcaseIcon, label: 'Briefcase' },
  { name: 'receipt', icon: ReceiptIcon, label: 'Receipt' },
  { name: 'scale', icon: ScaleIcon, label: 'Legal' },
  { name: 'medical', icon: MedicalIcon, label: 'Medical' },
  { name: 'dollar', icon: DollarIcon, label: 'Money' },
  { name: 'document', icon: DocumentIcon, label: 'Document' },
  { name: 'label', icon: LabelIcon, label: 'Label' },
  { name: 'bug', icon: BugIcon, label: 'Bug' },
  { name: 'build', icon: BuildIcon, label: 'Build' },
  { name: 'folder', icon: FolderIcon, label: 'Folder' },
  { name: 'assignment', icon: AssignmentIcon, label: 'Assignment' },
  { name: 'schedule', icon: ScheduleIcon, label: 'Schedule' },
];

const predefinedColors = [
  '#0969da', // GitHub blue
  '#d73a49', // GitHub red
  '#28a745', // GitHub green
  '#ffd33d', // GitHub yellow
  '#8250df', // GitHub purple
  '#fd7e14', // Orange
  '#20c997', // Teal
  '#6f42c1', // Indigo
  '#e83e8c', // Pink
  '#6c757d', // Gray
];

const LabelCreateDialog: React.FC<LabelCreateDialogProps> = ({
  open,
  onClose,
  onSubmit,
  prefilledName = '',
  editingLabel
}) => {
  const { t } = useTranslation();
  const [formData, setFormData] = useState({
    name: '',
    description: '',
    color: '#0969da',
    background_color: '',
    icon: '',
  });
  const [loading, setLoading] = useState(false);
  const [nameError, setNameError] = useState('');

  useEffect(() => {
    if (editingLabel) {
      setFormData({
        name: editingLabel.name,
        description: editingLabel.description || '',
        color: editingLabel.color,
        background_color: editingLabel.background_color || '',
        icon: editingLabel.icon || '',
      });
    } else {
      setFormData({
        name: prefilledName,
        description: '',
        color: '#0969da',
        background_color: '',
        icon: '',
      });
    }
    setNameError('');
  }, [editingLabel, prefilledName, open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!formData.name.trim()) {
      setNameError(t('labels.errors.invalidName'));
      return;
    }

    // Disallow commas in label names (breaks comma-separated search filters)
    // Also check for URL-encoded commas (%2c) which could cause issues in query parameters
    if (formData.name.includes(',') || formData.name.toLowerCase().includes('%2c')) {
      setNameError(t('labels.errors.commaNotAllowed'));
      return;
    }

    setLoading(true);
    try {
      await onSubmit({
        name: formData.name.trim(),
        description: formData.description.trim() || undefined,
        color: formData.color,
        background_color: formData.background_color || undefined,
        icon: formData.icon || undefined,
      });
      // Call onClose directly after successful submission
      // Don't use handleClose() here to avoid race conditions with loading state
      onClose();
    } catch (error) {
      console.error('Failed to save label:', error);
      // Extract error message from backend JSON response
      if (axios.isAxiosError(error) && error.response?.data?.error) {
        setNameError(error.response.data.error);
      } else if (error instanceof Error) {
        setNameError(error.message);
      } else {
        setNameError(t('labels.errors.serverError'));
      }
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    if (!loading) {
      onClose();
    }
  };

  const previewLabel: LabelData = {
    id: 'preview',
    name: formData.name || 'Label Preview',
    description: formData.description,
    color: formData.color,
    background_color: formData.background_color,
    icon: formData.icon,
    is_system: false,
    created_at: new Date().toISOString(),
    updated_at: new Date().toISOString(),
    document_count: 0,
    source_count: 0,
  };

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      maxWidth="sm"
      fullWidth
      PaperProps={{
        component: 'form',
        onSubmit: handleSubmit,
      }}
    >
      <DialogTitle>
        {editingLabel ? t('labels.create.editTitle') : t('labels.create.title')}
      </DialogTitle>

      <DialogContent>
        <Grid container spacing={3} sx={{ mt: 0.5 }}>
          {/* Name Field */}
          <Grid item xs={12}>
            <TextField
              label={t('labels.create.nameLabel')}
              value={formData.name}
              onChange={(e) => {
                setFormData({ ...formData, name: e.target.value });
                if (nameError) setNameError('');
              }}
              error={!!nameError}
              helperText={nameError}
              fullWidth
              required
              autoFocus
              disabled={loading}
            />
          </Grid>

          {/* Description Field */}
          <Grid item xs={12}>
            <TextField
              label={t('labels.create.descriptionLabel')}
              value={formData.description}
              onChange={(e) => setFormData({ ...formData, description: e.target.value })}
              fullWidth
              multiline
              rows={2}
              disabled={loading}
            />
          </Grid>

          {/* Color Selection */}
          <Grid item xs={12}>
            <Typography variant="subtitle2" gutterBottom>
              {t('labels.create.colorLabel')}
            </Typography>
            <Box display="flex" flexWrap="wrap" gap={1} mb={2}>
              {predefinedColors.map((color) => (
                <IconButton
                  key={color}
                  onClick={() => setFormData({ ...formData, color })}
                  disabled={loading}
                  sx={{
                    width: 32,
                    height: 32,
                    backgroundColor: color,
                    border: formData.color === color ? '3px solid' : '1px solid',
                    borderColor: formData.color === color ? 'primary.main' : 'divider',
                    '&:hover': {
                      backgroundColor: color,
                      opacity: 0.8,
                    },
                  }}
                />
              ))}
            </Box>
            <TextField
              label={t('labels.create.customColorLabel')}
              value={formData.color}
              onChange={(e) => setFormData({ ...formData, color: e.target.value })}
              size="small"
              disabled={loading}
              InputProps={{
                startAdornment: (
                  <Box
                    sx={{
                      width: 20,
                      height: 20,
                      backgroundColor: formData.color,
                      border: '1px solid',
                      borderColor: 'divider',
                      borderRadius: 0.5,
                      mr: 1,
                    }}
                  />
                ),
              }}
            />
          </Grid>

          {/* Icon Selection */}
          <Grid item xs={12}>
            <Typography variant="subtitle2" gutterBottom>
              {t('labels.create.iconLabel')}
            </Typography>
            <Box display="flex" flexWrap="wrap" gap={1}>
              <IconButton
                onClick={() => setFormData({ ...formData, icon: '' })}
                disabled={loading}
                sx={{
                  border: '1px solid',
                  borderColor: !formData.icon ? 'primary.main' : 'divider',
                  backgroundColor: !formData.icon ? 'action.selected' : 'transparent',
                }}
              >
                <Typography variant="caption">{t('labels.create.iconNone')}</Typography>
              </IconButton>
              {availableIcons.map((iconData) => {
                const IconComponent = iconData.icon;
                return (
                  <Tooltip key={iconData.name} title={iconData.label}>
                    <IconButton
                      onClick={() => setFormData({ ...formData, icon: iconData.name })}
                      disabled={loading}
                      sx={{
                        border: '1px solid',
                        borderColor: formData.icon === iconData.name ? 'primary.main' : 'divider',
                        backgroundColor: formData.icon === iconData.name ? 'action.selected' : 'transparent',
                      }}
                    >
                      <IconComponent fontSize="small" />
                    </IconButton>
                  </Tooltip>
                );
              })}
            </Box>
          </Grid>

          {/* Preview */}
          <Grid item xs={12}>
            <Typography variant="subtitle2" gutterBottom>
              {t('labels.create.previewLabel')}
            </Typography>
            <Paper sx={{ p: 2, backgroundColor: 'grey.50' }}>
              <Box display="flex" gap={1} flexWrap="wrap">
                <Label label={previewLabel} variant="filled" />
                <Label label={previewLabel} variant="outlined" />
              </Box>
            </Paper>
          </Grid>
        </Grid>
      </DialogContent>

      <DialogActions>
        <Button onClick={handleClose} disabled={loading}>
          {t('labels.create.cancel')}
        </Button>
        <Button
          type="submit"
          variant="contained"
          disabled={loading || !formData.name.trim()}
        >
          {loading ? t('labels.create.saving') : (editingLabel ? t('labels.create.update') : t('labels.create.create'))}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

export default LabelCreateDialog;