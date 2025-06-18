import React, { useState } from 'react';
import {
  Box,
  Paper,
  Typography,
  FormControl,
  FormControlLabel,
  Switch,
  Select,
  MenuItem,
  InputLabel,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Chip,
  Slider,
  TextField,
  Button,
  Divider,
  IconButton,
  Tooltip,
  Alert,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  ToggleButtonGroup,
  ToggleButton,
  Card,
  CardContent,
  Collapse,
  Badge,
  Grid,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  Settings as SettingsIcon,
  Speed as SpeedIcon,
  Visibility as VisibilityIcon,
  TextSnippet as SnippetIcon,
  Search as SearchIcon,
  Tune as TuneIcon,
  Psychology as PsychologyIcon,
  FormatQuote as QuoteIcon,
  Code as CodeIcon,
  BlurOn as BlurIcon,
  Help as HelpIcon,
  RestoreFromTrash as ResetIcon,
  BookmarkBorder as SaveIcon,
  Lightbulb as TipIcon,
} from '@mui/icons-material';

type SearchMode = 'simple' | 'phrase' | 'fuzzy' | 'boolean';

interface AdvancedSearchSettings {
  useEnhancedSearch: boolean;
  searchMode: SearchMode;
  includeSnippets: boolean;
  snippetLength: number;
  fuzzyThreshold: number;
  resultLimit: number;
  includeOcrText: boolean;
  includeFileContent: boolean;
  includeFilenames: boolean;
  boostRecentDocs: boolean;
  enableAutoCorrect: boolean;
}

interface AdvancedSearchPanelProps {
  settings: AdvancedSearchSettings;
  onSettingsChange: (settings: Partial<AdvancedSearchSettings>) => void;
  onSavePreset?: (name: string, settings: AdvancedSearchSettings) => void;
  onLoadPreset?: (preset: AdvancedSearchSettings) => void;
  availablePresets?: { name: string; settings: AdvancedSearchSettings }[];
  expanded?: boolean;
  onExpandedChange?: (expanded: boolean) => void;
}

