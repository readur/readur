import React from 'react';
import {
  Box,
  Typography,
  Container,
  Paper,
  Card,
  CardContent,
  List,
  ListItem,
  ListItemIcon,
  ListItemText,
} from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  CloudUpload as UploadIcon,
  AutoAwesome as AutoIcon,
  Search as SearchIcon,
  Security as SecurityIcon,
  Speed as SpeedIcon,
  Language as LanguageIcon,
} from '@mui/icons-material';
import UploadZone from '../components/Upload/UploadZone';
import { useNavigate } from 'react-router-dom';

interface Feature {
  icon: React.ComponentType<any>;
  title: string;
  description: string;
}

interface UploadedDocument {
  id: string;
  original_filename: string;
  filename: string;
  file_size: number;
  mime_type: string;
  created_at: string;
}

const features: Feature[] = [
  {
    icon: AutoIcon,
    title: 'AI-Powered OCR',
    description: 'Advanced text extraction from any document type',
  },
  {
    icon: SearchIcon,
    title: 'Full-Text Search',
    description: 'Find documents instantly by content or metadata',
  },
  {
    icon: SpeedIcon,
    title: 'Lightning Fast',
    description: 'Process documents in seconds, not minutes',
  },
  {
    icon: SecurityIcon,
    title: 'Secure & Private',
    description: 'Your documents are encrypted and protected',
  },
  {
    icon: LanguageIcon,
    title: 'Multi-Language',
    description: 'Support for 100+ languages and scripts',
  },
];

const UploadPage: React.FC = () => {
  const navigate = useNavigate();

  const handleUploadComplete = (document: UploadedDocument): void => {
    // Optionally navigate to the document or show a success message
    console.log('Upload completed:', document);
  };

  return (
    <Container maxWidth="lg">
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" sx={{ fontWeight: 700, mb: 1 }}>
          Upload Documents
        </Typography>
        <Typography variant="h6" color="text.secondary">
          Transform your documents with intelligent OCR processing
        </Typography>
      </Box>

      <Grid container spacing={4}>
        {/* Upload Zone */}
        <Grid item xs={12} lg={8}>
          <UploadZone onUploadComplete={handleUploadComplete} />
        </Grid>

        {/* Features Sidebar */}
        <Grid item xs={12} lg={4}>
          <Card elevation={0}>
            <CardContent>
              <Box sx={{ display: 'flex', alignItems: 'center', mb: 3 }}>
                <UploadIcon sx={{ fontSize: 28, color: 'primary.main', mr: 1 }} />
                <Typography variant="h6" sx={{ fontWeight: 600 }}>
                  Why Choose Readur?
                </Typography>
              </Box>
              
              <List sx={{ p: 0 }}>
                {features.map((feature, index) => (
                  <ListItem 
                    key={feature.title}
                    sx={{ 
                      px: 0,
                      py: 2,
                      borderBottom: index < features.length - 1 ? 1 : 0,
                      borderColor: 'divider',
                    }}
                  >
                    <ListItemIcon sx={{ minWidth: 40 }}>
                      <feature.icon sx={{ color: 'primary.main' }} />
                    </ListItemIcon>
                    <ListItemText
                      primary={
                        <Typography variant="subtitle2" sx={{ fontWeight: 600, mb: 0.5 }}>
                          {feature.title}
                        </Typography>
                      }
                      secondary={
                        <Typography variant="body2" color="text.secondary">
                          {feature.description}
                        </Typography>
                      }
                    />
                  </ListItem>
                ))}
              </List>
            </CardContent>
          </Card>

          {/* Tips Card */}
          <Card elevation={0} sx={{ mt: 3 }}>
            <CardContent>
              <Typography variant="h6" sx={{ fontWeight: 600, mb: 2 }}>
                📋 Upload Tips
              </Typography>
              <List dense sx={{ p: 0 }}>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    • For best OCR results, use high-resolution images
                  </Typography>
                </ListItem>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    • PDF files with text layers are processed faster
                  </Typography>
                </ListItem>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    • Ensure documents are well-lit and clearly readable
                  </Typography>
                </ListItem>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    • Maximum file size is 50MB per document
                  </Typography>
                </ListItem>
              </List>
            </CardContent>
          </Card>
        </Grid>
      </Grid>
    </Container>
  );
};

export default UploadPage;