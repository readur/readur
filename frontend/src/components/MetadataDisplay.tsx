import React, { useState } from 'react';
import {
  Box,
  Typography,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Chip,
  Grid,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Security as PermissionsIcon,
  Person as OwnerIcon,
  Group as GroupIcon,
  Storage as StorageIcon,
  Info as InfoIcon,
} from '@mui/icons-material';

interface MetadataDisplayProps {
  metadata: any;
  title?: string;
  compact?: boolean;
}

const MetadataDisplay: React.FC<MetadataDisplayProps> = ({
  metadata,
  title = "Source Metadata",
  compact = false,
}) => {
  const [expanded, setExpanded] = useState(!compact);

  if (!metadata || Object.keys(metadata).length === 0) {
    return null;
  }

  const formatValue = (key: string, value: any): React.ReactNode => {
    // Handle special metadata fields with better formatting
    if (key === 'permissions' && typeof value === 'number') {
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <PermissionsIcon sx={{ fontSize: 16, color: 'primary.main' }} />
          <Typography variant="body2" component="span">
            {value.toString(8)} (octal)
          </Typography>
        </Box>
      );
    }

    if (key === 'owner' || key === 'uid') {
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <OwnerIcon sx={{ fontSize: 16, color: 'primary.main' }} />
          <Typography variant="body2" component="span">
            {value}
          </Typography>
        </Box>
      );
    }

    if (key === 'group' || key === 'gid') {
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <GroupIcon sx={{ fontSize: 16, color: 'primary.main' }} />
          <Typography variant="body2" component="span">
            {value}
          </Typography>
        </Box>
      );
    }

    if (key === 'storage_class' || key === 'region') {
      return (
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
          <StorageIcon sx={{ fontSize: 16, color: 'primary.main' }} />
          <Typography variant="body2" component="span">
            {value}
          </Typography>
        </Box>
      );
    }

    // Handle arrays
    if (Array.isArray(value)) {
      return (
        <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
          {value.map((item, index) => (
            <Chip
              key={index}
              label={String(item)}
              size="small"
              variant="outlined"
            />
          ))}
        </Box>
      );
    }

    // Handle objects
    if (typeof value === 'object' && value !== null) {
      return (
        <Box sx={{ 
          backgroundColor: 'grey.100', 
          p: 1, 
          borderRadius: 1,
          fontFamily: 'monospace',
          fontSize: '0.75rem',
          maxHeight: '100px',
          overflow: 'auto'
        }}>
          <pre style={{ margin: 0, whiteSpace: 'pre-wrap' }}>
            {JSON.stringify(value, null, 2)}
          </pre>
        </Box>
      );
    }

    // Handle boolean values
    if (typeof value === 'boolean') {
      return (
        <Chip
          label={value ? 'Yes' : 'No'}
          color={value ? 'success' : 'default'}
          size="small"
          variant="outlined"
        />
      );
    }

    // Handle dates
    if (typeof value === 'string' && (
      key.includes('date') || 
      key.includes('time') || 
      key.includes('created') || 
      key.includes('modified')
    )) {
      try {
        const date = new Date(value);
        if (!isNaN(date.getTime())) {
          return (
            <Typography variant="body2" component="span">
              {date.toLocaleString()}
            </Typography>
          );
        }
      } catch {
        // Fall through to default handling
      }
    }

    // Default: display as string
    return (
      <Typography variant="body2" component="span">
        {String(value)}
      </Typography>
    );
  };

  const formatKeyName = (key: string): string => {
    // Convert snake_case and camelCase to Title Case
    return key
      .replace(/([a-z])([A-Z])/g, '$1 $2') // camelCase to spaces
      .replace(/_/g, ' ') // snake_case to spaces
      .replace(/\b\w/g, (letter) => letter.toUpperCase()); // Title Case
  };

  const renderMetadata = () => {
    return (
      <Grid container spacing={2}>
        {Object.entries(metadata).map(([key, value]) => (
          <Grid item xs={12} sm={6} key={key}>
            <Box sx={{ mb: 1 }}>
              <Typography 
                variant="caption" 
                color="text.secondary" 
                sx={{ fontWeight: 600, textTransform: 'uppercase', letterSpacing: 0.5 }}
              >
                {formatKeyName(key)}
              </Typography>
            </Box>
            <Box sx={{ pl: 1 }}>
              {formatValue(key, value)}
            </Box>
          </Grid>
        ))}
      </Grid>
    );
  };

  if (compact) {
    return (
      <Accordion expanded={expanded} onChange={(_, isExpanded) => setExpanded(isExpanded)}>
        <AccordionSummary
          expandIcon={<ExpandMoreIcon />}
          sx={{ 
            backgroundColor: 'grey.50',
            '&:hover': { backgroundColor: 'grey.100' }
          }}
        >
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <InfoIcon sx={{ fontSize: 20, color: 'primary.main' }} />
            <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
              {title}
            </Typography>
            <Chip 
              label={`${Object.keys(metadata).length} fields`} 
              size="small" 
              variant="outlined" 
            />
          </Box>
        </AccordionSummary>
        <AccordionDetails>
          {renderMetadata()}
        </AccordionDetails>
      </Accordion>
    );
  }

  return (
    <Box>
      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 2 }}>
        <InfoIcon sx={{ color: 'primary.main' }} />
        <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
          {title}
        </Typography>
      </Box>
      {renderMetadata()}
    </Box>
  );
};

export default MetadataDisplay;