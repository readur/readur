import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import {
  Box,
  Typography,
  Grid,
  Card,
  CardContent,
  TextField,
  InputAdornment,
  Button,
  Chip,
  Stack,
  FormControl,
  InputLabel,
  Select,
  MenuItem,
  OutlinedInput,
  Checkbox,
  ListItemText,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  Slider,
  ToggleButton,
  ToggleButtonGroup,
  CircularProgress,
  Alert,
  Divider,
  IconButton,
  Tooltip,
  Autocomplete,
  LinearProgress,
  FormControlLabel,
  Switch,
  Paper,
  Skeleton,
  SelectChangeEvent,
} from '@mui/material';
import {
  Search as SearchIcon,
  FilterList as FilterIcon,
  Clear as ClearIcon,
  ExpandMore as ExpandMoreIcon,
  GridView as GridViewIcon,
  ViewList as ListViewIcon,
  Download as DownloadIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  Description as DocIcon,
  TextSnippet as TextIcon,
  CalendarToday as DateIcon,
  Storage as SizeIcon,
  Tag as TagIcon,
  Visibility as ViewIcon,
  Settings as SettingsIcon,
  Speed as SpeedIcon,
  AccessTime as TimeIcon,
  TrendingUp as TrendingIcon,
} from '@mui/icons-material';
import { documentService, SearchRequest } from '../services/api';
import SearchGuidance from '../components/SearchGuidance';
import EnhancedSearchGuide from '../components/EnhancedSearchGuide';
import MimeTypeFacetFilter from '../components/MimeTypeFacetFilter';
import EnhancedSnippetViewer from '../components/EnhancedSnippetViewer';
import AdvancedSearchPanel from '../components/AdvancedSearchPanel';

interface Document {
  id: string;
  original_filename: string;
  filename?: string;
  file_size: number;
  mime_type: string;
  created_at: string;
  has_ocr_text?: boolean;
  tags: string[];
  snippets?: Snippet[];
  search_rank?: number;
}

interface Snippet {
  text: string;
  highlight_ranges?: HighlightRange[];
}

interface HighlightRange {
  start: number;
  end: number;
}

interface SearchResponse {
  documents: Document[];
  total: number;
  query_time_ms: number;
  suggestions?: string[];
}

interface MimeTypeOption {
  value: string;
  label: string;
}

interface SearchFilters {
  tags?: string[];
  mimeTypes?: string[];
  dateRange?: number[];
  fileSizeRange?: number[];
  hasOcr?: string;
}

type ViewMode = 'grid' | 'list';
type SearchMode = 'simple' | 'phrase' | 'fuzzy' | 'boolean';
type OcrStatus = 'all' | 'yes' | 'no';

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

