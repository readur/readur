import React, { useState } from 'react';
import {
  Box,
  Typography,
  Paper,
  IconButton,
  Collapse,
  Chip,
  Button,
  Menu,
  MenuItem,
  ListItemIcon,
  ListItemText,
  Tooltip,
  RadioGroup,
  FormControlLabel,
  Radio,
  Slider,
  Divider,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  ExpandLess as ExpandLessIcon,
  FormatSize as FontSizeIcon,
  ViewModule as ViewModeIcon,
  ContentCopy as CopyIcon,
  FormatQuote as QuoteIcon,
  Code as CodeIcon,
  WrapText as WrapIcon,
  Search as SearchIcon,
  Settings as SettingsIcon,
} from '@mui/icons-material';

interface HighlightRange {
  start: number;
  end: number;
}

interface Snippet {
  text: string;
  highlight_ranges?: HighlightRange[];
  source?: 'content' | 'ocr_text' | 'filename';
  page_number?: number;
  confidence?: number;
}

interface EnhancedSnippetViewerProps {
  snippets: Snippet[];
  searchQuery?: string;
  maxSnippetsToShow?: number;
  onSnippetClick?: (snippet: Snippet, index: number) => void;
}

type ViewMode = 'compact' | 'detailed' | 'context';
type HighlightStyle = 'background' | 'underline' | 'bold';