const AdvancedSearchPanel: React.FC<AdvancedSearchPanelProps> = ({
  settings,
  onSettingsChange,
  onSavePreset,
  onLoadPreset,
  availablePresets = [],
  expanded = false,
  onExpandedChange,
}) => {
  const [activeSection, setActiveSection] = useState<string>('search-behavior');
  const [showPresetSave, setShowPresetSave] = useState(false);
  const [presetName, setPresetName] = useState('');

  const sections = [
    {
      id: 'search-behavior',
      label: 'Search Behavior',
      icon: <SearchIcon />,
      description: 'How search queries are processed and matched',
    },
    {
      id: 'results-display',
      label: 'Results Display',
      icon: <VisibilityIcon />,
      description: 'How search results are shown and formatted',
    },
    {
      id: 'performance',
      label: 'Performance',
      icon: <SpeedIcon />,
      description: 'Speed and resource optimization settings',
    },
    {
      id: 'content-sources',
      label: 'Content Sources',
      icon: <PsychologyIcon />,
      description: 'Which parts of documents to search',
    },
  ];

  const handleSettingChange = <K extends keyof AdvancedSearchSettings>(
    key: K,
    value: AdvancedSearchSettings[K]
  ) => {
    onSettingsChange({ [key]: value });
  };

  const handleResetToDefaults = () => {
    const defaults: AdvancedSearchSettings = {
      useEnhancedSearch: true,
      searchMode: 'simple',
      includeSnippets: true,
      snippetLength: 200,
      fuzzyThreshold: 0.8,
      resultLimit: 100,
      includeOcrText: true,
      includeFileContent: true,
      includeFilenames: true,
      boostRecentDocs: false,
      enableAutoCorrect: true,
    };
    onSettingsChange(defaults);
  };

  const handleSavePreset = () => {
    if (presetName.trim() && onSavePreset) {
      onSavePreset(presetName.trim(), settings);
      setPresetName('');
      setShowPresetSave(false);
    }
  };

  const getSearchModeDescription = (mode: SearchMode) => {
    switch (mode) {
      case 'simple':
        return 'Basic keyword matching with stemming';
      case 'phrase':
        return 'Exact phrase matching in order';
      case 'fuzzy':
        return 'Flexible matching with typo tolerance';
      case 'boolean':
        return 'Advanced operators (AND, OR, NOT)';
      default:
        return '';
    }
  };

  return (
    <Paper 
      elevation={2} 
      sx={{ 
        mb: 3,
        overflow: 'hidden',
        border: '1px solid',
        borderColor: expanded ? 'primary.main' : 'divider',
        transition: 'all 0.3s ease',
      }}
    >
      <Box
        sx={{
          p: 2,
          backgroundColor: expanded ? 'primary.50' : 'background.paper',
          cursor: 'pointer',
          transition: 'all 0.2s ease',
          '&:hover': {
            backgroundColor: expanded ? 'primary.100' : 'action.hover',
          },
        }}
        onClick={() => onExpandedChange?.(!expanded)}
      >
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Box display="flex" alignItems="center" gap={2}>
            <Badge 
              color="primary" 
              variant="dot" 
              invisible={!settings.useEnhancedSearch}
            >
              <TuneIcon color={expanded ? 'primary' : 'action'} />
            </Badge>
            <Box>
              <Typography variant="h6" color={expanded ? 'primary.main' : 'text.primary'}>
                Advanced Search Options
              </Typography>
              <Typography variant="caption" color="text.secondary">
                Customize search behavior and result display
              </Typography>
            </Box>
          </Box>
          <Box display="flex" alignItems="center" gap={1}>
            <Chip
              label={settings.searchMode.toUpperCase()}
              size="small"
              color={settings.useEnhancedSearch ? 'primary' : 'default'}
              variant={expanded ? 'filled' : 'outlined'}
            />
            <IconButton size="small">
              {expanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
            </IconButton>
          </Box>
        </Box>
      </Box>

      <Collapse in={expanded}>
        <Box sx={{ p: 0 }}>
          {/* Section Tabs */}
          <Box sx={{ borderBottom: 1, borderColor: 'divider', px: 2 }}>
            <Box display="flex" gap={1} sx={{ overflowX: 'auto', py: 1 }}>
              {sections.map((section) => (
                <Button
                  key={section.id}
                  variant={activeSection === section.id ? 'contained' : 'text'}
                  size="small"
                  startIcon={section.icon}
                  onClick={() => setActiveSection(section.id)}
                  sx={{ minWidth: 'fit-content', whiteSpace: 'nowrap' }}
                >
                  {section.label}
                </Button>
              ))}
            </Box>
          </Box>

          <Box sx={{ p: 3 }}>
            {/* Search Behavior Section */}
            {activeSection === 'search-behavior' && (
              <Box>
                <Alert severity="info" sx={{ mb: 2 }}>
                  <Typography variant="body2">
                    These settings control how your search queries are interpreted and matched against documents.
                  </Typography>
                </Alert>

                <Box display="flex" flexDirection={{ xs: 'column', md: 'row' }} gap={3} mb={3}>
                  <Box flex={1}>
                    <FormControl fullWidth>
                    <InputLabel id="search-mode-label">Search Mode</InputLabel>
                    <Select
                      labelId="search-mode-label"
                      value={settings.searchMode}
                      onChange={(e) => handleSettingChange('searchMode', e.target.value as SearchMode)}
                      label="Search Mode"
                    >
                      <MenuItem value="simple">
                        <Box>
                          <Typography variant="body2">Simple Search</Typography>
                          <Typography variant="caption" color="text.secondary">
                            Basic keyword matching
                          </Typography>
                        </Box>
                      </MenuItem>
                      <MenuItem value="phrase">
                        <Box>
                          <Typography variant="body2">Phrase Search</Typography>
                          <Typography variant="caption" color="text.secondary">
                            Exact phrase matching
                          </Typography>
                        </Box>
                      </MenuItem>
                      <MenuItem value="fuzzy">
                        <Box>
                          <Typography variant="body2">Fuzzy Search</Typography>
                          <Typography variant="caption" color="text.secondary">
                            Typo-tolerant matching
                          </Typography>
                        </Box>
                      </MenuItem>
                      <MenuItem value="boolean">
                        <Box>
                          <Typography variant="body2">Boolean Search</Typography>
                          <Typography variant="caption" color="text.secondary">
                            AND, OR, NOT operators
                          </Typography>
                        </Box>
                      </MenuItem>
                    </Select>
                  </FormControl>
                    <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
                      {getSearchModeDescription(settings.searchMode)}
                    </Typography>
                  </Box>

                  <Box flex={1}>
                    <Typography variant="body2" gutterBottom>
                      Fuzzy Match Threshold: {settings.fuzzyThreshold}
                    </Typography>
                    <Slider
                      value={settings.fuzzyThreshold}
                      onChange={(_, value) => handleSettingChange('fuzzyThreshold', value as number)}
                      min={0.1}
                      max={1.0}
                      step={0.1}
                      marks
                      disabled={settings.searchMode !== 'fuzzy'}
                      valueLabelDisplay="auto"
                      valueLabelFormat={(value) => `${(value * 100).toFixed(0)}%`}
                    />
                    <Typography variant="caption" color="text.secondary">
                      Higher values = stricter matching
                    </Typography>
                  </Box>
                </Box>

                <Box>
                  <Box display="flex" flexWrap="wrap" gap={2}>
                    <FormControlLabel
                      control={
                        <Switch
                          checked={settings.useEnhancedSearch}
                          onChange={(e) => handleSettingChange('useEnhancedSearch', e.target.checked)}
                        />
                      }
                      label="Enhanced Search Engine"
                    />
                    <FormControlLabel
                      control={
                        <Switch
                          checked={settings.enableAutoCorrect}
                          onChange={(e) => handleSettingChange('enableAutoCorrect', e.target.checked)}
                        />
                      }
                      label="Auto-correct Spelling"
                    />
                    <FormControlLabel
                      control={
                        <Switch
                          checked={settings.boostRecentDocs}
                          onChange={(e) => handleSettingChange('boostRecentDocs', e.target.checked)}
                        />
                      }
                      label="Boost Recent Documents"
                    />
                  </Box>
                </Box>
              </Box>
            )}

            {/* Results Display Section */}
            {activeSection === 'results-display' && (
              <Grid container spacing={3}>
                <Grid size={12}>
                  <Alert severity="info" sx={{ mb: 2 }}>
                    <Typography variant="body2">
                      Control how search results are presented and what information is shown.
                    </Typography>
                  </Alert>
                </Grid>

                <Grid size={{ xs: 12, md: 4 }}>
                  <FormControlLabel
                    control={
                      <Switch
                        checked={settings.includeSnippets}
                        onChange={(e) => handleSettingChange('includeSnippets', e.target.checked)}
                      />
                    }
                    label="Show Text Snippets"
                  />
                </Grid>

                <Grid size={{ xs: 12, md: 4 }}>
                  <FormControl fullWidth disabled={!settings.includeSnippets}>
                    <InputLabel id="snippet-length-label">Snippet Length</InputLabel>
                    <Select
                      labelId="snippet-length-label"
                      value={settings.snippetLength}
                      onChange={(e) => handleSettingChange('snippetLength', e.target.value as number)}
                      label="Snippet Length"
                    >
                      <MenuItem value={100}>Short (100 chars)</MenuItem>
                      <MenuItem value={200}>Medium (200 chars)</MenuItem>
                      <MenuItem value={400}>Long (400 chars)</MenuItem>
                      <MenuItem value={600}>Extra Long (600 chars)</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>

                <Grid size={{ xs: 12, md: 4 }}>
                  <FormControl fullWidth>
                    <InputLabel id="results-per-page-label">Results Per Page</InputLabel>
                    <Select
                      labelId="results-per-page-label"
                      value={settings.resultLimit}
                      onChange={(e) => handleSettingChange('resultLimit', e.target.value as number)}
                      label="Results Per Page"
                    >
                      <MenuItem value={25}>25 results</MenuItem>
                      <MenuItem value={50}>50 results</MenuItem>
                      <MenuItem value={100}>100 results</MenuItem>
                      <MenuItem value={200}>200 results</MenuItem>
                    </Select>
                  </FormControl>
                </Grid>
              </Grid>
            )}

            {/* Content Sources Section */}
            {activeSection === 'content-sources' && (
              <Grid container spacing={3}>
                <Grid size={12}>
                  <Alert severity="info" sx={{ mb: 2 }}>
                    <Typography variant="body2">
                      Choose which parts of your documents to include in search.
                    </Typography>
                  </Alert>
                </Grid>

                <Grid size={12}>
                  <Typography variant="subtitle2" gutterBottom>
                    Search In:
                  </Typography>
                  <Box display="flex" flexDirection="column" gap={1}>
                    <FormControlLabel
                      control={
                        <Switch
                          checked={settings.includeFileContent}
                          onChange={(e) => handleSettingChange('includeFileContent', e.target.checked)}
                        />
                      }
                      label="Document Content"
                    />
                    <FormControlLabel
                      control={
                        <Switch
                          checked={settings.includeOcrText}
                          onChange={(e) => handleSettingChange('includeOcrText', e.target.checked)}
                        />
                      }
                      label="OCR Extracted Text"
                    />
                    <FormControlLabel
                      control={
                        <Switch
                          checked={settings.includeFilenames}
                          onChange={(e) => handleSettingChange('includeFilenames', e.target.checked)}
                        />
                      }
                      label="Filenames"
                    />
                  </Box>
                </Grid>
              </Grid>
            )}

            {/* Performance Section */}
            {activeSection === 'performance' && (
              <Grid container spacing={3}>
                <Grid size={12}>
                  <Alert severity="warning" sx={{ mb: 2 }}>
                    <Typography variant="body2">
                      These settings can affect search speed. Use with caution for large document collections.
                    </Typography>
                  </Alert>
                </Grid>

                <Grid size={12}>
                  <Typography variant="body2" gutterBottom>
                    Maximum Results: {settings.resultLimit}
                  </Typography>
                  <Slider
                    value={settings.resultLimit}
                    onChange={(_, value) => handleSettingChange('resultLimit', value as number)}
                    min={10}
                    max={500}
                    step={10}
                    marks={[
                      { value: 25, label: '25' },
                      { value: 100, label: '100' },
                      { value: 250, label: '250' },
                      { value: 500, label: '500' },
                    ]}
                    valueLabelDisplay="auto"
                  />
                  <Typography variant="caption" color="text.secondary">
                    Higher values may slow down search for large collections
                  </Typography>
                </Grid>
              </Grid>
            )}

            {/* Action Buttons */}
            <Divider sx={{ my: 3 }} />
            <Box display="flex" alignItems="center" justifyContent="between" gap={2}>
              <Box display="flex" gap={1}>
                <Button
                  variant="outlined"
                  size="small"
                  startIcon={<ResetIcon />}
                  onClick={handleResetToDefaults}
                >
                  Reset to Defaults
                </Button>
                
                <Button
                  variant="outlined"
                  size="small"
                  startIcon={<SaveIcon />}
                  onClick={() => setShowPresetSave(!showPresetSave)}
                >
                  Save Preset
                </Button>
              </Box>

              {availablePresets.length > 0 && (
                <FormControl size="small" sx={{ minWidth: 150 }}>
                  <InputLabel id="load-preset-label">Load Preset</InputLabel>
                  <Select
                    labelId="load-preset-label"
                    label="Load Preset"
                    onChange={(e) => {
                      const preset = availablePresets.find(p => p.name === e.target.value);
                      if (preset && onLoadPreset) {
                        onLoadPreset(preset.settings);
                      }
                    }}
                    value=""
                  >
                    {availablePresets.map((preset) => (
                      <MenuItem key={preset.name} value={preset.name}>
                        {preset.name}
                      </MenuItem>
                    ))}
                  </Select>
                </FormControl>
              )}
            </Box>

            {/* Save Preset Dialog */}
            <Collapse in={showPresetSave}>
              <Card variant="outlined" sx={{ mt: 2, p: 2 }}>
                <Typography variant="subtitle2" gutterBottom>
                  Save Current Settings as Preset
                </Typography>
                <Box display="flex" gap={1} alignItems="end">
                  <TextField
                    size="small"
                    label="Preset Name"
                    value={presetName}
                    onChange={(e) => setPresetName(e.target.value)}
                    fullWidth
                  />
                  <Button
                    variant="contained"
                    size="small"
                    onClick={handleSavePreset}
                    disabled={!presetName.trim()}
                  >
                    Save
                  </Button>
                  <Button
                    size="small"
                    onClick={() => {
                      setShowPresetSave(false);
                      setPresetName('');
                    }}
                  >
                    Cancel
                  </Button>
                </Box>
              </Card>
            </Collapse>
          </Box>
        </Box>
      </Collapse>
    </Paper>
  );
};

export default AdvancedSearchPanel;