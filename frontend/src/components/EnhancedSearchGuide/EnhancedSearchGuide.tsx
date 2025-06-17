import React, { useState } from 'react';
import {
  Box,
  Typography,
  Card,
  CardContent,
  Chip,
  IconButton,
  Collapse,
  Grid,
  Button,
  Tabs,
  Tab,
  Paper,
  Tooltip,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
  Divider,
  Alert,
} from '@mui/material';
import {
  ExpandMore as ExpandMoreIcon,
  ContentCopy as CopyIcon,
  Search as SearchIcon,
  Label as LabelIcon,
  TextFormat as TextIcon,
  Functions as FunctionIcon,
  DateRange as DateIcon,
  Storage as SizeIcon,
  Code as CodeIcon,
  Lightbulb as TipIcon,
  PlayArrow as PlayIcon,
} from '@mui/icons-material';

interface SearchExample {
  query: string;
  description: string;
  category: 'basic' | 'advanced' | 'filters' | 'operators';
  icon?: React.ReactElement;
}

interface EnhancedSearchGuideProps {
  onExampleClick?: (query: string) => void;
  compact?: boolean;
}

const EnhancedSearchGuide: React.FC<EnhancedSearchGuideProps> = ({ onExampleClick, compact = false }) => {
  const [expanded, setExpanded] = useState(!compact);
  const [activeTab, setActiveTab] = useState(0);
  const [copiedExample, setCopiedExample] = useState<string | null>(null);

  const searchExamples: SearchExample[] = [
    // Basic searches
    {
      query: 'invoice',
      description: 'Simple keyword search',
      category: 'basic',
      icon: <SearchIcon />,
    },
    {
      query: '"project proposal"',
      description: 'Exact phrase search',
      category: 'basic',
      icon: <TextIcon />,
    },
    {
      query: 'report*',
      description: 'Wildcard search (finds report, reports, reporting)',
      category: 'basic',
      icon: <FunctionIcon />,
    },
    
    // Advanced searches
    {
      query: 'invoice AND payment',
      description: 'Both terms must appear',
      category: 'advanced',
      icon: <CodeIcon />,
    },
    {
      query: 'budget OR forecast',
      description: 'Either term can appear',
      category: 'advanced',
      icon: <CodeIcon />,
    },
    {
      query: 'contract NOT draft',
      description: 'Exclude documents with "draft"',
      category: 'advanced',
      icon: <CodeIcon />,
    },
    {
      query: '(invoice OR receipt) AND 2024',
      description: 'Complex boolean search with grouping',
      category: 'advanced',
      icon: <CodeIcon />,
    },
    
    // Filter searches
    {
      query: 'tag:important',
      description: 'Search by tag',
      category: 'filters',
      icon: <LabelIcon />,
    },
    {
      query: 'tag:invoice tag:paid',
      description: 'Multiple tags (must have both)',
      category: 'filters',
      icon: <LabelIcon />,
    },
    {
      query: 'type:pdf invoice',
      description: 'Search only PDF files',
      category: 'filters',
      icon: <TextIcon />,
    },
    {
      query: 'size:>5MB presentation',
      description: 'Files larger than 5MB',
      category: 'filters',
      icon: <SizeIcon />,
    },
    {
      query: 'date:2024 quarterly report',
      description: 'Documents from 2024',
      category: 'filters',
      icon: <DateIcon />,
    },
    {
      query: 'ocr:yes scan',
      description: 'Only documents with OCR text',
      category: 'filters',
      icon: <TextIcon />,
    },
    
    // Power user operators
    {
      query: 'invoice NEAR payment',
      description: 'Terms appear close together',
      category: 'operators',
      icon: <FunctionIcon />,
    },
    {
      query: '"annual report" ~5 2024',
      description: 'Terms within 5 words of each other',
      category: 'operators',
      icon: <FunctionIcon />,
    },
    {
      query: 'proj* AND (budget OR cost*) tag:active',
      description: 'Complex query combining wildcards, boolean, and filters',
      category: 'operators',
      icon: <FunctionIcon />,
    },
  ];

  const categorizedExamples = {
    basic: searchExamples.filter(e => e.category === 'basic'),
    advanced: searchExamples.filter(e => e.category === 'advanced'),
    filters: searchExamples.filter(e => e.category === 'filters'),
    operators: searchExamples.filter(e => e.category === 'operators'),
  };

  const handleCopyExample = (query: string) => {
    navigator.clipboard.writeText(query);
    setCopiedExample(query);
    setTimeout(() => setCopiedExample(null), 2000);
  };

  const handleExampleClick = (query: string) => {
    onExampleClick?.(query);
  };

  const renderExampleCard = (example: SearchExample) => (
    <Card 
      key={example.query} 
      variant="outlined" 
      sx={{ 
        mb: 1.5,
        transition: 'all 0.2s',
        backgroundColor: (theme) => theme.palette.mode === 'dark' ? 'grey.800' : 'background.paper',
        '&:hover': {
          boxShadow: 2,
          transform: 'translateY(-2px)',
        },
      }}
    >
      <CardContent sx={{ py: 1.5, px: 2 }}>
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Box display="flex" alignItems="center" gap={1} flex={1}>
            {example.icon && (
              <Box sx={{ color: 'primary.main' }}>
                {example.icon}
              </Box>
            )}
            <Box flex={1}>
              <Typography 
                variant="body2" 
                fontFamily="monospace" 
                sx={{ 
                  backgroundColor: (theme) => theme.palette.mode === 'dark' ? 'grey.800' : 'grey.100',
                  px: 1,
                  py: 0.5,
                  borderRadius: 1,
                  display: 'inline-block',
                  mb: 0.5,
                }}
              >
                {example.query}
              </Typography>
              <Typography variant="caption" color="text.secondary" display="block">
                {example.description}
              </Typography>
            </Box>
          </Box>
          <Box display="flex" gap={0.5}>
            <Tooltip title="Copy to clipboard">
              <IconButton 
                size="small"
                onClick={() => handleCopyExample(example.query)}
                sx={{ 
                  color: copiedExample === example.query ? 'success.main' : 'text.secondary' 
                }}
              >
                <CopyIcon fontSize="small" />
              </IconButton>
            </Tooltip>
            <Tooltip title="Try this search">
              <IconButton 
                size="small"
                color="primary"
                onClick={() => handleExampleClick(example.query)}
              >
                <PlayIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
        </Box>
      </CardContent>
    </Card>
  );

  const renderQuickTips = () => (
    <Alert severity="info" sx={{ mb: 2 }}>
      <Typography variant="subtitle2" gutterBottom>
        <TipIcon sx={{ verticalAlign: 'middle', mr: 1 }} />
        Quick Tips
      </Typography>
      <List dense sx={{ mt: 1 }}>
        <ListItem>
          <ListItemText 
            primary="Use quotes for exact phrases"
            secondary='"annual report" finds the exact phrase'
          />
        </ListItem>
        <ListItem>
          <ListItemText 
            primary="Combine filters for precision"
            secondary='type:pdf tag:important date:2024'
          />
        </ListItem>
        <ListItem>
          <ListItemText 
            primary="Use wildcards for variations"
            secondary='doc* matches document, documentation, docs'
          />
        </ListItem>
      </List>
    </Alert>
  );

  if (compact && !expanded) {
    return (
      <Paper variant="outlined" sx={{ 
        p: 2, 
        mb: 2, 
        backgroundColor: (theme) => theme.palette.mode === 'dark' ? 'grey.900' : 'background.paper' 
      }}>
        <Box display="flex" alignItems="center" justifyContent="space-between">
          <Box display="flex" alignItems="center" gap={1}>
            <TipIcon color="primary" />
            <Typography variant="body2">
              Need help with search? View examples and syntax guide
            </Typography>
          </Box>
          <Button
            size="small"
            endIcon={<ExpandMoreIcon />}
            onClick={() => setExpanded(true)}
          >
            Show Guide
          </Button>
        </Box>
      </Paper>
    );
  }

  return (
    <Paper elevation={0} sx={{ p: 3, mb: 3, backgroundColor: (theme) => theme.palette.mode === 'dark' ? 'grey.900' : 'grey.50' }}>
      <Box display="flex" alignItems="center" justifyContent="space-between" mb={2}>
        <Typography variant="h6" display="flex" alignItems="center" gap={1}>
          <TipIcon color="primary" />
          Search Guide
        </Typography>
        {compact && (
          <IconButton onClick={() => setExpanded(false)} size="small">
            <ExpandMoreIcon sx={{ transform: 'rotate(180deg)' }} />
          </IconButton>
        )}
      </Box>

      {renderQuickTips()}

      <Tabs 
        value={activeTab} 
        onChange={(_, newValue) => setActiveTab(newValue)}
        sx={{ mb: 2 }}
      >
        <Tab label={`Basic (${categorizedExamples.basic.length})`} />
        <Tab label={`Advanced (${categorizedExamples.advanced.length})`} />
        <Tab label={`Filters (${categorizedExamples.filters.length})`} />
        <Tab label={`Power User (${categorizedExamples.operators.length})`} />
      </Tabs>

      <Box role="tabpanel" hidden={activeTab !== 0}>
        <Grid container spacing={2}>
          {categorizedExamples.basic.map(example => (
            <Grid item xs={12} md={6} key={example.query}>
              {renderExampleCard(example)}
            </Grid>
          ))}
        </Grid>
      </Box>

      <Box role="tabpanel" hidden={activeTab !== 1}>
        <Grid container spacing={2}>
          {categorizedExamples.advanced.map(example => (
            <Grid item xs={12} md={6} key={example.query}>
              {renderExampleCard(example)}
            </Grid>
          ))}
        </Grid>
      </Box>

      <Box role="tabpanel" hidden={activeTab !== 2}>
        <Grid container spacing={2}>
          {categorizedExamples.filters.map(example => (
            <Grid item xs={12} md={6} key={example.query}>
              {renderExampleCard(example)}
            </Grid>
          ))}
        </Grid>
      </Box>

      <Box role="tabpanel" hidden={activeTab !== 3}>
        <Grid container spacing={2}>
          {categorizedExamples.operators.map(example => (
            <Grid item xs={12} key={example.query}>
              {renderExampleCard(example)}
            </Grid>
          ))}
        </Grid>
      </Box>

      <Divider sx={{ my: 2 }} />
      
      <Typography variant="caption" color="text.secondary" display="block" textAlign="center">
        Click <PlayIcon sx={{ fontSize: 14, verticalAlign: 'middle' }} /> to try an example, 
        or <CopyIcon sx={{ fontSize: 14, verticalAlign: 'middle' }} /> to copy it
      </Typography>
    </Paper>
  );
};

export default EnhancedSearchGuide;