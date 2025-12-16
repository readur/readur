import React, { useState, useEffect, useCallback } from 'react';
import { useNavigate, useSearchParams } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import {
  Box,
  Typography,
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
  Menu,
  RadioGroup,
  Radio,
  Pagination,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  Search as SearchIcon,
  FilterList as FilterIcon,
  Clear as ClearIcon,
  ExpandMore as ExpandMoreIcon,
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
  TextFormat as TextFormatIcon,
} from '@mui/icons-material';
import { documentService, SearchRequest, api } from '../services/api';
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

type SnippetViewMode = 'compact' | 'detailed' | 'context';
type SnippetHighlightStyle = 'background' | 'underline' | 'bold';

interface SnippetSettings {
  viewMode: SnippetViewMode;
  highlightStyle: SnippetHighlightStyle;
  fontSize: number;
  contextLength: number;
  maxSnippetsToShow: number;
}

const SearchPage: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [searchParams, setSearchParams] = useSearchParams();
  const [searchQuery, setSearchQuery] = useState<string>(searchParams.get('q') || '');
  const [searchResults, setSearchResults] = useState<Document[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [error, setError] = useState<string | null>(null);
  const [queryTime, setQueryTime] = useState<number>(0);
  const [totalResults, setTotalResults] = useState<number>(0);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [isTyping, setIsTyping] = useState<boolean>(false);
  const [searchProgress, setSearchProgress] = useState<number>(0);
  const [quickSuggestions, setQuickSuggestions] = useState<string[]>([]);
  const [showFilters, setShowFilters] = useState<boolean>(false);
  const searchTips = [
    t('search.tips.exactPhrase'),
    t('search.tips.tags'),
    t('search.tips.combine'),
    t('search.tips.wildcards')
  ];
  
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
  
  // Global snippet settings
  const [snippetSettings, setSnippetSettings] = useState<SnippetSettings>({
    viewMode: 'detailed',
    highlightStyle: 'background',
    fontSize: 15,
    contextLength: 50,
    maxSnippetsToShow: 3,
  });
  const [snippetSettingsAnchor, setSnippetSettingsAnchor] = useState<null | HTMLElement>(null);
  
  // Pagination states
  const [currentPage, setCurrentPage] = useState<number>(1);
  const [resultsPerPage] = useState<number>(20);
  
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

  const performSearch = useCallback(async (query: string, filters: SearchFilters = {}, page: number = 1): Promise<void> => {
    const hasFilters = (filters.tags?.length ?? 0) > 0 ||
                       (filters.mimeTypes?.length ?? 0) > 0;
    if (!query.trim() && !hasFilters) {
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
        limit: resultsPerPage,
        offset: (page - 1) * resultsPerPage,
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

      // Clear progress after a brief delay
      setTimeout(() => setSearchProgress(0), 500);
      
    } catch (err) {
      setSearchProgress(0);
      setError('Search failed. Please try again.');
      console.error(err);
    } finally {
      setLoading(false);
    }
  }, [advancedSettings]);

  const debouncedSearch = useCallback(
    debounce((query: string, filters: SearchFilters, page: number = 1, resetPage: boolean = false) => {
      if (resetPage) {
        setCurrentPage(1);
        performSearch(query, filters, 1);
      } else {
        setCurrentPage(page);
        performSearch(query, filters, page);
      }
    }, 300),
    [performSearch]
  );
  
  const quickSuggestionsDebounced = useCallback(
    debounce((query: string) => generateQuickSuggestions(query), 150),
    [generateQuickSuggestions]
  );

  // Load available tags from labels API on component mount
  useEffect(() => {
    const loadLabels = async () => {
      try {
        const response = await api.get('/labels?include_counts=true');
        const labelNames = (response.data || []).map((label: any) => label.name);
        setAvailableTags(labelNames);
      } catch (error) {
        console.error('Failed to load labels:', error);
      }
    };
    loadLabels();
  }, []);

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
    // Reset to page 1 when search query or filters change
    const shouldResetPage = searchQuery !== searchParams.get('q') || 
                           JSON.stringify(filters) !== JSON.stringify({
                             tags: selectedTags,
                             mimeTypes: selectedMimeTypes,
                             dateRange: dateRange,
                             fileSizeRange: fileSizeRange,
                             hasOcr: hasOcr,
                           });
    
    debouncedSearch(searchQuery, filters, 1, shouldResetPage);
    quickSuggestionsDebounced(searchQuery);
    
    if (shouldResetPage) {
      setCurrentPage(1);
    }
    
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
    setCurrentPage(1);
  };

  const getFileIcon = (mimeType: string): React.ReactElement => {
    if (mimeType.includes('pdf')) return <PdfIcon color="error" sx={{ fontSize: '1.2rem' }} />;
    if (mimeType.includes('image')) return <ImageIcon color="primary" sx={{ fontSize: '1.2rem' }} />;
    if (mimeType.includes('text')) return <TextIcon color="info" sx={{ fontSize: '1.2rem' }} />;
    return <DocIcon color="secondary" sx={{ fontSize: '1.2rem' }} />;
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
      const link = window.document.createElement('a');
      link.href = url;
      link.setAttribute('download', doc.original_filename);
      window.document.body.appendChild(link);
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


  const handleSearchModeChange = (event: React.MouseEvent<HTMLElement>, newMode: SearchMode | null): void => {
    if (newMode) {
      setAdvancedSettings(prev => ({ ...prev, searchMode: newMode }));
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

  const handlePageChange = (event: React.ChangeEvent<unknown>, page: number): void => {
    setCurrentPage(page);
    const filters: SearchFilters = {
      tags: selectedTags,
      mimeTypes: selectedMimeTypes,
      dateRange: dateRange,
      fileSizeRange: fileSizeRange,
      hasOcr: hasOcr,
    };
    performSearch(searchQuery, filters, page);
    
    // Scroll to top of results
    const resultsElement = document.querySelector('.search-results-container');
    if (resultsElement) {
      resultsElement.scrollIntoView({ behavior: 'smooth', block: 'start' });
    }
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
          {t('search.title')}
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
              placeholder={t('search.placeholder')}
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
                      <Tooltip title={t('search.settings.title')}>
                        <IconButton
                          size="small"
                          onClick={() => setShowAdvanced(!showAdvanced)}
                          color={showAdvanced ? 'primary' : 'default'}
                        >
                          <SettingsIcon />
                        </IconButton>
                      </Tooltip>
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
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, alignItems: 'center' }}>
                <Chip
                  icon={<TrendingIcon />}
                  label={t('search.status.resultsFound', { count: totalResults })} 
                  size="small" 
                  color="primary"
                  variant="outlined"
                  sx={{ flexShrink: 0 }}
                />
                <Chip 
                  icon={<TimeIcon />}
                  label={`${queryTime}ms`} 
                  size="small" 
                  variant="outlined"
                  sx={{ flexShrink: 0 }}
                />
                {advancedSettings.useEnhancedSearch && (
                  <Chip
                    icon={<SpeedIcon />}
                    label={t('search.modes.enhanced')} 
                    size="small" 
                    color="success"
                    variant="outlined"
                    sx={{ flexShrink: 0 }}
                  />
                )}
              </Box>
              
              {/* Simplified Search Mode Selector */}
              <Box sx={{ display: 'flex', gap: 1, alignItems: 'center' }}>
                <ToggleButtonGroup
                  value={advancedSettings.searchMode}
                  exclusive
                  onChange={handleSearchModeChange}
                  size="small"
                >
                  <ToggleButton value="simple">{t('search.modes.smart')}</ToggleButton>
                  <ToggleButton value="phrase">{t('search.modes.exactPhrase')}</ToggleButton>
                  <ToggleButton value="fuzzy">{t('search.modes.similarWords')}</ToggleButton>
                  <ToggleButton value="boolean">{t('search.modes.advanced')}</ToggleButton>
                </ToggleButtonGroup>
              </Box>
            </Box>
          )}

          {/* Quick Suggestions */}
          {quickSuggestions.length > 0 && searchQuery && !loading && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="body2" color="text.secondary" gutterBottom>
                {t('search.quickSuggestions.title')}
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
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
                      flexShrink: 0,
                      '&:hover': { 
                        backgroundColor: 'primary.main',
                        color: 'primary.contrastText',
                      }
                    }}
                  />
                ))}
              </Box>
            </Box>
          )}

          {/* Server Suggestions */}
          {suggestions.length > 0 && (
            <Box sx={{ mt: 2 }}>
              <Typography variant="body2" color="text.secondary" gutterBottom>
                {t('search.relatedSearches.title')}
              </Typography>
              <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1 }}>
                {suggestions.map((suggestion, index) => (
                  <Chip
                    key={index}
                    label={suggestion}
                    size="small"
                    onClick={() => handleSuggestionClick(suggestion)}
                    clickable
                    variant="outlined"
                    sx={{ 
                      flexShrink: 0,
                      '&:hover': { 
                        backgroundColor: 'primary.light',
                        color: 'primary.contrastText',
                      }
                    }}
                  />
                ))}
              </Box>
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
                    {t('search.filters.title')}
                  </Typography>
                  <Button size="small" onClick={handleClearFilters} startIcon={<ClearIcon />}>
                    {t('common.actions.clear')}
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
                  {t('search.filters.title')}
                </Typography>
                <Button size="small" onClick={handleClearFilters} startIcon={<ClearIcon />}>
                  {t('common.actions.clear')}
                </Button>
              </Box>

              <Stack spacing={3}>
                {/* Tags Filter */}
                <Accordion defaultExpanded>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">{t('search.filters.tags')}</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <FormControl fullWidth size="small">
                      <InputLabel>{t('search.filters.selectTags')}</InputLabel>
                      <Select<string[]>
                        multiple
                        value={selectedTags}
                        onChange={handleTagsChange}
                        input={<OutlinedInput label={t('search.filters.selectTags')} />}
                        renderValue={(selected) => (
                          <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 0.5, overflow: 'hidden' }}>
                            {selected.map((value) => (
                              <Chip 
                                key={value} 
                                label={value} 
                                size="small" 
                                sx={{ 
                                  flexShrink: 0,
                                  maxWidth: '100px',
                                  '& .MuiChip-label': {
                                    overflow: 'hidden',
                                    textOverflow: 'ellipsis',
                                    whiteSpace: 'nowrap',
                                  }
                                }}
                              />
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
                    <Typography variant="subtitle2">{t('search.filters.ocrStatus')}</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <FormControl fullWidth size="small">
                      <InputLabel>{t('search.filters.ocrText')}</InputLabel>
                      <Select
                        value={hasOcr}
                        onChange={handleOcrChange}
                        label={t('search.filters.ocrText')}
                      >
                        <MenuItem value="all">{t('search.filters.allDocuments')}</MenuItem>
                        <MenuItem value="yes">{t('search.filters.hasOcrText')}</MenuItem>
                        <MenuItem value="no">{t('search.filters.noOcrText')}</MenuItem>
                      </Select>
                    </FormControl>
                  </AccordionDetails>
                </Accordion>

                {/* Date Range Filter */}
                <Accordion>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">{t('search.filters.dateRange')}</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <Typography variant="body2" color="text.secondary" gutterBottom>
                      {t('search.filters.daysAgo', { min: dateRange[0], max: dateRange[1] })}
                    </Typography>
                    <Slider
                      value={dateRange}
                      onChange={(e, newValue) => setDateRange(newValue as number[])}
                      valueLabelDisplay="auto"
                      min={0}
                      max={365}
                      marks={[
                        { value: 0, label: t('search.filters.dateMarks.today') },
                        { value: 30, label: t('search.filters.dateMarks.30d') },
                        { value: 90, label: t('search.filters.dateMarks.90d') },
                        { value: 365, label: t('search.filters.dateMarks.1y') },
                      ]}
                    />
                  </AccordionDetails>
                </Accordion>

                {/* File Size Filter */}
                <Accordion>
                  <AccordionSummary expandIcon={<ExpandMoreIcon />}>
                    <Typography variant="subtitle2">{t('search.filters.fileSize')}</Typography>
                  </AccordionSummary>
                  <AccordionDetails>
                    <Typography variant="body2" color="text.secondary" gutterBottom>
                      {t('search.filters.sizeRange', { min: fileSizeRange[0], max: fileSizeRange[1] })}
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
        <Grid item xs={12} md={9} className="search-results-container">

          {/* Results Header */}
          {searchQuery && (
            <Box sx={{ mb: 3 }}>
              <Box sx={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: 2, mb: 1 }}>
                <Typography variant="body2" color="text.secondary">
                  {loading ? t('search.status.searching') : t('search.status.resultsFound', { count: searchResults.length })}
                </Typography>
                
                {/* Snippet Settings Button */}
                <Button
                  variant="outlined"
                  size="small"
                  startIcon={<TextFormatIcon />}
                  onClick={(e) => setSnippetSettingsAnchor(e.currentTarget)}
                  sx={{
                    flexShrink: 0,
                    position: 'relative',
                  }}
                >
                  {t('search.display.settings')}
                  {/* Show indicator if settings are customized */}
                  {(snippetSettings.viewMode !== 'detailed' || 
                    snippetSettings.highlightStyle !== 'background' || 
                    snippetSettings.fontSize !== 15 || 
                    snippetSettings.maxSnippetsToShow !== 3) && (
                    <Box
                      sx={{
                        position: 'absolute',
                        top: -4,
                        right: -4,
                        width: 8,
                        height: 8,
                        borderRadius: '50%',
                        bgcolor: 'primary.main',
                      }}
                    />
                  )}
                </Button>
              </Box>
              
              {/* Current Settings Preview */}
              {!loading && searchResults.length > 0 && (
                <Box sx={{ display: 'flex', flexWrap: 'wrap', gap: 1, alignItems: 'center' }}>
                  <Typography variant="caption" color="text.secondary">
                    {t('search.results.showing')}
                  </Typography>
                  <Chip
                    label={t('search.results.snippetsCount', { count: snippetSettings.maxSnippetsToShow })} 
                    size="small" 
                    variant="outlined"
                    sx={{ fontSize: '0.7rem' }}
                  />
                  <Chip
                    label={t('search.results.fontSize', { size: snippetSettings.fontSize })} 
                    size="small" 
                    variant="outlined"
                    sx={{ fontSize: '0.7rem' }}
                  />
                  <Chip 
                    label={snippetSettings.viewMode} 
                    size="small" 
                    variant="outlined"
                    sx={{ fontSize: '0.7rem', textTransform: 'capitalize' }}
                  />
                </Box>
              )}
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
                {t('search.noResults.title', { query: searchQuery })}
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                {t('search.noResults.subtitle')}
              </Typography>

              {/* Helpful suggestions for no results */}
              <Box sx={{ mb: 2 }}>
                <Typography variant="subtitle2" color="text.primary" gutterBottom>
                  {t('search.noResults.suggestions.title')}
                </Typography>
                <Stack spacing={1} alignItems="center">
                  <Typography variant="body2" color="text.secondary">• {t('search.noResults.suggestions.simpler')}</Typography>
                  <Typography variant="body2" color="text.secondary">• {t('search.noResults.suggestions.spelling')}</Typography>
                  <Typography variant="body2" color="text.secondary">• {t('search.noResults.suggestions.removeFilters')}</Typography>
                  <Typography variant="body2" color="text.secondary">• {t('search.noResults.suggestions.useQuotes')}</Typography>
                </Stack>
              </Box>
              
              <Stack direction="row" spacing={1} justifyContent="center" flexWrap="wrap">
                <Button
                  size="small"
                  variant="outlined"
                  onClick={handleClearFilters}
                  startIcon={<ClearIcon />}
                >
                  {t('search.actions.clearFilters')}
                </Button>
                <Button
                  size="small"
                  variant="outlined"
                  onClick={() => setSearchQuery('')}
                >
                  {t('search.actions.newSearch')}
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
                {t('search.empty.title')}
              </Typography>
              <Typography variant="body2" color="text.secondary" sx={{ mb: 3 }}>
                {t('search.empty.subtitle')}
              </Typography>

              {/* Search Tips */}
              <Box sx={{ mb: 3, maxWidth: 600, mx: 'auto' }}>
                <Typography variant="subtitle2" color="text.primary" gutterBottom>
                  {t('search.tips.title')}
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
                  label={t('search.examples.invoice')} 
                  size="small" 
                  variant="outlined" 
                  clickable
                  onClick={() => {
                    setSearchQuery('invoice');
                    setCurrentPage(1);
                  }}
                />
                <Chip
                  label={t('search.examples.contract')} 
                  size="small" 
                  variant="outlined" 
                  clickable
                  onClick={() => {
                    setSearchQuery('contract');
                    setCurrentPage(1);
                  }}
                />
                <Chip
                  label={t('search.examples.tagImportant')} 
                  size="small" 
                  variant="outlined" 
                  clickable
                  onClick={() => {
                    setSearchQuery('tag:important');
                    setCurrentPage(1);
                  }}
                />
              </Stack>
            </Box>
          )}

          {!loading && !error && searchResults.length > 0 && (
            <>
              <Grid container spacing={1}>
                {searchResults.map((doc) => (
                <Grid 
                  item 
                  xs={12} 
                  key={doc.id}
                >
                  <Card 
                    className="search-result-card search-loading-fade"
                    sx={{ 
                      height: '100%',
                      display: 'flex',
                      flexDirection: 'row',
                    }}
                  >
                    
                    <CardContent 
                      className="search-card" 
                      sx={{ 
                        flexGrow: 1, 
                        overflow: 'hidden',
                        py: 1.5,
                        px: 2,
                        '&:last-child': {
                          pb: 1.5
                        }
                      }}
                    >
                      <Box sx={{ 
                        display: 'flex', 
                        alignItems: 'center', 
                        gap: 1, 
                        width: '100%'
                      }}>
                        <Box sx={{ mr: 1.5, mt: 0.5, flexShrink: 0 }}>
                          {getFileIcon(doc.mime_type)}
                        </Box>
                        
                        <Box sx={{ flexGrow: 1, minWidth: 0, overflow: 'hidden' }}>
                          <Typography 
                            variant="h6" 
                            sx={{ 
                              fontSize: '1.05rem',
                              fontWeight: 600,
                              mb: 1,
                              overflow: 'hidden',
                              textOverflow: 'ellipsis',
                              whiteSpace: 'nowrap',
                              display: 'block',
                              width: '100%',
                              color: 'text.primary',
                            }}
                            title={doc.original_filename}
                          >
                            {doc.original_filename}
                          </Typography>
                          
                          <Box sx={{ 
                            mb: 0.5, 
                            display: 'flex', 
                            flexWrap: 'wrap', 
                            gap: 0.75, 
                            overflow: 'hidden',
                            alignItems: 'center',
                          }}>
                            <Typography variant="caption" color="text.secondary" sx={{ mr: 1 }}>
                              {formatFileSize(doc.file_size)} • {formatDate(doc.created_at)}
                              {doc.has_ocr_text && t('search.results.hasOcr')}
                            </Typography>
                          </Box>
                          
                          {doc.tags.length > 0 && (
                            <Box sx={{ 
                              mb: 1, 
                              display: 'flex', 
                              flexWrap: 'wrap', 
                              gap: 0.5, 
                              overflow: 'hidden',
                              alignItems: 'center',
                            }}>
                              <Typography variant="caption" color="text.secondary" sx={{ mr: 0.5 }}>
                                {t('search.results.tags')}
                              </Typography>
                              {doc.tags.slice(0, 3).map((tag, index) => (
                                <Chip 
                                  key={index}
                                  className="search-chip"
                                  label={tag} 
                                  size="small" 
                                  color="primary"
                                  variant="outlined"
                                  sx={{ 
                                    fontSize: '0.7rem', 
                                    height: '18px',
                                    flexShrink: 0,
                                    maxWidth: '120px',
                                    '& .MuiChip-label': {
                                      overflow: 'hidden',
                                      textOverflow: 'ellipsis',
                                      whiteSpace: 'nowrap',
                                    }
                                  }}
                                />
                              ))}
                              {doc.tags.length > 3 && (
                                <Typography variant="caption" color="text.secondary">
                                  {t('common.moreCount', { count: doc.tags.length - 3 })}
                                </Typography>
                              )}
                            </Box>
                          )}

                          {/* Enhanced Search Snippets */}
                          {doc.snippets && doc.snippets.length > 0 && (
                            <Box sx={{ 
                              mt: 0.5, 
                              mb: 1 
                            }}>
                              <EnhancedSnippetViewer
                                snippets={doc.snippets}
                                searchQuery={searchQuery}
                                maxSnippetsToShow={snippetSettings.maxSnippetsToShow}
                                viewMode={snippetSettings.viewMode}
                                highlightStyle={snippetSettings.highlightStyle}
                                fontSize={snippetSettings.fontSize}
                                contextLength={snippetSettings.contextLength}
                                showSettings={false}
                                onSnippetClick={(snippet, index) => {
                                  console.log('Snippet clicked:', snippet, index);
                                }}
                              />
                            </Box>
                          )}

                        </Box>
                        
                        <Box sx={{ 
                          display: 'flex', 
                          flexDirection: 'column',
                          flexShrink: 0, 
                          ml: 2,
                          gap: 0.5,
                          alignItems: 'center',
                          justifyContent: 'flex-start',
                          pt: 0.5,
                        }}>
                          <Tooltip title={t('common.actions.viewDetails')}>
                            <IconButton
                              className="search-filter-button search-focusable"
                              size="small"
                              sx={{
                                p: 0.75,
                                minWidth: 32,
                                minHeight: 32,
                                bgcolor: 'primary.main',
                                color: 'primary.contrastText',
                                '&:hover': {
                                  bgcolor: 'primary.dark',
                                }
                              }}
                              onClick={() => navigate(`/documents/${doc.id}`)}
                            >
                              <ViewIcon sx={{ fontSize: '1.1rem' }} />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title={t('common.actions.download')}>
                            <IconButton
                              className="search-filter-button search-focusable"
                              size="small"
                              sx={{
                                p: 0.75,
                                minWidth: 32,
                                minHeight: 32,
                                bgcolor: 'action.hover',
                                '&:hover': {
                                  bgcolor: 'action.selected',
                                }
                              }}
                              onClick={() => handleDownload(doc)}
                            >
                              <DownloadIcon sx={{ fontSize: '1.1rem' }} />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      </Box>
                    </CardContent>
                  </Card>
                </Grid>
                ))}
              </Grid>
              
              {/* Pagination */}
              {totalResults > resultsPerPage && (
                <Box sx={{ display: 'flex', justifyContent: 'center', mt: 4, mb: 2 }}>
                  <Pagination
                    count={Math.ceil(totalResults / resultsPerPage)}
                    page={currentPage}
                    onChange={handlePageChange}
                    color="primary"
                    size="large"
                    showFirstButton
                    showLastButton
                    siblingCount={1}
                    boundaryCount={1}
                    sx={{
                      '& .MuiPagination-ul': {
                        flexWrap: 'wrap',
                        justifyContent: 'center',
                      },
                    }}
                  />
                </Box>
              )}
              
              {/* Results Summary */}
              <Box sx={{ textAlign: 'center', mt: 2, mb: 2 }}>
                <Typography variant="body2" color="text.secondary">
                  {t('search.results.pagination', {
                    start: ((currentPage - 1) * resultsPerPage) + 1,
                    end: Math.min(currentPage * resultsPerPage, totalResults),
                    total: totalResults
                  })}
                </Typography>
              </Box>
            </>
          )}
        </Grid>
      </Grid>
      
      {/* Global Snippet Settings Menu */}
      <Menu
        anchorEl={snippetSettingsAnchor}
        open={Boolean(snippetSettingsAnchor)}
        onClose={() => setSnippetSettingsAnchor(null)}
        PaperProps={{ sx: { width: 320, p: 2 } }}
      >
        <Typography variant="subtitle2" sx={{ mb: 2 }}>
          {t('search.display.textSettings')}
        </Typography>
        
        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            {t('search.display.viewMode.label')}
          </Typography>
          <RadioGroup
            value={snippetSettings.viewMode}
            onChange={(e) => setSnippetSettings(prev => ({ ...prev, viewMode: e.target.value as SnippetViewMode }))}
          >
            <FormControlLabel
              value="compact"
              control={<Radio size="small" />}
              label={t('search.display.viewMode.compact')}
            />
            <FormControlLabel
              value="detailed"
              control={<Radio size="small" />}
              label={t('search.display.viewMode.detailed')}
            />
            <FormControlLabel
              value="context"
              control={<Radio size="small" />}
              label={t('search.display.viewMode.contextFocus')}
            />
          </RadioGroup>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            {t('search.display.highlightStyle.label')}
          </Typography>
          <RadioGroup
            value={snippetSettings.highlightStyle}
            onChange={(e) => setSnippetSettings(prev => ({ ...prev, highlightStyle: e.target.value as SnippetHighlightStyle }))}
          >
            <FormControlLabel
              value="background"
              control={<Radio size="small" />}
              label={t('search.display.highlightStyle.background')}
            />
            <FormControlLabel
              value="underline"
              control={<Radio size="small" />}
              label={t('search.display.highlightStyle.underline')}
            />
            <FormControlLabel
              value="bold"
              control={<Radio size="small" />}
              label={t('search.display.highlightStyle.bold')}
            />
          </RadioGroup>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            {t('search.display.fontSizeLabel', { size: snippetSettings.fontSize })}
          </Typography>
          <Slider
            value={snippetSettings.fontSize}
            onChange={(_, value) => setSnippetSettings(prev => ({ ...prev, fontSize: value as number }))}
            min={12}
            max={20}
            marks
            valueLabelDisplay="auto"
          />
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            {t('search.display.snippetsPerResult', { count: snippetSettings.maxSnippetsToShow })}
          </Typography>
          <Slider
            value={snippetSettings.maxSnippetsToShow}
            onChange={(_, value) => setSnippetSettings(prev => ({ ...prev, maxSnippetsToShow: value as number }))}
            min={1}
            max={5}
            marks
            valueLabelDisplay="auto"
          />
        </Box>

        {snippetSettings.viewMode === 'context' && (
          <>
            <Divider sx={{ my: 2 }} />
            <Box>
              <Typography variant="caption" color="text.secondary" gutterBottom>
                {t('search.display.contextLength', { length: snippetSettings.contextLength })}
              </Typography>
              <Slider
                value={snippetSettings.contextLength}
                onChange={(_, value) => setSnippetSettings(prev => ({ ...prev, contextLength: value as number }))}
                min={20}
                max={200}
                step={10}
                marks
                valueLabelDisplay="auto"
              />
            </Box>
          </>
        )}
      </Menu>
    </Box>
  );
};

export default SearchPage;