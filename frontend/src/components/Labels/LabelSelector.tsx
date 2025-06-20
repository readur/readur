import React, { useState, useEffect, useMemo } from 'react';
import {
  Autocomplete,
  TextField,
  Chip,
  Box,
  Paper,
  Typography,
  Divider,
  IconButton,
  Tooltip,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Button,
} from '@mui/material';
import { Add as AddIcon, Edit as EditIcon } from '@mui/icons-material';
import Label, { type LabelData } from './Label';
import LabelCreateDialog from './LabelCreateDialog';

interface LabelSelectorProps {
  selectedLabels: LabelData[];
  availableLabels: LabelData[];
  onLabelsChange: (labels: LabelData[]) => void;
  onCreateLabel?: (labelData: Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>) => Promise<LabelData>;
  placeholder?: string;
  size?: 'small' | 'medium';
  disabled?: boolean;
  multiple?: boolean;
  showCreateButton?: boolean;
  maxTags?: number;
}

const LabelSelector: React.FC<LabelSelectorProps> = ({
  selectedLabels,
  availableLabels,
  onLabelsChange,
  onCreateLabel,
  placeholder = "Search or create labels...",
  size = 'medium',
  disabled = false,
  multiple = true,
  showCreateButton = true,
  maxTags
}) => {
  const [inputValue, setInputValue] = useState('');
  const [createDialogOpen, setCreateDialogOpen] = useState(false);
  const [prefilledName, setPrefilledName] = useState('');

  // Memoize filtered options for performance
  const filteredOptions = useMemo(() => {
    const selectedIds = new Set(selectedLabels.map(label => label.id));
    return availableLabels.filter(label => !selectedIds.has(label.id));
  }, [availableLabels, selectedLabels]);

  // Group options by system vs user labels
  const groupedOptions = useMemo(() => {
    const systemLabels = filteredOptions.filter(label => label.is_system);
    const userLabels = filteredOptions.filter(label => !label.is_system);
    
    return [
      ...(systemLabels.length > 0 ? [{ group: 'System Labels', options: systemLabels }] : []),
      ...(userLabels.length > 0 ? [{ group: 'My Labels', options: userLabels }] : [])
    ];
  }, [filteredOptions]);

  const handleLabelChange = (event: any, newValue: LabelData | LabelData[] | null) => {
    if (!multiple) {
      onLabelsChange(newValue ? [newValue as LabelData] : []);
      return;
    }

    const newLabels = newValue as LabelData[] || [];
    
    // Check max tags limit
    if (maxTags && newLabels.length > maxTags) {
      return;
    }
    
    onLabelsChange(newLabels);
  };

  const handleCreateNew = () => {
    setPrefilledName(inputValue);
    setCreateDialogOpen(true);
  };

  const handleCreateLabel = async (labelData: Omit<LabelData, 'id' | 'is_system' | 'created_at' | 'updated_at' | 'document_count' | 'source_count'>) => {
    if (onCreateLabel) {
      try {
        const newLabel = await onCreateLabel(labelData);
        onLabelsChange([...selectedLabels, newLabel]);
        setCreateDialogOpen(false);
        setInputValue('');
        setPrefilledName('');
      } catch (error) {
        console.error('Failed to create label:', error);
      }
    }
  };

  const canCreateNew = inputValue.trim() && 
    !availableLabels.some(label => 
      label.name.toLowerCase() === inputValue.trim().toLowerCase()
    ) && 
    onCreateLabel && 
    showCreateButton;

  return (
    <>
      <Autocomplete<LabelData, boolean, false, false>
        multiple={multiple}
        value={multiple ? selectedLabels : selectedLabels[0] || null}
        onChange={handleLabelChange}
        inputValue={inputValue}
        onInputChange={(event, newInputValue) => setInputValue(newInputValue)}
        options={filteredOptions}
        groupBy={(option: LabelData) => option.is_system ? 'System Labels' : 'My Labels'}
        getOptionLabel={(option: LabelData) => option.name}
        isOptionEqualToValue={(option: LabelData, value: LabelData) => option.id === value.id}
        disabled={disabled}
        size={size}
        renderInput={(params) => (
          <TextField
            {...params}
            placeholder={selectedLabels.length === 0 ? placeholder : ''}
            InputProps={{
              ...params.InputProps,
              endAdornment: (
                <>
                  {canCreateNew && (
                    <Tooltip title={`Create label "${inputValue}"`}>
                      <IconButton
                        size="small"
                        onClick={handleCreateNew}
                        sx={{ mr: 1 }}
                      >
                        <AddIcon fontSize="small" />
                      </IconButton>
                    </Tooltip>
                  )}
                  {params.InputProps.endAdornment}
                </>
              ),
            }}
          />
        )}
        renderTags={(tagValue, getTagProps) =>
          tagValue.map((option, index) => {
            const tagProps = getTagProps({ index });
            const { key, ...restTagProps } = tagProps;
            return (
              <Label
                key={option.id}
                label={option}
                size="small"
                deletable={!disabled}
                onDelete={() => {
                  const newLabels = tagValue.filter((_, i) => i !== index);
                  onLabelsChange(newLabels);
                }}
                {...restTagProps}
              />
            );
          })
        }
        renderOption={(props, option, { selected }) => {
          const { key, ...restProps } = props;
          return (
            <Box component="li" key={option.id} {...restProps}>
              <Label
                label={option}
                size="small"
                showCount
                variant={selected ? 'filled' : 'outlined'}
              />
            </Box>
          );
        }}
        renderGroup={(params) => (
          <Box key={params.key}>
            <Typography
              variant="caption"
              sx={{
                px: 2,
                py: 1,
                color: 'text.secondary',
                fontWeight: 600,
                textTransform: 'uppercase',
                letterSpacing: '0.5px'
              }}
            >
              {params.group}
            </Typography>
            <Box>{params.children}</Box>
            {params.group === 'System Labels' && <Divider sx={{ my: 1 }} />}
          </Box>
        )}
        PaperComponent={({ children, ...paperProps }) => (
          <Paper {...paperProps}>
            {children}
            {canCreateNew && (
              <>
                <Divider />
                <Box
                  sx={{
                    p: 1,
                    cursor: 'pointer',
                    '&:hover': { backgroundColor: 'action.hover' }
                  }}
                  onClick={handleCreateNew}
                >
                  <Box display="flex" alignItems="center" gap={1}>
                    <AddIcon fontSize="small" color="primary" />
                    <Typography variant="body2" color="primary">
                      Create "{inputValue}"
                    </Typography>
                  </Box>
                </Box>
              </>
            )}
          </Paper>
        )}
        noOptionsText={
          inputValue.trim() ? (
            canCreateNew ? (
              <Box>
                <Typography variant="body2" color="text.secondary">
                  No labels found
                </Typography>
                <Button
                  startIcon={<AddIcon />}
                  onClick={handleCreateNew}
                  size="small"
                  sx={{ mt: 1 }}
                >
                  Create "{inputValue}"
                </Button>
              </Box>
            ) : (
              `No labels match "${inputValue}"`
            )
          ) : 'No labels available'
        }
        filterOptions={(options, { inputValue }) => {
          if (!inputValue) return options;
          
          return options.filter(option =>
            option.name.toLowerCase().includes(inputValue.toLowerCase()) ||
            (option.description && option.description.toLowerCase().includes(inputValue.toLowerCase()))
          );
        }}
      />

      {onCreateLabel && (
        <LabelCreateDialog
          open={createDialogOpen}
          onClose={() => {
            setCreateDialogOpen(false);
            setPrefilledName('');
          }}
          onSubmit={handleCreateLabel}
          prefilledName={prefilledName}
        />
      )}
    </>
  );
};

export default LabelSelector;