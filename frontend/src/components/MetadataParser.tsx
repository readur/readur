import React from 'react';
import {
  Box,
  Typography,
  Chip,
  Stack,
  Paper,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Divider,
  IconButton,
  Tooltip,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  ExpandMore as ExpandMoreIcon,
  PhotoCamera as CameraIcon,
  LocationOn as LocationIcon,
  DateRange as DateIcon,
  Settings as SettingsIcon,
  AspectRatio as AspectRatioIcon,
  ColorLens as ColorIcon,
  Copyright as CopyrightIcon,
  Person as PersonIcon,
  Business as BusinessIcon,
  FileCopy as DocumentIcon,
  ContentCopy as CopyIcon,
} from '@mui/icons-material';
import { modernTokens } from '../theme';

// Define border radius values since they might not be in modernTokens
const borderRadius = {
  sm: 4,
  md: 8,
  lg: 12,
  xl: 16,
};

interface MetadataParserProps {
  metadata: Record<string, any>;
  fileType: string;
  compact?: boolean;
}

interface ParsedMetadata {
  category: string;
  icon: React.ReactElement;
  items: Array<{
    label: string;
    value: any;
    type: 'text' | 'date' | 'location' | 'technical' | 'copyable';
    unit?: string;
  }>;
}

const MetadataParser: React.FC<MetadataParserProps> = ({ 
  metadata, 
  fileType, 
  compact = false 
}) => {
  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const parseExifData = (exif: Record<string, any>): ParsedMetadata[] => {
    const sections: ParsedMetadata[] = [];

    // Camera Information
    const cameraInfo = [];
    if (exif.make) cameraInfo.push({ label: 'Camera Make', value: exif.make, type: 'text' as const });
    if (exif.model) cameraInfo.push({ label: 'Camera Model', value: exif.model, type: 'text' as const });
    if (exif.lens_make) cameraInfo.push({ label: 'Lens Make', value: exif.lens_make, type: 'text' as const });
    if (exif.lens_model) cameraInfo.push({ label: 'Lens Model', value: exif.lens_model, type: 'text' as const });

    if (cameraInfo.length > 0) {
      sections.push({
        category: 'Camera',
        icon: <CameraIcon />,
        items: cameraInfo,
      });
    }

    // Technical Settings
    const technicalInfo = [];
    if (exif.focal_length) technicalInfo.push({ label: 'Focal Length', value: exif.focal_length, type: 'technical' as const, unit: 'mm' });
    if (exif.aperture) technicalInfo.push({ label: 'Aperture', value: `f/${exif.aperture}`, type: 'technical' as const });
    if (exif.exposure_time) technicalInfo.push({ label: 'Shutter Speed', value: exif.exposure_time, type: 'technical' as const, unit: 's' });
    if (exif.iso) technicalInfo.push({ label: 'ISO', value: exif.iso, type: 'technical' as const });
    if (exif.flash) technicalInfo.push({ label: 'Flash', value: exif.flash, type: 'text' as const });

    if (technicalInfo.length > 0) {
      sections.push({
        category: 'Camera Settings',
        icon: <SettingsIcon />,
        items: technicalInfo,
      });
    }

    // Image Properties
    const imageInfo = [];
    if (exif.width && exif.height) {
      imageInfo.push({ 
        label: 'Dimensions', 
        value: `${exif.width} × ${exif.height}`, 
        type: 'technical' as const,
        unit: 'px'
      });
    }
    if (exif.resolution_x && exif.resolution_y) {
      imageInfo.push({ 
        label: 'Resolution', 
        value: `${exif.resolution_x} × ${exif.resolution_y}`, 
        type: 'technical' as const,
        unit: 'dpi'
      });
    }
    if (exif.color_space) imageInfo.push({ label: 'Color Space', value: exif.color_space, type: 'text' as const });
    if (exif.orientation) imageInfo.push({ label: 'Orientation', value: exif.orientation, type: 'text' as const });

    if (imageInfo.length > 0) {
      sections.push({
        category: 'Image Properties',
        icon: <AspectRatioIcon />,
        items: imageInfo,
      });
    }

    // Location Data
    if (exif.gps_latitude && exif.gps_longitude) {
      sections.push({
        category: 'Location',
        icon: <LocationIcon />,
        items: [
          { 
            label: 'Coordinates', 
            value: `${exif.gps_latitude}, ${exif.gps_longitude}`, 
            type: 'location' as const 
          },
          ...(exif.gps_altitude ? [{ 
            label: 'Altitude', 
            value: exif.gps_altitude, 
            type: 'technical' as const,
            unit: 'm'
          }] : []),
        ],
      });
    }

    // Timestamps
    const dateInfo = [];
    if (exif.date_time_original) dateInfo.push({ label: 'Date Taken', value: exif.date_time_original, type: 'date' as const });
    if (exif.date_time_digitized) dateInfo.push({ label: 'Date Digitized', value: exif.date_time_digitized, type: 'date' as const });

    if (dateInfo.length > 0) {
      sections.push({
        category: 'Timestamps',
        icon: <DateIcon />,
        items: dateInfo,
      });
    }

    return sections;
  };

  const parsePdfMetadata = (pdf: Record<string, any>): ParsedMetadata[] => {
    const sections: ParsedMetadata[] = [];

    // Document Information
    const docInfo = [];
    if (pdf.title) docInfo.push({ label: 'Title', value: pdf.title, type: 'text' as const });
    if (pdf.author) docInfo.push({ label: 'Author', value: pdf.author, type: 'text' as const });
    if (pdf.subject) docInfo.push({ label: 'Subject', value: pdf.subject, type: 'text' as const });
    if (pdf.keywords) docInfo.push({ label: 'Keywords', value: pdf.keywords, type: 'text' as const });

    if (docInfo.length > 0) {
      sections.push({
        category: 'Document Info',
        icon: <DocumentIcon />,
        items: docInfo,
      });
    }

    // Technical Details
    const techInfo = [];
    if (pdf.creator) techInfo.push({ label: 'Created With', value: pdf.creator, type: 'text' as const });
    if (pdf.producer) techInfo.push({ label: 'PDF Producer', value: pdf.producer, type: 'text' as const });
    if (pdf.pdf_version) techInfo.push({ label: 'PDF Version', value: pdf.pdf_version, type: 'technical' as const });
    if (pdf.page_count) techInfo.push({ label: 'Pages', value: pdf.page_count, type: 'technical' as const });
    if (pdf.encrypted !== undefined) techInfo.push({ label: 'Encrypted', value: pdf.encrypted ? 'Yes' : 'No', type: 'text' as const });

    if (techInfo.length > 0) {
      sections.push({
        category: 'Technical',
        icon: <SettingsIcon />,
        items: techInfo,
      });
    }

    // Timestamps
    const dateInfo = [];
    if (pdf.creation_date) dateInfo.push({ label: 'Created', value: pdf.creation_date, type: 'date' as const });
    if (pdf.modification_date) dateInfo.push({ label: 'Modified', value: pdf.modification_date, type: 'date' as const });

    if (dateInfo.length > 0) {
      sections.push({
        category: 'Timestamps',
        icon: <DateIcon />,
        items: dateInfo,
      });
    }

    return sections;
  };

  const parseOfficeMetadata = (office: Record<string, any>): ParsedMetadata[] => {
    const sections: ParsedMetadata[] = [];

    // Document Properties
    const docInfo = [];
    if (office.title) docInfo.push({ label: 'Title', value: office.title, type: 'text' as const });
    if (office.author) docInfo.push({ label: 'Author', value: office.author, type: 'text' as const });
    if (office.company) docInfo.push({ label: 'Company', value: office.company, type: 'text' as const });
    if (office.manager) docInfo.push({ label: 'Manager', value: office.manager, type: 'text' as const });
    if (office.category) docInfo.push({ label: 'Category', value: office.category, type: 'text' as const });

    if (docInfo.length > 0) {
      sections.push({
        category: 'Document Properties',
        icon: <PersonIcon />,
        items: docInfo,
      });
    }

    // Application Info
    const appInfo = [];
    if (office.application) appInfo.push({ label: 'Application', value: office.application, type: 'text' as const });
    if (office.app_version) appInfo.push({ label: 'Version', value: office.app_version, type: 'technical' as const });
    if (office.template) appInfo.push({ label: 'Template', value: office.template, type: 'text' as const });

    if (appInfo.length > 0) {
      sections.push({
        category: 'Application',
        icon: <BusinessIcon />,
        items: appInfo,
      });
    }

    return sections;
  };

  const parseGenericMetadata = (data: Record<string, any>): ParsedMetadata[] => {
    const sections: ParsedMetadata[] = [];
    
    // Group remaining metadata
    const otherItems = Object.entries(data)
      .filter(([key, value]) => value !== null && value !== undefined && value !== '')
      .map(([key, value]) => ({
        label: key.replace(/_/g, ' ').replace(/\b\w/g, l => l.toUpperCase()),
        value: typeof value === 'object' ? JSON.stringify(value) : String(value),
        type: 'text' as const,
      }));

    if (otherItems.length > 0) {
      sections.push({
        category: 'Additional Properties',
        icon: <SettingsIcon />,
        items: otherItems,
      });
    }

    return sections;
  };

  const formatValue = (item: any) => {
    switch (item.type) {
      case 'date':
        try {
          return new Date(item.value).toLocaleString();
        } catch {
          return item.value;
        }
      case 'location':
        return item.value;
      case 'technical':
        return `${item.value}${item.unit ? ` ${item.unit}` : ''}`;
      case 'copyable':
        return (
          <Box sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <Typography variant="body2" sx={{ fontFamily: 'monospace', flex: 1 }}>
              {item.value}
            </Typography>
            <Tooltip title="Copy to clipboard">
              <IconButton size="small" onClick={() => copyToClipboard(item.value)}>
                <CopyIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
        );
      default:
        return item.value;
    }
  };

  // Parse metadata based on file type
  let parsedSections: ParsedMetadata[] = [];
  
  if (fileType.includes('image') && metadata.exif) {
    parsedSections = [...parsedSections, ...parseExifData(metadata.exif)];
  }
  
  if (fileType.includes('pdf') && metadata.pdf) {
    parsedSections = [...parsedSections, ...parsePdfMetadata(metadata.pdf)];
  }
  
  if ((fileType.includes('officedocument') || fileType.includes('msword')) && metadata.office) {
    parsedSections = [...parsedSections, ...parseOfficeMetadata(metadata.office)];
  }

  // Add any remaining metadata
  const remainingMetadata = { ...metadata };
  delete remainingMetadata.exif;
  delete remainingMetadata.pdf;
  delete remainingMetadata.office;
  
  if (Object.keys(remainingMetadata).length > 0) {
    parsedSections = [...parsedSections, ...parseGenericMetadata(remainingMetadata)];
  }

  if (parsedSections.length === 0) {
    return (
      <Typography variant="body2" color="text.secondary" sx={{ fontStyle: 'italic' }}>
        No detailed metadata available for this file type
      </Typography>
    );
  }

  if (compact) {
    return (
      <Box>
        {parsedSections.slice(0, 2).map((section, index) => (
          <Box key={index} sx={{ mb: 2 }}>
            <Box sx={{ display: 'flex', alignItems: 'center', mb: 1 }}>
              <Box sx={{ fontSize: 16, mr: 1, color: modernTokens.colors.primary[500], display: 'inline-flex' }}>
                {section.icon}
              </Box>
              <Typography variant="subtitle2" sx={{ fontWeight: 600 }}>
                {section.category}
              </Typography>
            </Box>
            <Stack spacing={1}>
              {section.items.slice(0, 3).map((item, itemIndex) => (
                <Box key={itemIndex} sx={{ display: 'flex', justifyContent: 'space-between' }}>
                  <Typography variant="caption" color="text.secondary">
                    {item.label}
                  </Typography>
                  <Typography variant="caption" sx={{ fontWeight: 500 }}>
                    {formatValue(item)}
                  </Typography>
                </Box>
              ))}
            </Stack>
          </Box>
        ))}
        {parsedSections.length > 2 && (
          <Typography variant="caption" color="text.secondary">
            +{parsedSections.length - 2} more sections...
          </Typography>
        )}
      </Box>
    );
  }

  return (
    <Box>
      {parsedSections.map((section, index) => (
        <Accordion 
          key={index} 
          sx={{ 
            boxShadow: 'none', 
            border: `1px solid ${modernTokens.colors.neutral[200]}`,
            borderRadius: borderRadius.lg,
            mb: 1,
            '&:before': { display: 'none' },
          }}
        >
          <AccordionSummary 
            expandIcon={<ExpandMoreIcon />}
            sx={{ 
              borderRadius: borderRadius.lg,
              '& .MuiAccordionSummary-content': {
                alignItems: 'center',
              },
            }}
          >
            <Box sx={{ fontSize: 20, mr: 1, color: modernTokens.colors.primary[500], display: 'inline-flex' }}>
              {section.icon}
            </Box>
            <Typography variant="subtitle1" sx={{ fontWeight: 600 }}>
              {section.category}
            </Typography>
            <Chip 
              label={section.items.length} 
              size="small" 
              sx={{ ml: 'auto', mr: 1 }}
            />
          </AccordionSummary>
          <AccordionDetails>
            <Grid container spacing={2}>
              {section.items.map((item, itemIndex) => (
                <Grid item xs={12} sm={6} key={itemIndex}>
                  <Paper 
                    sx={{ 
                      p: 2, 
                      backgroundColor: modernTokens.colors.neutral[50],
                      border: `1px solid ${modernTokens.colors.neutral[200]}`,
                    }}
                  >
                    <Typography 
                      variant="caption" 
                      color="text.secondary" 
                      sx={{ display: 'block', mb: 0.5, fontWeight: 500 }}
                    >
                      {item.label}
                    </Typography>
                    <Typography variant="body2" sx={{ fontWeight: 500 }}>
                      {formatValue(item)}
                    </Typography>
                  </Paper>
                </Grid>
              ))}
            </Grid>
          </AccordionDetails>
        </Accordion>
      ))}
    </Box>
  );
};

export default MetadataParser;