import React, { useState } from 'react';
import {
  Box,
  Typography,
  Chip,
  Stack,
  Accordion,
  AccordionSummary,
  AccordionDetails,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Paper,
  IconButton,
  Collapse,
  SxProps,
  Theme,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  Help as HelpIcon,
  Search as SearchIcon,
  FormatQuote as QuoteIcon,
  Tag as TagIcon,
  Extension as ExtensionIcon,
  Close as CloseIcon,
  TrendingUp as TrendingIcon,
} from '../../design/icons';

interface SearchExample {
  query: string;
  description: string;
  icon: React.ReactElement;
}

interface SearchGuidanceProps {
  onExampleClick?: (query: string) => void;
  compact?: boolean;
  sx?: SxProps<Theme>;
  [key: string]: any;
}

const SearchGuidance: React.FC<SearchGuidanceProps> = ({ 
  onExampleClick, 
  compact = false, 
  sx, 
  ...props 
}) => {
  const [showHelp, setShowHelp] = useState<boolean>(false);

  const searchExamples: SearchExample[] = [
    {
      query: 'invoice 2024',
      description: 'Find documents containing both "invoice" and "2024"',
      icon: <SearchIcon />,
    },
    {
      query: '"project proposal"',
      description: 'Search for exact phrase "project proposal"',
      icon: <QuoteIcon />,
    },
    {
      query: 'tag:important',
      description: 'Find all documents tagged as "important"',
      icon: <TagIcon />,
    },
    {
      query: 'contract AND payment',
      description: 'Advanced search using AND operator',
      icon: <ExtensionIcon />,
    },
    {
      query: 'proj*',
      description: 'Wildcard search for project, projects, etc.',
      icon: <TrendingIcon />,
    },
  ];

  const searchTips: string[] = [
    'Use quotes for exact phrases: "annual report"',
    'Search by tags: tag:urgent or tag:personal',
    'Use AND/OR for complex queries: (invoice OR receipt) AND 2024',
    'Wildcards work great: proj* finds project, projects, projection',
    'Search OCR text in images and PDFs automatically',
    'File types are searchable: PDF, Word, Excel, images',
  ];

  const handleExampleClick = (query: string): void => {
    if (onExampleClick) {
      onExampleClick(query);
    }
  };

  if (compact) {
    return (
      <Box sx={{ position: 'relative', ...sx }} {...props}>
        <IconButton
          size="small"
          onClick={() => setShowHelp(!showHelp)}
          color={showHelp ? 'primary' : 'default'}
          sx={{ 
            position: 'absolute', 
            top: 0, 
            right: 0, 
            zIndex: 1,
            backgroundColor: 'background.paper',
            '&:hover': {
              backgroundColor: 'action.hover',
            }
          }}
        >
          {showHelp ? <CloseIcon /> : <HelpIcon />}
        </IconButton>
        
        <Collapse in={showHelp}>
          <Paper 
            elevation={3} 
            sx={{ 
              p: 2, 
              mt: 1, 
              border: '1px solid',
              borderColor: 'primary.light',
              backgroundColor: 'background.paper',
            }}
          >
            <Typography variant="subtitle2" gutterBottom sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
              <HelpIcon color="primary" />
              Quick Search Tips
            </Typography>
            
            <Stack spacing={1}>
              {searchTips.slice(0, 3).map((tip, index) => (
                <Typography key={index} variant="body2" color="text.secondary" sx={{ fontSize: '0.8rem' }}>
                  • {tip}
                </Typography>
              ))}
            </Stack>
            
            <Typography variant="caption" color="text.secondary" sx={{ mt: 1, display: 'block' }}>
              Try these examples:
            </Typography>
            <Stack direction="row" spacing={0.5} flexWrap="wrap" sx={{ mt: 0.5 }}>
              {searchExamples.slice(0, 3).map((example, index) => (
                <Chip
                  key={index}
                  label={example.query}
                  size="small"
                  variant="outlined"
                  clickable
                  onClick={() => handleExampleClick(example.query)}
                  sx={{ 
                    fontSize: '0.7rem',
                    height: 20,
                    '&:hover': {
                      backgroundColor: 'primary.light',
                      color: 'primary.contrastText',
                    }
                  }}
                />
              ))}
            </Stack>
          </Paper>
        </Collapse>
      </Box>
    );
  }

  return (
    <Box sx={sx} {...props}>
      <Accordion>
        <AccordionSummary expandIcon={<ExpandMoreIcon />}>
          <Typography variant="h6" sx={{ display: 'flex', alignItems: 'center', gap: 1 }}>
            <HelpIcon color="primary" />
            Search Help & Examples
          </Typography>
        </AccordionSummary>
        <AccordionDetails>
          <Stack spacing={3}>
            {/* Search Examples */}
            <Box>
              <Typography variant="subtitle2" gutterBottom>
                Example Searches
              </Typography>
              <List dense>
                {searchExamples.map((example, index) => (
                  <ListItem
                    key={index}
                    component="div"
                    onClick={() => handleExampleClick(example.query)}
                    sx={{
                      borderRadius: 1,
                      mb: 0.5,
                      cursor: 'pointer',
                      '&:hover': {
                        backgroundColor: 'action.hover',
                      },
                    }}
                  >
                    <ListItemIcon sx={{ minWidth: 32 }}>
                      {example.icon}
                    </ListItemIcon>
                    <ListItemText
                      primary={
                        <Typography variant="body2" sx={{ fontFamily: 'monospace', fontWeight: 600 }}>
                          {example.query}
                        </Typography>
                      }
                      secondary={
                        <Typography variant="caption" color="text.secondary">
                          {example.description}
                        </Typography>
                      }
                    />
                  </ListItem>
                ))}
              </List>
            </Box>

            {/* Search Tips */}
            <Box>
              <Typography variant="subtitle2" gutterBottom>
                Search Tips
              </Typography>
              <Stack spacing={1}>
                {searchTips.map((tip, index) => (
                  <Typography key={index} variant="body2" color="text.secondary">
                    • {tip}
                  </Typography>
                ))}
              </Stack>
            </Box>

            {/* Quick Actions */}
            <Box>
              <Typography variant="subtitle2" gutterBottom>
                Quick Start
              </Typography>
              <Stack direction="row" spacing={1} flexWrap="wrap">
                {searchExamples.map((example, index) => (
                  <Chip
                    key={index}
                    label={example.query}
                    size="small"
                    variant="outlined"
                    clickable
                    onClick={() => handleExampleClick(example.query)}
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
          </Stack>
        </AccordionDetails>
      </Accordion>
    </Box>
  );
};

export default SearchGuidance;