const EnhancedSnippetViewer: React.FC<EnhancedSnippetViewerProps> = ({
  snippets,
  searchQuery,
  maxSnippetsToShow = 3,
  onSnippetClick,
}) => {
  const [expanded, setExpanded] = useState(false);
  const [viewMode, setViewMode] = useState<ViewMode>('detailed');
  const [highlightStyle, setHighlightStyle] = useState<HighlightStyle>('background');
  const [fontSize, setFontSize] = useState<number>(14);
  const [contextLength, setContextLength] = useState<number>(50);
  const [settingsAnchor, setSettingsAnchor] = useState<null | HTMLElement>(null);
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);

  const visibleSnippets = expanded ? snippets : snippets.slice(0, maxSnippetsToShow);

  const handleCopySnippet = (text: string, index: number) => {
    navigator.clipboard.writeText(text);
    setCopiedIndex(index);
    setTimeout(() => setCopiedIndex(null), 2000);
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
      const highlightedText = text.substring(range.start, range.end);
      parts.push(
        <Box
          key={`highlight-${index}`}
          component="mark"
          sx={{
            backgroundColor: highlightStyle === 'background' ? 'primary.light' : 'transparent',
            color: highlightStyle === 'background' ? 'primary.contrastText' : 'primary.main',
            textDecoration: highlightStyle === 'underline' ? 'underline' : 'none',
            textDecorationColor: 'primary.main',
            textDecorationThickness: '2px',
            fontWeight: highlightStyle === 'bold' ? 700 : 600,
            padding: highlightStyle === 'background' ? '0 2px' : 0,
            borderRadius: highlightStyle === 'background' ? '2px' : 0,
          }}
        >
          {highlightedText}
        </Box>
      );
      
      lastIndex = range.end;
    });

    // Add remaining text
    if (lastIndex < text.length) {
      parts.push(
        <span key={`text-end`}>
          {text.substring(lastIndex)}
        </span>
      );
    }

    return parts;
  };

  const getSourceIcon = (source?: string) => {
    switch (source) {
      case 'ocr_text':
        return <SearchIcon fontSize="small" />;
      case 'filename':
        return <QuoteIcon fontSize="small" />;
      default:
        return <CodeIcon fontSize="small" />;
    }
  };

  const getSourceLabel = (source?: string) => {
    switch (source) {
      case 'ocr_text':
        return 'OCR Text';
      case 'filename':
        return 'Filename';
      default:
        return 'Document Content';
    }
  };

  const renderSnippet = (snippet: Snippet, index: number) => {
    const isCompact = viewMode === 'compact';
    const showContext = viewMode === 'context';

    // Extract context around highlights if in context mode
    let displayText = snippet.text;
    if (showContext && snippet.highlight_ranges && snippet.highlight_ranges.length > 0) {
      const firstHighlight = snippet.highlight_ranges[0];
      const lastHighlight = snippet.highlight_ranges[snippet.highlight_ranges.length - 1];
      
      const contextStart = Math.max(0, firstHighlight.start - contextLength);
      const contextEnd = Math.min(snippet.text.length, lastHighlight.end + contextLength);
      
      displayText = (contextStart > 0 ? '...' : '') + 
                   snippet.text.substring(contextStart, contextEnd) + 
                   (contextEnd < snippet.text.length ? '...' : '');
      
      // Adjust highlight ranges for the new substring
      if (snippet.highlight_ranges) {
        snippet = {
          ...snippet,
          text: displayText,
          highlight_ranges: snippet.highlight_ranges.map(range => ({
            start: range.start - contextStart + (contextStart > 0 ? 3 : 0),
            end: range.end - contextStart + (contextStart > 0 ? 3 : 0),
          })),
        };
      }
    }

    return (
      <Paper
        key={index}
        variant="outlined"
        sx={{
          p: isCompact ? 1 : 2,
          mb: 1.5,
          backgroundColor: (theme) => theme.palette.mode === 'light' ? 'grey.50' : 'grey.900',
          borderLeft: '3px solid',
          borderLeftColor: snippet.source === 'ocr_text' ? 'warning.main' : 'primary.main',
          cursor: onSnippetClick ? 'pointer' : 'default',
          transition: 'all 0.2s',
          '&:hover': onSnippetClick ? {
            backgroundColor: (theme) => theme.palette.mode === 'light' ? 'grey.100' : 'grey.800',
            transform: 'translateX(4px)',
          } : {},
        }}
        onClick={() => onSnippetClick?.(snippet, index)}
      >
        <Box display="flex" alignItems="flex-start" justifyContent="space-between">
          <Box flex={1}>
            {!isCompact && (
              <Box display="flex" alignItems="center" gap={1} mb={1}>
                <Chip
                  icon={getSourceIcon(snippet.source)}
                  label={getSourceLabel(snippet.source)}
                  size="small"
                  variant="outlined"
                />
                {snippet.page_number && (
                  <Chip
                    label={`Page ${snippet.page_number}`}
                    size="small"
                    variant="outlined"
                  />
                )}
                {snippet.confidence && snippet.confidence < 0.8 && (
                  <Chip
                    label={`${(snippet.confidence * 100).toFixed(0)}% confidence`}
                    size="small"
                    color="warning"
                    variant="outlined"
                  />
                )}
              </Box>
            )}
            
            <Typography
              variant="body2"
              sx={{
                fontSize: `${fontSize}px`,
                lineHeight: 1.6,
                color: 'text.primary',
                wordWrap: 'break-word',
                fontFamily: viewMode === 'context' ? 'monospace' : 'inherit',
              }}
            >
              {renderHighlightedText(snippet.text, snippet.highlight_ranges)}
            </Typography>
          </Box>
          
          <Box display="flex" gap={0.5} ml={2}>
            <Tooltip title="Copy snippet">
              <IconButton
                size="small"
                onClick={(e) => {
                  e.stopPropagation();
                  handleCopySnippet(snippet.text, index);
                }}
                sx={{ 
                  color: copiedIndex === index ? 'success.main' : 'text.secondary' 
                }}
              >
                <CopyIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>
      </Paper>
    );
  };

  return (
    <Box>
      <Box display="flex" alignItems="center" justifyContent="space-between" mb={1}>
        <Box display="flex" alignItems="center" gap={1}>
          <Typography variant="subtitle2" fontWeight="bold">
            Search Results
          </Typography>
          {snippets.length > 0 && (
            <Chip 
              label={`${snippets.length} matches`} 
              size="small" 
              color="primary"
              variant="outlined"
            />
          )}
        </Box>
        
        <Box display="flex" alignItems="center" gap={1}>
          <Tooltip title="Snippet settings">
            <IconButton
              size="small"
              onClick={(e) => setSettingsAnchor(e.currentTarget)}
            >
              <SettingsIcon fontSize="small" />
            </IconButton>
          </Tooltip>
          
          {snippets.length > maxSnippetsToShow && (
            <Button
              size="small"
              onClick={() => setExpanded(!expanded)}
              endIcon={expanded ? <ExpandLessIcon /> : <ExpandMoreIcon />}
            >
              {expanded ? 'Show Less' : `Show All (${snippets.length})`}
            </Button>
          )}
        </Box>
      </Box>

      {searchQuery && (
        <Box mb={2}>
          <Typography variant="caption" color="text.secondary">
            Showing matches for: <strong>{searchQuery}</strong>
          </Typography>
        </Box>
      )}

      {visibleSnippets.map((snippet, index) => renderSnippet(snippet, index))}

      {snippets.length === 0 && (
        <Paper variant="outlined" sx={{ p: 3, textAlign: 'center' }}>
          <Typography variant="body2" color="text.secondary">
            No text snippets available for this search result
          </Typography>
        </Paper>
      )}

      {/* Settings Menu */}
      <Menu
        anchorEl={settingsAnchor}
        open={Boolean(settingsAnchor)}
        onClose={() => setSettingsAnchor(null)}
        PaperProps={{ sx: { width: 320, p: 2 } }}
      >
        <Typography variant="subtitle2" sx={{ mb: 2 }}>
          Snippet Display Settings
        </Typography>
        
        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            View Mode
          </Typography>
          <RadioGroup
            value={viewMode}
            onChange={(e) => setViewMode(e.target.value as ViewMode)}
          >
            <FormControlLabel
              value="compact"
              control={<Radio size="small" />}
              label="Compact"
            />
            <FormControlLabel
              value="detailed"
              control={<Radio size="small" />}
              label="Detailed (with metadata)"
            />
            <FormControlLabel
              value="context"
              control={<Radio size="small" />}
              label="Context Focus"
            />
          </RadioGroup>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            Highlight Style
          </Typography>
          <RadioGroup
            value={highlightStyle}
            onChange={(e) => setHighlightStyle(e.target.value as HighlightStyle)}
          >
            <FormControlLabel
              value="background"
              control={<Radio size="small" />}
              label="Background Color"
            />
            <FormControlLabel
              value="underline"
              control={<Radio size="small" />}
              label="Underline"
            />
            <FormControlLabel
              value="bold"
              control={<Radio size="small" />}
              label="Bold Text"
            />
          </RadioGroup>
        </Box>

        <Divider sx={{ my: 2 }} />

        <Box mb={2}>
          <Typography variant="caption" color="text.secondary" gutterBottom>
            Font Size: {fontSize}px
          </Typography>
          <Slider
            value={fontSize}
            onChange={(_, value) => setFontSize(value as number)}
            min={12}
            max={20}
            marks
            valueLabelDisplay="auto"
          />
        </Box>

        {viewMode === 'context' && (
          <>
            <Divider sx={{ my: 2 }} />
            <Box>
              <Typography variant="caption" color="text.secondary" gutterBottom>
                Context Length: {contextLength} characters
              </Typography>
              <Slider
                value={contextLength}
                onChange={(_, value) => setContextLength(value as number)}
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

export default EnhancedSnippetViewer;