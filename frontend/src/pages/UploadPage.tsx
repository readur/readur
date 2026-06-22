import React from 'react';
import { useTranslation } from 'react-i18next';
import { Box } from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import UploadZone from '../components/Upload/UploadZone';
import { PageHeader, Panel, PanelHead } from '../design/components';

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

  const handleUploadComplete = (document: UploadedDocument): void => {
    console.log('Upload completed:', document);
  };

  const tips: string[] = [
    t('upload.tips.highRes'),
    t('upload.tips.pdfText'),
    t('upload.tips.clarity'),
    t('upload.tips.maxSize'),
  ];

  return (
    <Box>
      <PageHeader
        kicker={t('navigation.sections.ingest')}
        title={t('upload.title')}
        subtitle={t('upload.subtitle')}
      />

      <Grid container spacing={3}>
        <Grid item xs={12} lg={8}>
          <UploadZone onUploadComplete={handleUploadComplete} />
        </Grid>
        <Grid item xs={12} lg={4}>
          <Panel flush>
            <PanelHead title={t('upload.tips.title')} subtitle="Best practices" />
            <Box
              component="ul"
              sx={{
                listStyle: 'none',
                margin: 0,
                padding: 'var(--s-4) var(--s-5)',
                display: 'flex',
                flexDirection: 'column',
                gap: 'var(--s-3)',
              }}
            >
              {tips.map((tip, i) => (
                <Box
                  key={i}
                  component="li"
                  sx={{
                    display: 'flex',
                    gap: 'var(--s-3)',
                    fontFamily: 'var(--font-sans)',
                    fontSize: 'var(--fs-body)',
                    lineHeight: 'var(--lh-body)',
                    color: 'var(--fg-2)',
                  }}
                >
                  <Box
                    aria-hidden
                    sx={{
                      width: 6,
                      height: 6,
                      borderRadius: '50%',
                      background: 'var(--accent-50)',
                      marginTop: 8,
                      flexShrink: 0,
                    }}
                  />
                  <Box sx={{ minWidth: 0 }}>{tip}</Box>
                </Box>
              ))}
            </Box>
          </Panel>
        </Grid>
      </Grid>
    </Box>
  );
};

export default UploadPage;
