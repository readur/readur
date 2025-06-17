import React, { useEffect, useState } from 'react';
import {
  Box,
  Typography,
  FormControl,
  FormGroup,
  FormControlLabel,
  Checkbox,
  Chip,
  CircularProgress,
  Collapse,
  IconButton,
  Badge,
  Paper,
  Divider,
  Button,
  TextField,
  InputAdornment,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  Description as DocIcon,
  TextSnippet as TextIcon,
  Article as ArticleIcon,
  TableChart as SpreadsheetIcon,
  Code as CodeIcon,
  Folder as FolderIcon,
  Search as SearchIcon,
  Clear as ClearIcon,
} from '@mui/icons-material';
import { documentService, FacetItem } from '../../services/api';

interface MimeTypeFacetFilterProps {
  selectedMimeTypes: string[];
  onMimeTypeChange: (mimeTypes: string[]) => void;
  maxItemsToShow?: number;
}

interface MimeTypeGroup {
  label: string;
  icon: React.ReactElement;
  patterns: string[];
  color: string;
}

const MIME_TYPE_GROUPS: MimeTypeGroup[] = [
  {
    label: 'PDFs',
    icon: <PdfIcon />,
    patterns: ['application/pdf'],
    color: '#d32f2f',
  },
  {
    label: 'Images',
    icon: <ImageIcon />,
    patterns: ['image/'],
    color: '#1976d2',
  },
  {
    label: 'Documents',
    icon: <DocIcon />,
    patterns: ['application/msword', 'application/vnd.openxmlformats-officedocument.wordprocessingml', 'application/rtf'],
    color: '#388e3c',
  },
  {
    label: 'Spreadsheets',
    icon: <SpreadsheetIcon />,
    patterns: ['application/vnd.ms-excel', 'application/vnd.openxmlformats-officedocument.spreadsheetml', 'text/csv'],
    color: '#f57c00',
  },
  {
    label: 'Text Files',
    icon: <TextIcon />,
    patterns: ['text/plain', 'text/markdown', 'text/x-'],
    color: '#7b1fa2',
  },
  {
    label: 'Code',
    icon: <CodeIcon />,
    patterns: ['application/javascript', 'application/json', 'application/xml', 'text/html', 'text/css'],
    color: '#00796b',
  },
];

