import React from 'react';
import { useTranslation } from 'react-i18next';
import {
  Box,
  Typography,
  Container,
  Card,
  CardContent,
  List,
  ListItem,
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

const UploadPage: React.FC = () => {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const features: Feature[] = [
    {
      icon: AutoIcon,
      title: t('upload.features.aiOcr.title'),
      description: t('upload.features.aiOcr.description'),
    },
    {
      icon: SearchIcon,
      title: t('upload.features.fullTextSearch.title'),
      description: t('upload.features.fullTextSearch.description'),
    },
    {
      icon: SpeedIcon,
      title: t('upload.features.lightningFast.title'),
      description: t('upload.features.lightningFast.description'),
    },
    {
      icon: SecurityIcon,
      title: t('upload.features.secure.title'),
      description: t('upload.features.secure.description'),
    },
    {
      icon: LanguageIcon,
      title: t('upload.features.multiLanguage.title'),
      description: t('upload.features.multiLanguage.description'),
    },
  ];

  const handleUploadComplete = (document: UploadedDocument): void => {
    // Optionally navigate to the document or show a success message
    console.log('Upload completed:', document);
  };

  return (
    <Container maxWidth="lg">
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" sx={{ fontWeight: 700, mb: 1 }}>
          {t('upload.title')}
        </Typography>
        <Typography variant="h6" color="text.secondary">
          {t('upload.subtitle')}
        </Typography>
      </Box>

      <Grid container spacing={4}>
        {/* Upload Zone */}
        <Grid item xs={12} lg={8}>
          <UploadZone onUploadComplete={handleUploadComplete} />
        </Grid>

        {/* Features Sidebar */}
        <Grid item xs={12} lg={4}>

          {/* Tips Card */}
          <Card elevation={0} sx={{ mt: 3 }}>
            <CardContent>
              <Typography variant="h6" sx={{ fontWeight: 600, mb: 2 }}>
                {t('upload.tips.title')}
              </Typography>
              <List dense sx={{ p: 0 }}>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    {t('upload.tips.highRes')}
                  </Typography>
                </ListItem>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    {t('upload.tips.pdfText')}
                  </Typography>
                </ListItem>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    {t('upload.tips.clarity')}
                  </Typography>
                </ListItem>
                <ListItem sx={{ px: 0 }}>
                  <Typography variant="body2" color="text.secondary">
                    {t('upload.tips.maxSize')}
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