const SearchPage: React.FC = () => {
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [searchQuery, setSearchQuery] = useState<string>(searchParams.get('q') || '');
  const [searchResults, setSearchResults] = useState<Document[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [viewMode, setViewMode] = useState<ViewMode>('grid');
  const [queryTime, setQueryTime] = useState<number>(0);
  const [totalResults, setTotalResults] = useState<number>(0);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [isTyping, setIsTyping] = useState<boolean>(false);
  const [searchProgress, setSearchProgress] = useState<number>(0);
  const [quickSuggestions, setQuickSuggestions] = useState<string[]>([]);
  const [showFilters, setShowFilters] = useState<boolean>(false);
  const [searchTips] = useState<string[]>([
    'Use quotes for exact phrases: "project plan"',
    'Search by tags: tag:important or tag:invoice', 
    'Combine terms: contract AND payment',
    'Use wildcards: proj* for project, projects, etc.'
  ]);
  
  // Search settings - consolidated into advanced settings
  const [showAdvanced, setShowAdvanced] = useState<boolean>(false);
  const [advancedSettings, setAdvancedSettings] = useState<AdvancedSearchSettings>({
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
  });
  
  // Filter states
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [selectedMimeTypes, setSelectedMimeTypes] = useState<string[]>([]);
  const [dateRange, setDateRange] = useState<number[]>([0, 365]); // days
  const [fileSizeRange, setFileSizeRange] = useState<number[]>([0, 100]); // MB
  const [hasOcr, setHasOcr] = useState<OcrStatus>('all');
  
  // Available options (would typically come from API)
  const [availableTags, setAvailableTags] = useState<string[]>([]);
  const mimeTypeOptions: MimeTypeOption[] = [
    { value: 'application/pdf', label: 'PDF' },
    { value: 'image/', label: 'Images' },
    { value: 'text/', label: 'Text Files' },
    { value: 'application/msword', label: 'Word Documents' },
    { value: 'application/vnd.openxmlformats-officedocument', label: 'Office Documents' },
  ];

  // Enhanced debounced search with typing indicators
  const debounce = useCallback((func: (...args: any[]) => void, delay: number) => {
    let timeoutId: NodeJS.Timeout;
    return (...args: any[]) => {
      clearTimeout(timeoutId);
      setIsTyping(true);
      timeoutId = setTimeout(() => {
        setIsTyping(false);
        func.apply(null, args);
      }, delay);
    };
  }, []);

  // Quick suggestions generator
  const generateQuickSuggestions = useCallback((query: string): void => {
    if (!query || query.length < 2) {
      setQuickSuggestions([]);
      return;
    }
    
    const suggestions: string[] = [];
    
    // Add exact phrase suggestion
    if (!query.includes('"')) {
      suggestions.push(`"${query}"`);
    }
    
    // Add tag suggestions
    if (!query.startsWith('tag:')) {
      suggestions.push(`tag:${query}`);
    }
    
    // Add wildcard suggestion
    if (!query.includes('*')) {
      suggestions.push(`${query}*`);
    }
    
    setQuickSuggestions(suggestions.slice(0, 3));
  }, []);

  const performSearch = useCallback(async (query: string, filters: SearchFilters = {}): Promise<void> => {
    if (!query.trim()) {
      setSearchResults([]);
      setTotalResults(0);
      setQueryTime(0);
      setSuggestions([]);
      setQuickSuggestions([]);
      return;
    }

    try {
      setLoading(true);
      setError(null);
      setSearchProgress(0);
      
      // Simulate progressive loading for better UX
      const progressInterval = setInterval(() => {
        setSearchProgress(prev => Math.min(prev + 20, 90));
      }, 100);
      
      const searchRequest: SearchRequest = {
        query: query.trim(),
        tags: filters.tags?.length ? filters.tags : undefined,
        mime_types: filters.mimeTypes?.length ? filters.mimeTypes : undefined,
        limit: advancedSettings.resultLimit,
        offset: 0,
        include_snippets: advancedSettings.includeSnippets,
        snippet_length: advancedSettings.snippetLength,
        search_mode: advancedSettings.searchMode,
      };

      const response = advancedSettings.useEnhancedSearch 
        ? await documentService.enhancedSearch(searchRequest)
        : await documentService.search(searchRequest);
      
      // Apply additional client-side filters
      let results = response.data.documents || [];
      
      // Filter by date range
      if (filters.dateRange) {
        const now = new Date();
        const [minDays, maxDays] = filters.dateRange;
        results = results.filter(doc => {
          const docDate = new Date(doc.created_at);
          const daysDiff = Math.ceil((now.getTime() - docDate.getTime()) / (1000 * 60 * 60 * 24));
          return daysDiff >= minDays && daysDiff <= maxDays;
        });
      }
      
      // Filter by file size
      if (filters.fileSizeRange) {
        const [minMB, maxMB] = filters.fileSizeRange;
        results = results.filter(doc => {
          const sizeMB = doc.file_size / (1024 * 1024);
          return sizeMB >= minMB && sizeMB <= maxMB;
        });
      }
      
      // Filter by OCR status
      if (filters.hasOcr && filters.hasOcr !== 'all') {
        results = results.filter(doc => {
          return filters.hasOcr === 'yes' ? doc.has_ocr_text : !doc.has_ocr_text;
        });
      }
      
      clearInterval(progressInterval);
      setSearchProgress(100);
      
      setSearchResults(results);
      setTotalResults(response.data.total || results.length);
      setQueryTime(response.data.query_time_ms || 0);
      setSuggestions(response.data.suggestions || []);
      
      // Extract unique tags for filter options
      const tags = [...new Set(results.flatMap(doc => doc.tags))];
      setAvailableTags(tags);
      
      // Clear progress after a brief delay
      setTimeout(() => setSearchProgress(0), 500);
      
    } catch (err) {
      clearInterval(progressInterval);
      setSearchProgress(0);
      setError('Search failed. Please try again.');
      console.error(err);
    } finally {
      setLoading(false);
    }
  }, [advancedSettings]);

  const debouncedSearch = useCallback(
    debounce((query: string, filters: SearchFilters) => performSearch(query, filters), 300),
    [performSearch]
  );
  
  const quickSuggestionsDebounced = useCallback(
    debounce((query: string) => generateQuickSuggestions(query), 150),
    [generateQuickSuggestions]
  );

  // Handle URL search params
  useEffect(() => {
    const queryFromUrl = searchParams.get('q');
    if (queryFromUrl && queryFromUrl !== searchQuery) {
      setSearchQuery(queryFromUrl);
    }
  }, [searchParams]);

  useEffect(() => {
    const filters: SearchFilters = {
      tags: selectedTags,
      mimeTypes: selectedMimeTypes,
      dateRange: dateRange,
      fileSizeRange: fileSizeRange,
      hasOcr: hasOcr,
    };
    debouncedSearch(searchQuery, filters);
    quickSuggestionsDebounced(searchQuery);
    
    // Update URL params when search query changes
    if (searchQuery) {
      setSearchParams({ q: searchQuery });
    } else {
      setSearchParams({});
    }
  }, [searchQuery, selectedTags, selectedMimeTypes, dateRange, fileSizeRange, hasOcr, debouncedSearch, quickSuggestionsDebounced, setSearchParams]);

  const handleClearFilters = (): void => {
    setSelectedTags([]);
    setSelectedMimeTypes([]);
    setDateRange([0, 365]);
    setFileSizeRange([0, 100]);
    setHasOcr('all');
  };

  const getFileIcon = (mimeType: string): React.ReactElement => {
    if (mimeType.includes('pdf')) return <PdfIcon color="error" />;
    if (mimeType.includes('image')) return <ImageIcon color="primary" />;
    if (mimeType.includes('text')) return <TextIcon color="info" />;
    return <DocIcon color="secondary" />;
  };

  const formatFileSize = (bytes: number): string => {
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    if (bytes === 0) return '0 Bytes';
    const i = Math.floor(Math.log(bytes) / Math.log(1024));
    return Math.round(bytes / Math.pow(1024, i) * 100) / 100 + ' ' + sizes[i];
  };

  const formatDate = (dateString: string): string => {
    return new Date(dateString).toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
    });
  };

  const handleDownload = async (doc: Document): Promise<void> => {
    try {
      const response = await documentService.download(doc.id);
      const url = window.URL.createObjectURL(new Blob([response.data]));
      const link = document.createElement('a');
      link.href = url;
      link.setAttribute('download', doc.original_filename);
      document.body.appendChild(link);
      link.click();
      link.remove();
      window.URL.revokeObjectURL(url);
    } catch (err) {
      console.error('Download failed:', err);
    }
  };

  const renderHighlightedText = (text: string, highlightRanges?: HighlightRange[]): React.ReactNode => {
    if (!highlightRanges || highlightRanges.length === 0) {
      return text;
    }

    const parts: React.ReactNode[] = [];
    let lastIndex = 0;

    highlightRanges.forEach((range, index) => {
      // Add text before highlight
      if (range.start > lastIndex) {
        parts.push(
          <span key={`text-${index}`}>
            {text.substring(lastIndex, range.start)}
          </span>
        );
      }
      
      // Add highlighted text
      parts.push(
        <Box
          key={`highlight-${index}`}
          component="mark"
          sx={{
            backgroundColor: 'primary.light',
            color: 'primary.contrastText',
            padding: '0 2px',
            borderRadius: '2px',
            fontWeight: 600,
          }}
        >
          {text.substring(range.start, range.end)}
        </Box>
      );
      
      lastIndex = range.end;
    });

    // Add remaining text
    if (lastIndex < text.length) {
      parts.push(
        <span key="final-text">
          {text.substring(lastIndex)}
        </span>
      );
    }

    return parts;
  };

  const handleSuggestionClick = (suggestion: string): void => {
    setSearchQuery(suggestion);
  };

  const handleViewModeChange = (event: React.MouseEvent<HTMLElement>, newView: ViewMode | null): void => {
    if (newView) {
      setViewMode(newView);
    }
  };

  const handleSearchModeChange = (event: React.MouseEvent<HTMLElement>, newMode: SearchMode | null): void => {
    if (newMode) {
      setSearchMode(newMode);
    }
  };

  const handleTagsChange = (event: SelectChangeEvent<string[]>): void => {
    const value = event.target.value;
    setSelectedTags(typeof value === 'string' ? value.split(',') : value);
  };

  const handleMimeTypesChange = (event: SelectChangeEvent<string[]>): void => {
    const value = event.target.value;
    setSelectedMimeTypes(typeof value === 'string' ? value.split(',') : value);
  };

  const handleOcrChange = (event: SelectChangeEvent<OcrStatus>): void => {
    setHasOcr(event.target.value as OcrStatus);
  };


  return (
    <Box sx={{ p: 3 }}>
      {/* Header with Prominent Search */}
      <Box sx={{ mb: 4 }}>
        <Typography 
          variant="h4" 
          sx={{ 
            fontWeight: 800,
            background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
            backgroundClip: 'text',
            WebkitBackgroundClip: 'text',
            color: 'transparent',
            mb: 2,
          }}
        >
          Search Documents
        </Typography>
        
        {/* Enhanced Search Bar */}
        <Paper 
          elevation={3}
          className="search-input-responsive"
          sx={{
            p: 2,
            mb: 3,
            background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.05) 0%, rgba(139, 92, 246, 0.05) 100%)',
            border: '1px solid',
            borderColor: 'primary.light',
          }}
        >
          <Box sx={{ position: 'relative' }}>
            <TextField
              fullWidth
              placeholder="Search documents by content, filename, or tags... Try 'invoice', 'contract', or tag:important"
              variant="outlined"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              InputProps={{
                startAdornment: (
                  <InputAdornment position="start">
                    <SearchIcon color="primary" sx={{ fontSize: '1.5rem' }} />
                  </InputAdornment>
                ),
                endAdornment: (
                  <InputAdornment position="end">
                    <Stack direction="row" spacing={1}>
                      {(loading || isTyping) && (
                        <CircularProgress 
                          size={20} 
                          variant={searchProgress > 0 ? "determinate" : "indeterminate"}
                          value={searchProgress}
                        />
                      )}
                      {searchQuery && (
                        <IconButton
                          size="small"
                          onClick={() => setSearchQuery('')}
                        >
                          <ClearIcon />
                        </IconButton>
                      )}
                      <IconButton
                        size="small"
                        onClick={() => setShowAdvanced(!showAdvanced)}
                        color={showAdvanced ? 'primary' : 'default'}
                      >
                        <SettingsIcon />
                      </IconButton>
                      <IconButton
                        size="small"
                        onClick={() => setShowFilters(!showFilters)}
                        color={showFilters ? 'primary' : 'default'}
                        sx={{ display: { xs: 'inline-flex', md: 'none' } }}
                      >
                        <FilterIcon />
                      </IconButton>
                    </Stack>
                  </InputAdornment>
                ),
              }}
              sx={{
                '& .MuiOutlinedInput-root': {
                  '& fieldset': {
                    borderWidth: 2,
                  },
                  '&:hover fieldset': {
                    borderColor: 'primary.main',
                  },
                  '&.Mui-focused fieldset': {
                    borderColor: 'primary.main',
                  },
                },
                '& .MuiInputBase-input': {
                  fontSize: '1.1rem',
                  py: 2,
                },
              }}
            />
            
            {/* Enhanced Loading Progress Bar */}
            {(loading || isTyping || searchProgress > 0) && (
              <LinearProgress 
                variant={searchProgress > 0 ? "determinate" : "indeterminate"}
                value={searchProgress}
                sx={{ 
                  position: 'absolute',
                  bottom: 0,
                  left: 0,
                  right: 0,
                  borderRadius: '0 0 4px 4px',
                  opacity: isTyping ? 0.5 : 1,
                  transition: 'opacity 0.2s ease-in-out',
                }}
              />
            )}
          </Box>

          {/* Quick Stats */}
          {(searchQuery && !loading) && (
            <Box sx={{ 
              mt: 2, 
              display: 'flex', 
              justifyContent: 'space-between',
              alignItems: 'center',
              flexWrap: 'wrap',
              gap: 2,
            }}>
              <Stack direction="row" spacing={2} alignItems="center">
                <Chip 
                  icon={<TrendingIcon />}
                  label={`${totalResults} results`} 
                  size="small" 
                  color="primary"
                  variant="outlined"
                />
                <Chip 
                  icon={<TimeIcon />}
                  label={`${queryTime}ms`} 
                  size="small" 
                  variant="outlined"
                />
                {useEnhancedSearch && (
                  <Chip 
                    icon={<SpeedIcon />}
                    label="Enhanced" 
                    size="small" 
                    color="success"
                    variant="outlined"
                  />
                )}
              </Stack>
              
              {/* Simplified Search Mode Selector */}
              <ToggleButtonGroup
                value={searchMode}
                exclusive
                onChange={handleSearchModeChange}
                size="small"
              >
                <ToggleButton value="simple">Smart</ToggleButton>
                <ToggleButton value="phrase">Exact phrase</ToggleButton>
                <ToggleButton value="fuzzy">Similar words</ToggleButton>
                <ToggleButton value="boolean">Advanced</ToggleButton>
              </ToggleButtonGroup>
            </Box>
          )}

          {/* Quick Suggestions */}
          {quickSuggestions.length > 0 && searchQuery && !loading && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="body2" color="text.secondary" gutterBottom>
                Quick suggestions:
              </Typography>
              <Stack direction="row" spacing={1} flexWrap="wrap">
                {quickSuggestions.map((suggestion, index) => (
                  <Chip
                    key={index}
                    label={suggestion}
                    size="small"
                    onClick={() => handleSuggestionClick(suggestion)}
                    clickable
                    variant="outlined"
                    color="primary"
                    sx={{ 
                      '&:hover': { 
                        backgroundColor: 'primary.main',
                        color: 'primary.contrastText',
                      }
                    }}
                  />
                ))}
              </Stack>
            </Box>
          )}

          {/* Server Suggestions */}
          {suggestions.length > 0 && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="body2" color="text.secondary" gutterBottom>
                Related searches:
              </Typography>
              <Stack direction="row" spacing={1} flexWrap="wrap">
                {suggestions.map((suggestion, index) => (
                  <Chip
                    key={index}
                    label={suggestion}
                    size="small"
                    onClick={() => handleSuggestionClick(suggestion)}
                    clickable
                    variant="outlined"
                    sx={{ 
                      '&:hover': { 
                        backgroundColor: 'primary.light',
                        color: 'primary.contrastText',
                      }
                    }}
                  />
                ))}
              </Stack>
            </Box>
          )}

          {/* Enhanced Search Guide when not in advanced mode */}
          {!showAdvanced && (
            <Box sx={{ mt: 2 }}>
              <EnhancedSearchGuide 
                compact 
                onExampleClick={setSearchQuery}
              />
            </Box>
          )}
        </Paper>
      </Box>

      {/* Advanced Search Panel */}
      <AdvancedSearchPanel
        settings={advancedSettings}
        onSettingsChange={(newSettings) => 
          setAdvancedSettings(prev => ({ ...prev, ...newSettings }))
        }
        expanded={showAdvanced}
        onExpandedChange={setShowAdvanced}
      />

      <Grid container spacing={3}>
        {/* Mobile Filters Drawer */}
        {showFilters && (
          <Grid item xs={12} sx={{ display: { xs: 'block', md: 'none' } }}>
            <Card>
              <CardContent>
                <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                  <Typography variant="h6" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                    <FilterIcon />
                    Filters
                  </Typography>
                  <Button size="small" onClick={handleClearFilters} startIcon={<ClearIcon />}>
                    Clear
                  </Button>
                </Box>
                {/* Mobile filter content would go here - simplified */}
                <Typography variant="body2" color="text.secondary">
                  Mobile filters coming soon...
                </Typography>
              </CardContent>
            </Card>
          </Grid>
        )}

        {/* Desktop Filters Sidebar */}
        <Grid item xs={12} md={3} sx={{ display: { xs: 'none', md: 'block' } }}>
          <Card sx={{ position: 'sticky', top: 20 }}>  
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 2 }}>
                <Typography variant="h6" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
                  <FilterIcon />
                  Filters
                </Typography>
                <Button size="small" onClick={handleClearFilters} startIcon={<ClearIcon />}>
                  Clear
                </Button>
              </Box>

              <Stack spacing={3}>
                {/* Tags Filter */}
                <Accordion defaultExpanded>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">Tags</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <FormControl fullWidth size="small">
                      <InputLabel>Select Tags</InputLabel>
                      <Select
                        multiple
                        value={selectedTags}
                        onChange={handleTagsChange}
                        input={<OutlinedInput label="Select Tags" />}
                        renderValue={(selected) => (
                          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5 }}>
                            {selected.map((value) => (
                              <Chip key={value} label={value} size="small" />
                            ))}
                          </Box>
                        )}
                      >
                        {availableTags.map((tag) => (
                          <MenuItem key={tag} value={tag}>
                            <Checkbox checked={selectedTags.indexOf(tag) > -1} />
                            <ListItemText primary={tag} />
                          </MenuItem>
                        ))}
                      </Select>
                    </FormControl>
                  </AccordionDetails>
                </Accordion>

                {/* File Type Filter with Facets */}
                <Box sx={{ mb: 2 }}>
                  <MimeTypeFacetFilter
                    selectedMimeTypes={selectedMimeTypes}
                    onMimeTypeChange={setSelectedMimeTypes}
                    maxItemsToShow={8}
                  />
                </Box>

                {/* OCR Filter */}
                <Accordion>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">OCR Status</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <FormControl fullWidth size="small">
                      <InputLabel>OCR Text</InputLabel>
                      <Select
                        value={hasOcr}
                        onChange={handleOcrChange}
                        label="OCR Text"
                      >
                        <MenuItem value="all">All Documents</MenuItem>
                        <MenuItem value="yes">Has OCR Text</MenuItem>
                        <MenuItem value="no">No OCR Text</MenuItem>
                      </Select>
                    </FormControl>
                  </AccordionDetails>
                </Accordion>

                {/* Date Range Filter */}
                <Accordion>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">Date Range</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <Typography variant="body2" color="text.secondary" gutterBottom>
                      Days ago: {dateRange[0]} - {dateRange[1]}
                    </Typography>
                    <Slider
                      value={dateRange}
                      onChange={(e, newValue) => setDateRange(newValue as number[])}
                      valueLabelDisplay="auto"
                      min={0}
                      max={365}
                      marks={[
                        { value: 0, label: 'Today' },
                        { value: 30, label: '30d' },
                        { value: 90, label: '90d' },
                        { value: 365, label: '1y' },
                      ]}
                    />
                  </AccordionDetails>
                </Accordion>

                {/* File Size Filter */}
                <Accordion>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">File Size</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <Typography variant="body2" color="text.secondary" gutterBottom>
                      Size: {fileSizeRange[0]}MB - {fileSizeRange[1]}MB
                    </Typography>
                    <Slider
                      value={fileSizeRange}
                      onChange={(e, newValue) => setFileSizeRange(newValue as number[])}
                      valueLabelDisplay="auto"
                      min={0}
                      max={100}
                      marks={[
                        { value: 0, label: '0MB' },
                        { value: 10, label: '10MB' },
                        { value: 50, label: '50MB' },
                        { value: 100, label: '100MB' },
                      ]}
                    />
                  </AccordionDetails>
                </Accordion>
              </Stack>
            </CardContent>
          </Card>
        </Grid>

        {/* Search Results */}
        <Grid item xs={12} md={9}>

          {/* Toolbar */}
          {searchQuery && (
            <Box sx={{ 
              mb: 3, 
              display: 'flex', 
              justifyContent: 'space-between',
              alignItems: 'center',
            }}>
              <Typography variant="body2" color="text.secondary">
                {loading ? 'Searching...' : `${searchResults.length} results found`}
              </Typography>
              
              <ToggleButtonGroup
                value={viewMode}
                exclusive
                onChange={handleViewModeChange}
                size="small"
              >
                <ToggleButton value="grid">
                  <GridViewIcon />
                </ToggleButton>
                <ToggleButton value="list">
                  <ListViewIcon />
                </ToggleButton>
              </ToggleButtonGroup>
            </Box>
          )}

          {/* Results */}
          {loading && (
            <Box display="flex" justifyContent="center" alignItems="center" minHeight="200px">
              <CircularProgress />
            </Box>
          )}

          {error && (
            <Alert severity="error" sx={{ mb: 3 }}>
              {error}
            </Alert>
          )}

          {!loading && !error && searchQuery && searchResults.length === 0 && (
            <Box 
              sx={{ 
                textAlign: 'center', 
                py: 8,
                background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.05) 0%, rgba(139, 92, 246, 0.05) 100%)',
                borderRadius: 2,
                border: '1px dashed',
                borderColor: 'primary.main',
              }}
            >
              <Typography variant="h6" color="text.secondary" gutterBottom>
                No results found for "{searchQuery}"
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                Try adjusting your search terms or filters
              </Typography>
              
              {/* Helpful suggestions for no results */}
              <Box sx={{ mb: 2 }}>
                <Typography variant="subtitle2" color="text.primary" gutterBottom>
                  Suggestions:
                </Typography>
                <Stack spacing={1} alignItems="center">
                  <Typography variant="body2" color="text.secondary">• Try simpler or more general terms</Typography>
                  <Typography variant="body2" color="text.secondary">• Check spelling and try different keywords</Typography>
                  <Typography variant="body2" color="text.secondary">• Remove some filters to broaden your search</Typography>
                  <Typography variant="body2" color="text.secondary">• Use quotes for exact phrases</Typography>
                </Stack>
              </Box>
              
              <Stack direction="row" spacing={1} justifyContent="center" flexWrap="wrap">
                <Button 
                  size="small" 
                  variant="outlined" 
                  onClick={handleClearFilters}
                  startIcon={<ClearIcon />}
                >
                  Clear Filters
                </Button>
                <Button 
                  size="small" 
                  variant="outlined" 
                  onClick={() => setSearchQuery('')}
                >
                  New Search
                </Button>
              </Stack>
            </Box>
          )}

          {!loading && !error && !searchQuery && (
            <Box 
              sx={{ 
                textAlign: 'center', 
                py: 8,
                background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.05) 0%, rgba(139, 92, 246, 0.05) 100%)',
                borderRadius: 2,
              }}
            >
              <SearchIcon sx={{ fontSize: 64, color: 'primary.main', mb: 2 }} />
              <Typography variant="h6" color="text.secondary" gutterBottom>
                Start searching your documents
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                Use the enhanced search bar above to find documents by content, filename, or tags
              </Typography>
              
              {/* Search Tips */}
              <Box sx={{ mb: 3, maxWidth: 600, mx: 'auto' }}>
                <Typography variant="subtitle2" color="text.primary" gutterBottom>
                  Search Tips:
                </Typography>
                <Stack spacing={1} alignItems="center">
                  {searchTips.map((tip, index) => (
                    <Typography key={index} variant="body2" color="text.secondary" sx={{ fontSize: '0.85rem' }}>
                      {tip}
                    </Typography>
                  ))}
                </Stack>
              </Box>
              
              <Stack direction="row" spacing={1} justifyContent="center" flexWrap="wrap">
                <Chip 
                  label="Try: invoice" 
                  size="small" 
                  variant="outlined" 
                  clickable
                  onClick={() => setSearchQuery('invoice')}
                />
                <Chip 
                  label="Try: contract" 
                  size="small" 
                  variant="outlined" 
                  clickable
                  onClick={() => setSearchQuery('contract')}
                />
                <Chip 
                  label="Try: tag:important" 
                  size="small" 
                  variant="outlined" 
                  clickable
                  onClick={() => setSearchQuery('tag:important')}
                />
              </Stack>
            </Box>
          )}

          {!loading && !error && searchResults.length > 0 && (
            <Grid container spacing={viewMode === 'grid' ? 3 : 1}>
              {searchResults.map((doc) => (
                <Grid 
                  item 
                  xs={12} 
                  sm={viewMode === 'grid' ? 6 : 12} 
                  md={viewMode === 'grid' ? 6 : 12} 
                  lg={viewMode === 'grid' ? 4 : 12}
                  key={doc.id}
                >
                  <Card 
                    className="search-result-card search-loading-fade"
                    sx={{ 
                      height: '100%',
                      display: 'flex',
                      flexDirection: viewMode === 'list' ? 'row' : 'column',
                    }}
                  >
                    {viewMode === 'grid' && (
                      <Box
                        sx={{
                          height: 100,
                          display: 'flex',
                          alignItems: 'center',
                          justifyContent: 'center',
                          background: 'linear-gradient(135deg, rgba(99, 102, 241, 0.1) 0%, rgba(139, 92, 246, 0.1) 100%)',
                        }}
                      >
                        <Box sx={{ fontSize: '2.5rem' }}>
                          {getFileIcon(doc.mime_type)}
                        </Box>
                      </Box>
                    )}
                    
                    <CardContent className="search-card" sx={{ flexGrow: 1 }}>
                      <Box sx={{ display: 'flex', alignItems: 'flex-start', gap: 1 }}>
                        {viewMode === 'list' && (
                          <Box sx={{ mr: 1, mt: 0.5 }}>
                            {getFileIcon(doc.mime_type)}
                          </Box>
                        )}
                        
                        <Box sx={{ flexGrow: 1, minWidth: 0, pr: 1 }}>
                          <Typography 
                            variant="h6" 
                            sx={{ 
                              fontSize: '0.95rem',
                              fontWeight: 600,
                              mb: 1,
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              whiteSpace: 'nowrap',
                            }}
                            title={doc.original_filename}
                          >
                            {doc.original_filename}
                          </Typography>
                          
                          <Stack direction="row" spacing={1} sx={{ mb: 1, flexWrap: 'wrap', gap: 0.5 }}>
                            <Chip 
                              className="search-chip"
                              label={formatFileSize(doc.file_size)} 
                              size="small" 
                              variant="outlined"
                            />
                            <Chip 
                              className="search-chip"
                              label={formatDate(doc.created_at)} 
                              size="small" 
                              variant="outlined"
                            />
                            {doc.has_ocr_text && (
                              <Chip 
                                className="search-chip"
                                label="OCR" 
                                size="small" 
                                color="success"
                                variant="outlined"
                              />
                            )}
                          </Stack>
                          
                          {doc.tags.length > 0 && (
                            <Stack direction="row" spacing={0.5} sx={{ mb: 1, flexWrap: 'wrap' }}>
                              {doc.tags.slice(0, 2).map((tag, index) => (
                                <Chip 
                                  key={index}
                                  className="search-chip"
                                  label={tag} 
                                  size="small" 
                                  color="primary"
                                  variant="outlined"
                                  sx={{ fontSize: '0.7rem', height: '18px' }}
                                />
                              ))}
                              {doc.tags.length > 2 && (
                                <Chip 
                                  className="search-chip"
                                  label={`+${doc.tags.length - 2}`}
                                  size="small" 
                                  variant="outlined"
                                  sx={{ fontSize: '0.7rem', height: '18px' }}
                                />
                              )}
                            </Stack>
                          )}

                          {/* Enhanced Search Snippets */}
                          {doc.snippets && doc.snippets.length > 0 && (
                            <Box sx={{ mt: 2, mb: 1 }}>
                              <EnhancedSnippetViewer
                                snippets={doc.snippets}
                                searchQuery={searchQuery}
                                maxSnippetsToShow={2}
                                onSnippetClick={(snippet, index) => {
                                  // Could navigate to document with snippet highlighted
                                  console.log('Snippet clicked:', snippet, index);
                                }}
                              />
                            </Box>
                          )}

                          {/* Search Rank */}
                          {doc.search_rank && (
                            <Box sx={{ mt: 1 }}>
                              <Chip 
                                className="search-chip"
                                label={`Relevance: ${(doc.search_rank * 100).toFixed(1)}%`}
                                size="small" 
                                color="info"
                                variant="outlined"
                                sx={{ fontSize: '0.7rem', height: '18px' }}
                              />
                            </Box>
                          )}
                        </Box>
                        
                        <Tooltip title="View Details">
                          <IconButton
                            className="search-filter-button search-focusable"
                            size="small"
                            onClick={() => navigate(`/documents/${doc.id}`)}
                          >
                            <ViewIcon />
                          </IconButton>
                        </Tooltip>
                        <Tooltip title="Download">
                          <IconButton
                            className="search-filter-button search-focusable"
                            size="small"
                            onClick={() => handleDownload(doc)}
                          >
                            <DownloadIcon />
                          </IconButton>
                        </Tooltip>
                      </Box>
                    </CardContent>
                  </Card>
                </Grid>
              ))}
            </Grid>
          )}
        </Grid>
      </Grid>
    </Box>
  );
};

export default SearchPage;