const MimeTypeFacetFilter: React.FC<MimeTypeFacetFilterProps> = ({
  selectedMimeTypes,
  onMimeTypeChange,
  maxItemsToShow = 10,
}) => {
  const [facets, setFacets] = useState<FacetItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [expanded, setExpanded] = useState(true);
  const [showAll, setShowAll] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');

  useEffect(() => {
    loadFacets();
  }, []);

  const loadFacets = async () => {
    try {
      setLoading(true);
      const response = await documentService.getFacets();
      setFacets(response.data.mime_types);
    } catch (error) {
      console.error('Failed to load facets:', error);
    } finally {
      setLoading(false);
    }
  };

  const getGroupForMimeType = (mimeType: string): MimeTypeGroup | undefined => {
    return MIME_TYPE_GROUPS.find(group =>
      group.patterns.some(pattern => mimeType.startsWith(pattern))
    );
  };

  const getMimeTypeIcon = (mimeType: string): React.ReactElement => {
    const group = getGroupForMimeType(mimeType);
    return group ? group.icon : <FolderIcon />;
  };

  const getMimeTypeLabel = (mimeType: string): string => {
    const labels: Record<string, string> = {
      'application/pdf': 'PDF Documents',
      'image/jpeg': 'JPEG Images',
      'image/png': 'PNG Images',
      'image/gif': 'GIF Images',
      'image/webp': 'WebP Images',
      'text/plain': 'Plain Text',
      'text/html': 'HTML',
      'text/css': 'CSS',
      'text/csv': 'CSV Files',
      'text/markdown': 'Markdown',
      'application/json': 'JSON',
      'application/xml': 'XML',
      'application/msword': 'Word Documents',
      'application/vnd.openxmlformats-officedocument.wordprocessingml.document': 'Word Documents (DOCX)',
      'application/vnd.ms-excel': 'Excel Spreadsheets',
      'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet': 'Excel Spreadsheets (XLSX)',
      'application/rtf': 'Rich Text Format',
    };
    
    return labels[mimeType] || mimeType.split('/').pop()?.toUpperCase() || mimeType;
  };

  const handleToggleMimeType = (mimeType: string) => {
    const newSelection = selectedMimeTypes.includes(mimeType)
      ? selectedMimeTypes.filter(mt => mt !== mimeType)
      : [...selectedMimeTypes, mimeType];
    onMimeTypeChange(newSelection);
  };

  const handleSelectGroup = (group: MimeTypeGroup) => {
    const groupMimeTypes = facets
      .filter(facet => group.patterns.some(pattern => facet.value.startsWith(pattern)))
      .map(facet => facet.value);
    
    const allSelected = groupMimeTypes.every(mt => selectedMimeTypes.includes(mt));
    
    if (allSelected) {
      // Deselect all in group
      onMimeTypeChange(selectedMimeTypes.filter(mt => !groupMimeTypes.includes(mt)));
    } else {
      // Select all in group
      onMimeTypeChange([...new Set([...selectedMimeTypes, ...groupMimeTypes])]);
    }
  };

  const filteredFacets = facets.filter(facet =>
    searchTerm === '' || 
    facet.value.toLowerCase().includes(searchTerm.toLowerCase()) ||
    getMimeTypeLabel(facet.value).toLowerCase().includes(searchTerm.toLowerCase())
  );

  const visibleFacets = showAll ? filteredFacets : filteredFacets.slice(0, maxItemsToShow);

  const renderGroupedFacets = () => {
    const groupedFacets: Map<string, FacetItem[]> = new Map();
    const ungroupedFacets: FacetItem[] = [];

    filteredFacets.forEach(facet => {
      const group = getGroupForMimeType(facet.value);
      if (group) {
        if (!groupedFacets.has(group.label)) {
          groupedFacets.set(group.label, []);
        }
        groupedFacets.get(group.label)!.push(facet);
      } else {
        ungroupedFacets.push(facet);
      }
    });

    return (
      <>
        {MIME_TYPE_GROUPS.map(group => {
          const groupFacets = groupedFacets.get(group.label) || [];
          if (groupFacets.length === 0) return null;

          const totalCount = groupFacets.reduce((sum, facet) => sum + facet.count, 0);
          const selectedCount = groupFacets.filter(facet => selectedMimeTypes.includes(facet.value)).length;
          const allSelected = selectedCount === groupFacets.length;
          const someSelected = selectedCount > 0 && selectedCount < groupFacets.length;

          return (
            <Box key={group.label} sx={{ mb: 2 }}>
              <Box 
                display="flex" 
                alignItems="center" 
                sx={{ 
                  cursor: 'pointer',
                  '&:hover': { backgroundColor: 'action.hover' },
                  p: 1,
                  borderRadius: 1,
                }}
                onClick={() => handleSelectGroup(group)}
              >
                <Checkbox
                  checked={allSelected}
                  indeterminate={someSelected}
                  sx={{ p: 0, mr: 1 }}
                />
                <Box display="flex" alignItems="center" gap={1} flex={1}>
                  <Box sx={{ color: group.color }}>{group.icon}</Box>
                  <Typography variant="subtitle2" fontWeight="bold">
                    {group.label}
                  </Typography>
                  <Chip 
                    label={totalCount} 
                    size="small" 
                    variant={selectedCount > 0 ? "filled" : "outlined"}
                    color={selectedCount > 0 ? "primary" : "default"}
                  />
                </Box>
              </Box>
              
              <Box sx={{ pl: 4 }}>
                {groupFacets.map(facet => (
                  <FormControlLabel
                    key={facet.value}
                    control={
                      <Checkbox
                        size="small"
                        checked={selectedMimeTypes.includes(facet.value)}
                        onChange={() => handleToggleMimeType(facet.value)}
                      />
                    }
                    label={
                      <Box display="flex" alignItems="center" gap={1}>
                        <Typography variant="body2">
                          {getMimeTypeLabel(facet.value)}
                        </Typography>
                        <Chip label={facet.count} size="small" variant="outlined" />
                      </Box>
                    }
                    sx={{ display: 'flex', width: '100%', mb: 0.5 }}
                  />
                ))}
              </Box>
            </Box>
          );
        })}

        {ungroupedFacets.length > 0 && (
          <Box sx={{ mb: 2 }}>
            <Typography variant="subtitle2" fontWeight="bold" sx={{ mb: 1 }}>
              Other Types
            </Typography>
            {ungroupedFacets.map(facet => (
              <FormControlLabel
                key={facet.value}
                control={
                  <Checkbox
                    size="small"
                    checked={selectedMimeTypes.includes(facet.value)}
                    onChange={() => handleToggleMimeType(facet.value)}
                  />
                }
                label={
                  <Box display="flex" alignItems="center" gap={1}>
                    <Typography variant="body2">
                      {getMimeTypeLabel(facet.value)}
                    </Typography>
                    <Chip label={facet.count} size="small" variant="outlined" />
                  </Box>
                }
                sx={{ display: 'flex', width: '100%', mb: 0.5 }}
              />
            ))}
          </Box>
        )}
      </>
    );
  };

  return (
    <Paper variant="outlined" sx={{ p: 2 }}>
      <Box display="flex" alignItems="center" justifyContent="space-between" mb={1}>
        <Typography variant="subtitle1" fontWeight="bold">
          File Types
        </Typography>
        <Box display="flex" alignItems="center" gap={1}>
          {selectedMimeTypes.length > 0 && (
            <Chip
              label={`${selectedMimeTypes.length} selected`}
              size="small"
              onDelete={() => onMimeTypeChange([])}
              deleteIcon={<ClearIcon />}
            />
          )}
          <IconButton size="small" onClick={() => setExpanded(!expanded)}>
            {expanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
          </IconButton>
        </Box>
      </Box>

      <Collapse in={expanded}>
        {loading ? (
          <Box display="flex" justifyContent="center" p={2}>
            <CircularProgress size={24} />
          </Box>
        ) : (
          <>
            {facets.length > maxItemsToShow && (
              <TextField
                size="small"
                fullWidth
                placeholder="Search file types..."
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                sx={{ mb: 2 }}
                InputProps={{
                  startAdornment: (
                    <InputAdornment position="start">
                      <SearchIcon fontSize="small" />
                    </InputAdornment>
                  ),
                  endAdornment: searchTerm && (
                    <InputAdornment position="end">
                      <IconButton size="small" onClick={() => setSearchTerm('')}>
                        <ClearIcon fontSize="small" />
                      </IconButton>
                    </InputAdornment>
                  ),
                }}
              />
            )}

            <FormGroup>
              {renderGroupedFacets()}
            </FormGroup>

            {filteredFacets.length > maxItemsToShow && (
              <Box mt={2} display="flex" justifyContent="center">
                <Button
                  size="small"
                  onClick={() => setShowAll(!showAll)}
                  endIcon={showAll ? <ExpandLessIcon /> : <ExpandMoreIcon />}
                >
                  {showAll ? 'Show Less' : `Show All (${filteredFacets.length})`}
                </Button>
              </Box>
            )}

            {filteredFacets.length === 0 && searchTerm && (
              <Typography variant="body2" color="text.secondary" textAlign="center" py={2}>
                No file types match "{searchTerm}"
              </Typography>
            )}
          </>
        )}
      </Collapse>
    </Paper>
  );
};

export default MimeTypeFacetFilter;