import React, { useState, useCallback, useRef, useEffect } from 'react';
import {
  Box,
  TextField,
  InputAdornment,
  IconButton,
  Paper,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Typography,
  Chip,
  Stack,
  ClickAwayListener,
  Grow,
  Popper,
  CircularProgress,
  LinearProgress,
  Skeleton,
  SxProps,
  Theme,
  useTheme,
} from '@mui/material';
import {
  Search as SearchIcon,
  Clear as ClearIcon,
  Description as DocIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  TextSnippet as TextIcon,
  TrendingUp as TrendingIcon,
  AccessTime as TimeIcon,
} from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { documentService, SearchRequest, EnhancedDocument, SearchResponse } from '../../services/api';

interface GlobalSearchBarProps {
  sx?: SxProps<Theme>;
  [key: string]: any;
}

const GlobalSearchBar: React.FC<GlobalSearchBarProps> = ({ sx, ...props }) => {
  const navigate = useNavigate();
  const theme = useTheme();
  const [query, setQuery] = useState<string>('');
  const [results, setResults] = useState<EnhancedDocument[]>([]);
  const [loading, setLoading] = useState<boolean>(false);
  const [showResults, setShowResults] = useState<boolean>(false);
  const [recentSearches, setRecentSearches] = useState<string[]>([]);
  const [isTyping, setIsTyping] = useState<boolean>(false);
  const [searchProgress, setSearchProgress] = useState<number>(0);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [popularSearches] = useState<string[]>(['invoice', 'contract', 'report', 'presentation', 'agreement']);
  const searchInputRef = useRef<HTMLInputElement>(null);
  const anchorRef = useRef<HTMLDivElement>(null);

  // Load recent searches from localStorage
  useEffect(() => {
    const saved = localStorage.getItem('recentSearches');
    if (saved) {
      try {
        setRecentSearches(JSON.parse(saved));
      } catch (e) {
        console.error('Failed to parse recent searches:', e);
      }
    }
  }, []);

  // Save recent searches to localStorage
  const saveRecentSearch = useCallback((searchQuery: string): void => {
    if (!searchQuery.trim()) return;
    
    const updated = [
      searchQuery,
      ...recentSearches.filter(s => s !== searchQuery)
    ].slice(0, 5); // Keep only last 5 searches
    
    setRecentSearches(updated);
    localStorage.setItem('recentSearches', JSON.stringify(updated));
  }, [recentSearches]);

  // Enhanced debounced search function with typing indicators
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

  // Generate smart suggestions
  const generateSuggestions = useCallback((searchQuery: string): void => {
    if (!searchQuery || searchQuery.length < 2) {
      setSuggestions([]);
      return;
    }
    
    const smartSuggestions: string[] = [];
    
    // Add similar popular searches
    const similar = popularSearches.filter(search => 
      search.toLowerCase().includes(searchQuery.toLowerCase()) ||
      searchQuery.toLowerCase().includes(search.toLowerCase())
    );
    smartSuggestions.push(...similar);
    
    // Add exact phrase suggestion
    if (!searchQuery.includes('"')) {
      smartSuggestions.push(`"${searchQuery}"`);
    }
    
    // Add tag search suggestion
    if (!searchQuery.startsWith('tag:')) {
      smartSuggestions.push(`tag:${searchQuery}`);
    }
    
    setSuggestions(smartSuggestions.slice(0, 3));
  }, [popularSearches]);

  const performSearch = useCallback(async (searchQuery: string): Promise<void> => {
    if (!searchQuery.trim()) {
      setResults([]);
      setSuggestions([]);
      return;
    }

    try {
      setLoading(true);
      setSearchProgress(0);
      
      // Progressive loading for better UX
      const progressInterval = setInterval(() => {
        setSearchProgress(prev => Math.min(prev + 25, 90));
      }, 50);
      
      const searchRequest: SearchRequest = {
        query: searchQuery.trim(),
        limit: 5, // Show only top 5 results in global search
        include_snippets: true, // Include snippets for context
        snippet_length: 100, // Shorter snippets for quick search
        search_mode: searchQuery.length < 4 ? 'fuzzy' : 'simple', // Use fuzzy for short queries (substring matching)
      };

      const response = await documentService.enhancedSearch(searchRequest);

      clearInterval(progressInterval);
      setSearchProgress(100);
      setResults(response.data.documents || []);
      
      // Clear progress after brief delay
      setTimeout(() => setSearchProgress(0), 300);
    } catch (error) {
      console.error('Global search failed:', error);
      setResults([]);
      setSearchProgress(0);
    } finally {
      setLoading(false);
    }
  }, []);

  const debouncedSearch = useCallback(
    debounce(performSearch, 200), // Even faster debounce for global search
    [performSearch]
  );
  
  const debouncedSuggestions = useCallback(
    debounce(generateSuggestions, 100), // Very fast suggestions
    [generateSuggestions]
  );

  const handleInputChange = (event: React.ChangeEvent<HTMLInputElement>): void => {
    const value = event.target.value;
    setQuery(value);
    setShowResults(true);
    
    if (value.trim()) {
      debouncedSearch(value);
      debouncedSuggestions(value);
    } else {
      setResults([]);
      setSuggestions([]);
    }
  };

  const handleInputFocus = (): void => {
    setShowResults(true);
  };

  const handleClickAway = (): void => {
    setShowResults(false);
  };

  const handleClear = (): void => {
    setQuery('');
    setResults([]);
    setSuggestions([]);
    setShowResults(false);
    setIsTyping(false);
    setSearchProgress(0);
  };

  const handleDocumentClick = (doc: EnhancedDocument): void => {
    saveRecentSearch(query);
    setShowResults(false);
    navigate(`/documents/${doc.id}`);
  };

  const handleRecentSearchClick = (searchQuery: string): void => {
    setQuery(searchQuery);
    performSearch(searchQuery);
  };
  
  const handleSuggestionClick = (suggestion: string): void => {
    setQuery(suggestion);
    performSearch(suggestion);
  };
  
  const handlePopularSearchClick = (search: string): void => {
    setQuery(search);
    performSearch(search);
    setShowResults(false);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLInputElement>): void => {
    if (event.key === 'Enter' && query.trim()) {
      saveRecentSearch(query);
      setShowResults(false);
      navigate(`/search?q=${encodeURIComponent(query)}`);
    }
    if (event.key === 'Escape') {
      setShowResults(false);
      searchInputRef.current?.blur();
    }
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

  // Function to highlight search terms in text (including substrings)
  const highlightText = useCallback((text: string, searchTerm: string): React.ReactNode => {
    if (!searchTerm || !text) return text;
    
    const terms = searchTerm.toLowerCase().split(/\s+/).filter(term => term.length >= 2);
    let highlightedText = text;
    
    terms.forEach(term => {
      const regex = new RegExp(`(${term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
      highlightedText = highlightedText.replace(regex, (match) => `**${match}**`);
    });
    
    // Split by ** markers and create spans
    const parts = highlightedText.split(/\*\*(.*?)\*\*/);
    
    return parts.map((part, index) => {
      if (index % 2 === 1) {
        // This is a highlighted part
        return (
          <Box
            key={index}
            component="mark"
            sx={{
              backgroundColor: theme.palette.mode === 'light' 
                ? 'rgba(102, 126, 234, 0.2)' 
                : 'rgba(155, 181, 255, 0.25)',
              color: theme.palette.mode === 'light'
                ? theme.palette.primary.dark
                : theme.palette.primary.light,
              padding: '0 2px',
              borderRadius: '2px',
              fontWeight: 600,
            }}
          >
            {part}
          </Box>
        );
      }
      return part;
    });
  }, [theme.palette.mode, theme.palette.primary]);

  // Enhanced search with context snippets
  const generateContextSnippet = useCallback((filename: string, searchTerm: string): string => {
    if (!searchTerm || !filename) return filename;
    
    const lowerFilename = filename.toLowerCase();
    const lowerTerm = searchTerm.toLowerCase();
    
    // Find the best match (exact term or substring)
    const exactMatch = lowerFilename.indexOf(lowerTerm);
    if (exactMatch !== -1) {
      // Show context around the match
      const start = Math.max(0, exactMatch - 10);
      const end = Math.min(filename.length, exactMatch + searchTerm.length + 10);
      const snippet = filename.substring(start, end);
      return start > 0 ? `...${snippet}` : snippet;
    }
    
    // Look for partial word matches
    const words = filename.split(/[_\-\s\.]/);
    const matchingWord = words.find(word => 
      word.toLowerCase().includes(lowerTerm) || lowerTerm.includes(word.toLowerCase())
    );
    
    if (matchingWord) {
      const wordIndex = words.indexOf(matchingWord);
      const contextWords = words.slice(Math.max(0, wordIndex - 1), Math.min(words.length, wordIndex + 2));
      return contextWords.join(' ');
    }
    
    return filename;
  }, []);

  return (
    <ClickAwayListener onClickAway={handleClickAway}>
      <Box sx={{ position: 'relative', ...sx }} {...props}>
        <Box sx={{ position: 'relative' }}>
          <TextField
            ref={searchInputRef}
            size="small"
            placeholder="Search documents..."
            value={query}
            onChange={handleInputChange}
            onFocus={handleInputFocus}
            onKeyDown={handleKeyDown}
            InputProps={{
              startAdornment: (
                <InputAdornment position="start">
                  <SearchIcon color="action" />
                </InputAdornment>
              ),
              endAdornment: (
                <InputAdornment position="end">
                  <Stack direction="row" spacing={0.5} alignItems="center">
                    {(loading || isTyping) && (
                      <CircularProgress 
                        size={16} 
                        variant={searchProgress > 0 ? "determinate" : "indeterminate"}
                        value={searchProgress}
                      />
                    )}
                    {query && (
                      <IconButton size="small" onClick={handleClear}>
                        <ClearIcon />
                      </IconButton>
                    )}
                  </Stack>
                </InputAdornment>
              ),
            }}
            sx={{
              minWidth: 320,
              maxWidth: 420,
              '& .MuiOutlinedInput-root': {
                background: theme.palette.mode === 'light'
                  ? 'linear-gradient(135deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.90) 100%)'
                  : 'linear-gradient(135deg, rgba(50,50,50,0.95) 0%, rgba(30,30,30,0.90) 100%)',
                backdropFilter: 'blur(20px)',
                border: theme.palette.mode === 'light'
                  ? '1px solid rgba(226,232,240,0.5)'
                  : '1px solid rgba(255,255,255,0.1)',
                borderRadius: 3,
                transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                boxShadow: theme.palette.mode === 'light'
                  ? '0 4px 16px rgba(0,0,0,0.04)'
                  : '0 4px 16px rgba(0,0,0,0.2)',
                '&:hover': {
                  background: theme.palette.mode === 'light'
                    ? 'linear-gradient(135deg, rgba(255,255,255,0.98) 0%, rgba(248,250,252,0.95) 100%)'
                    : 'linear-gradient(135deg, rgba(60,60,60,0.98) 0%, rgba(40,40,40,0.95) 100%)',
                  borderColor: 'rgba(99,102,241,0.4)',
                  transform: 'translateY(-2px)',
                  boxShadow: '0 8px 32px rgba(99,102,241,0.15)',
                },
                '&.Mui-focused': {
                  background: theme.palette.mode === 'light'
                    ? 'linear-gradient(135deg, rgba(255,255,255,1) 0%, rgba(248,250,252,0.98) 100%)'
                    : 'linear-gradient(135deg, rgba(70,70,70,1) 0%, rgba(50,50,50,0.98) 100%)',
                  borderColor: '#6366f1',
                  borderWidth: 2,
                  transform: 'translateY(-2px)',
                  boxShadow: '0 12px 40px rgba(99,102,241,0.2)',
                },
                '& .MuiInputBase-input': {
                  fontWeight: 500,
                  letterSpacing: '0.025em',
                  fontSize: '0.95rem',
                  color: theme.palette.text.primary,
                  '&::placeholder': {
                    color: theme.palette.mode === 'light'
                      ? 'rgba(148,163,184,0.8)'
                      : 'rgba(200,200,200,0.6)',
                    fontWeight: 400,
                  },
                },
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
                height: 2,
                borderRadius: '0 0 4px 4px',
                opacity: isTyping ? 0.6 : 1,
                transition: 'opacity 0.2s ease-in-out',
              }}
            />
          )}
        </Box>

        {/* Search Results Dropdown */}
        <Popper
          open={showResults}
          anchorEl={searchInputRef.current}
          placement="bottom-start"
          style={{ zIndex: 1300, width: searchInputRef.current?.offsetWidth }}
          transition
        >
          {({ TransitionProps }) => (
            <Grow {...TransitionProps}>
              <Paper
                elevation={0}
                sx={{
                  mt: 1,
                  maxHeight: 420,
                  overflowY: 'auto',
                  overflowX: 'hidden',
                  background: theme.palette.mode === 'light'
                    ? 'linear-gradient(180deg, rgba(255,255,255,0.98) 0%, rgba(248,250,252,0.95) 100%)'
                    : 'linear-gradient(180deg, rgba(40,40,40,0.98) 0%, rgba(25,25,25,0.95) 100%)',
                  backdropFilter: 'blur(24px)',
                  border: theme.palette.mode === 'light'
                    ? '1px solid rgba(226,232,240,0.6)'
                    : '1px solid rgba(255,255,255,0.1)',
                  borderRadius: 3,
                  boxShadow: theme.palette.mode === 'light'
                    ? '0 20px 60px rgba(0,0,0,0.12), 0 8px 25px rgba(0,0,0,0.08)'
                    : '0 20px 60px rgba(0,0,0,0.4), 0 8px 25px rgba(0,0,0,0.3)',
                  width: '100%',
                  minWidth: 0,
                }}
              >
                {(loading || isTyping) && (
                  <Box sx={{ 
                    p: 3, 
                    textAlign: 'center',
                    background: 'linear-gradient(135deg, rgba(99,102,241,0.02) 0%, rgba(139,92,246,0.02) 100%)',
                  }}>
                    <Stack spacing={1.5} alignItems="center">
                      <Box sx={{
                        p: 1.5,
                        borderRadius: 2,
                        background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                      }}>
                        <CircularProgress size={20} thickness={4} sx={{ color: '#6366f1' }} />
                      </Box>
                      <Typography variant="body2" sx={{ 
                        color: 'text.secondary',
                        fontWeight: 500,
                        letterSpacing: '0.025em',
                      }}>
                        {isTyping ? 'Searching as you type...' : 'Searching...'}
                      </Typography>
                    </Stack>
                  </Box>
                )}
                
                {/* Loading Skeletons for better UX */}
                {loading && query && (
                  <List sx={{ py: 0 }}>
                    {[1, 2, 3].map((i) => (
                      <ListItem key={i} sx={{ py: 1 }}>
                        <ListItemIcon sx={{ minWidth: 40 }}>
                          <Skeleton variant="circular" width={24} height={24} />
                        </ListItemIcon>
                        <ListItemText
                          primary={<Skeleton variant="text" width="80%" />}
                          secondary={<Skeleton variant="text" width="60%" />}
                        />
                      </ListItem>
                    ))}
                  </List>
                )}

                {!loading && !isTyping && query && results.length === 0 && (
                  <Box sx={{ 
                    p: 3, 
                    textAlign: 'center',
                    background: 'linear-gradient(135deg, rgba(99,102,241,0.02) 0%, rgba(139,92,246,0.02) 100%)',
                  }}>
                    <Typography variant="body2" sx={{
                      color: 'text.secondary',
                      fontWeight: 500,
                      letterSpacing: '0.025em',
                      mb: 1,
                    }}>
                      No documents found for "{query}"
                    </Typography>
                    <Typography variant="caption" sx={{
                      color: 'text.secondary',
                      fontWeight: 500,
                      mb: 2,
                      display: 'block',
                    }}>
                      Press Enter to search with advanced options
                    </Typography>
                    
                    {/* Smart suggestions for no results */}
                    {suggestions.length > 0 && (
                      <>
                        <Typography variant="caption" sx={{
                          color: 'text.primary',
                          fontWeight: 600,
                          letterSpacing: '0.05em',
                          textTransform: 'uppercase',
                          fontSize: '0.7rem',
                          mb: 1.5,
                          display: 'block',
                        }}>
                          Try these suggestions:
                        </Typography>
                        <Stack direction="row" spacing={0.5} justifyContent="center" flexWrap="wrap">
                          {suggestions.map((suggestion, index) => (
                            <Chip
                              key={index}
                              label={suggestion}
                              size="small"
                              variant="outlined"
                              clickable
                              onClick={() => handleSuggestionClick(suggestion)}
                              sx={{ 
                                fontSize: '0.7rem', 
                                height: 24,
                                fontWeight: 500,
                                border: '1px solid rgba(99,102,241,0.3)',
                                background: 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)',
                                backdropFilter: 'blur(10px)',
                                transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                                '&:hover': {
                                  background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                                  color: 'white',
                                  transform: 'translateY(-2px)',
                                  boxShadow: '0 8px 24px rgba(99,102,241,0.2)',
                                },
                              }}
                            />
                          ))}
                        </Stack>
                      </>
                    )}
                  </Box>
                )}

                {!loading && !isTyping && results.length > 0 && (
                  <>
                    <Box sx={{ 
                      p: 2, 
                      borderBottom: '1px solid rgba(226,232,240,0.4)',
                      background: 'linear-gradient(135deg, rgba(99,102,241,0.03) 0%, rgba(139,92,246,0.03) 100%)',
                    }}>
                      <Stack direction="row" justifyContent="space-between" alignItems="center">
                        <Typography variant="caption" sx={{
                          color: 'text.secondary',
                          fontWeight: 600,
                          letterSpacing: '0.05em',
                          textTransform: 'uppercase',
                          fontSize: '0.7rem',
                        }}>
                          Quick Results
                        </Typography>
                        <Box sx={{
                          px: 1.5,
                          py: 0.5,
                          borderRadius: 2,
                          background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                          color: 'white',
                        }}>
                          <Typography variant="caption" sx={{
                            fontWeight: 600,
                            fontSize: '0.7rem',
                          }}>
                            {results.length} found
                          </Typography>
                        </Box>
                      </Stack>
                    </Box>
                    <List sx={{ py: 0 }}>
                      {results.map((doc) => (
                        <ListItem
                          key={doc.id}
                          component="div"
                          onClick={() => handleDocumentClick(doc)}
                          sx={{
                            py: 1.5,
                            cursor: 'pointer',
                            borderRadius: 2,
                            mx: 1,
                            minWidth: 0,
                            overflow: 'hidden',
                            transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                            '&:hover': {
                              background: 'linear-gradient(135deg, rgba(99,102,241,0.08) 0%, rgba(139,92,246,0.08) 100%)',
                              transform: 'translateX(4px)',
                              boxShadow: '0 4px 16px rgba(99,102,241,0.1)',
                            },
                          }}
                        >
                          <ListItemIcon sx={{ minWidth: 40 }}>
                            {getFileIcon(doc.mime_type)}
                          </ListItemIcon>
                          <ListItemText
                            primary={
                              <Typography
                                variant="body2"
                                sx={{
                                  overflow: 'hidden',
                                  textOverflow: 'ellipsis',
                                  whiteSpace: 'nowrap',
                                  maxWidth: '100%',
                                  flex: 1,
                                }}
                              >
                                {highlightText(doc.original_filename || doc.filename, query)}
                              </Typography>
                            }
                            secondary={
                              <Box>
                                <Stack direction="row" spacing={1} alignItems="center" sx={{ mb: 0.5 }}>
                                  <Typography variant="caption" color="text.secondary">
                                    {formatFileSize(doc.file_size)}
                                  </Typography>
                                  {doc.has_ocr_text && (
                                    <Chip
                                      label="OCR"
                                      size="small"
                                      color="success"
                                      variant="outlined"
                                      sx={{ 
                                        height: 16, 
                                        fontSize: '0.6rem',
                                        '& .MuiChip-label': {
                                          color: theme.palette.mode === 'light' 
                                            ? 'success.dark' 
                                            : 'rgba(102, 187, 106, 0.8)',
                                        },
                                      }}
                                    />
                                  )}
                                  {doc.search_rank && (
                                    <Chip
                                      icon={<TrendingIcon sx={{ fontSize: 10 }} />}
                                      label={`${(doc.search_rank * 100).toFixed(0)}%`}
                                      size="small"
                                      color="info"
                                      variant="outlined"
                                      sx={{ 
                                        height: 16, 
                                        fontSize: '0.6rem',
                                        '& .MuiChip-label': {
                                          color: theme.palette.mode === 'light' 
                                            ? 'info.dark' 
                                            : 'rgba(100, 181, 246, 0.8)',
                                        },
                                      }}
                                    />
                                  )}
                                </Stack>
                                
                                {/* Show content snippet if available */}
                                {doc.snippets && doc.snippets.length > 0 && (
                                  <Typography 
                                    variant="caption" 
                                    color="text.secondary"
                                    sx={{
                                      display: 'block',
                                      overflow: 'hidden',
                                      textOverflow: 'ellipsis',
                                      whiteSpace: 'nowrap',
                                      fontSize: '0.7rem',
                                      fontStyle: 'italic',
                                      maxWidth: '100%',
                                      flex: 1,
                                    }}
                                  >
                                    {highlightText(doc.snippets[0]?.text?.substring(0, 80) + '...' || '', query)}
                                  </Typography>
                                )}
                              </Box>
                            }
                          />
                        </ListItem>
                      ))}
                    </List>
                    
                    {results.length >= 5 && (
                      <Box sx={{ 
                        p: 2, 
                        textAlign: 'center', 
                        borderTop: '1px solid rgba(226,232,240,0.4)',
                        background: 'linear-gradient(135deg, rgba(99,102,241,0.03) 0%, rgba(139,92,246,0.03) 100%)',
                      }}>
                        <Box
                          sx={{
                            display: 'inline-flex',
                            alignItems: 'center',
                            px: 3,
                            py: 1.5,
                            borderRadius: 2,
                            background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
                            cursor: 'pointer',
                            transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                            '&:hover': {
                              background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                              transform: 'translateY(-2px)',
                              boxShadow: '0 8px 24px rgba(99,102,241,0.2)',
                              '& .view-all-text': {
                                color: 'white',
                              },
                            },
                          }}
                          onClick={() => {
                            saveRecentSearch(query);
                            setShowResults(false);
                            navigate(`/search?q=${encodeURIComponent(query)}`);
                          }}
                        >
                          <Typography
                            className="view-all-text"
                            variant="caption"
                            sx={{
                              color: '#6366f1',
                              fontWeight: 600,
                              letterSpacing: '0.025em',
                              fontSize: '0.8rem',
                              transition: 'color 0.2s ease-in-out',
                            }}
                          >
                            View all results for "{query}"
                          </Typography>
                        </Box>
                      </Box>
                    )}
                  </>
                )}

                {!query && recentSearches.length > 0 && (
                  <>
                    <Box sx={{ 
                      p: 2, 
                      borderBottom: '1px solid rgba(226,232,240,0.4)',
                      background: 'linear-gradient(135deg, rgba(99,102,241,0.03) 0%, rgba(139,92,246,0.03) 100%)',
                    }}>
                      <Typography variant="caption" sx={{
                        color: 'text.secondary',
                        fontWeight: 600,
                        letterSpacing: '0.05em',
                        textTransform: 'uppercase',
                        fontSize: '0.7rem',
                      }}>
                        Recent Searches
                      </Typography>
                    </Box>
                    <List sx={{ py: 0 }}>
                      {recentSearches.map((search, index) => (
                        <ListItem
                          key={index}
                          component="div"
                          onClick={() => handleRecentSearchClick(search)}
                          sx={{
                            py: 1.5,
                            cursor: 'pointer',
                            borderRadius: 2,
                            mx: 1,
                            minWidth: 0,
                            overflow: 'hidden',
                            transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                            '&:hover': {
                              background: 'linear-gradient(135deg, rgba(99,102,241,0.08) 0%, rgba(139,92,246,0.08) 100%)',
                              transform: 'translateX(4px)',
                              boxShadow: '0 4px 16px rgba(99,102,241,0.1)',
                            },
                          }}
                        >
                          <ListItemIcon sx={{ minWidth: 40 }}>
                            <TimeIcon color="action" />
                          </ListItemIcon>
                          <ListItemText
                            primary={
                              <Typography 
                                variant="body2"
                                sx={{
                                  overflow: 'hidden',
                                  textOverflow: 'ellipsis',
                                  whiteSpace: 'nowrap',
                                  maxWidth: '100%',
                                  flex: 1,
                                }}
                              >
                                {search}
                              </Typography>
                            }
                          />
                        </ListItem>
                      ))}
                    </List>
                  </>
                )}

                {!query && recentSearches.length === 0 && (
                  <Box sx={{ 
                    p: 3, 
                    textAlign: 'center',
                    background: 'linear-gradient(135deg, rgba(99,102,241,0.02) 0%, rgba(139,92,246,0.02) 100%)',
                  }}>
                    <Typography variant="body2" sx={{
                      color: 'text.secondary',
                      fontWeight: 500,
                      letterSpacing: '0.025em',
                      mb: 1,
                    }}>
                      Start typing to search documents
                    </Typography>
                    <Typography variant="caption" sx={{
                      color: 'text.secondary',
                      fontWeight: 600,
                      letterSpacing: '0.05em',
                      textTransform: 'uppercase',
                      fontSize: '0.7rem',
                      mb: 2,
                      display: 'block',
                    }}>
                      Popular searches:
                    </Typography>
                    <Stack direction="row" spacing={1} justifyContent="center" flexWrap="wrap">
                      {popularSearches.slice(0, 3).map((search, index) => (
                        <Chip 
                          key={index}
                          label={search} 
                          size="small" 
                          variant="outlined" 
                          clickable
                          onClick={() => handlePopularSearchClick(search)}
                          sx={{ 
                            fontSize: '0.75rem',
                            fontWeight: 500,
                            border: '1px solid rgba(99,102,241,0.3)',
                            background: 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)',
                            backdropFilter: 'blur(10px)',
                            transition: 'all 0.2s cubic-bezier(0.4, 0, 0.2, 1)',
                            '&:hover': {
                              background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                              color: 'white',
                              transform: 'translateY(-2px)',
                              boxShadow: '0 8px 24px rgba(99,102,241,0.2)',
                            }
                          }}
                        />
                      ))}
                    </Stack>
                  </Box>
                )}
              </Paper>
            </Grow>
          )}
        </Popper>
      </Box>
    </ClickAwayListener>
  );
};

export default GlobalSearchBar;