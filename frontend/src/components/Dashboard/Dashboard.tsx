import React, { useState, useEffect } from 'react';
import { Box, Button, IconButton, Tooltip } from '@mui/material';
import Grid from '@mui/material/GridLegacy';
import {
  CloudUpload as UploadIcon,
  Article as DocumentIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  TextSnippet as TextIcon,
  InsertDriveFile as FileIcon,
  Visibility as ViewIcon,
  GetApp as DownloadIcon,
} from '../../design/icons';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import api, { documentService } from '../../services/api';
import { useTranslation } from 'react-i18next';
import {
  PageHeader,
  Panel,
  PanelHead,
  StatCard,
  Pill,
  MetaText,
  EmptyState,
} from '../../design/components';

interface Document {
  id: string;
  original_filename?: string;
  filename?: string;
  file_size?: number;
  mime_type?: string;
  created_at?: string;
  ocr_text?: string;
  has_ocr_text?: boolean;
}

interface DashboardStats {
  totalDocuments: number;
  totalSize: number;
  ocrProcessed: number;
  searchablePages: number;
}

const formatBytes = (bytes: number): string => {
  if (!bytes) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
};

const formatDate = (dateString?: string): string => {
  if (!dateString) return '—';
  const d = new Date(dateString);
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, '0');
  const day = String(d.getDate()).padStart(2, '0');
  return `${y}-${m}-${day}`;
};

const getFileIcon = (mimeType?: string): React.ComponentType<any> => {
  if (mimeType?.includes('pdf')) return PdfIcon;
  if (mimeType?.includes('image')) return ImageIcon;
  if (mimeType?.includes('text')) return TextIcon;
  return FileIcon;
};

const getStatus = (doc: Document): { variant: 'ok' | 'warn' | 'neutral'; label: string } => {
  if (doc.has_ocr_text || doc.ocr_text) return { variant: 'ok', label: 'Indexed' };
  if (doc.mime_type?.includes('image') || doc.mime_type?.includes('pdf')) {
    return { variant: 'warn', label: 'Reading' };
  }
  return { variant: 'neutral', label: 'Stored' };
};

const Dashboard: React.FC = () => {
  const navigate = useNavigate();
  const { user } = useAuth();
  const { t } = useTranslation();
  const [documents, setDocuments] = useState<Document[]>([]);
  const [stats, setStats] = useState<DashboardStats>({
    totalDocuments: 0,
    totalSize: 0,
    ocrProcessed: 0,
    searchablePages: 0,
  });
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    const fetchDashboardData = async (): Promise<void> => {
      try {
        let docs: Document[] = [];
        try {
          const docsResponse = await api.get('/documents', { params: { limit: 10, offset: 0 } });
          if (Array.isArray(docsResponse.data)) {
            docs = docsResponse.data;
          } else if (docsResponse.data?.documents) {
            docs = docsResponse.data.documents;
          }
        } catch (docError) {
          console.error('Failed to fetch documents:', docError);
        }
        setDocuments(docs);

        let metricsData: any = null;
        try {
          const metricsResponse = await api.get<any>('/metrics');
          metricsData = metricsResponse.data;
        } catch (metricsError) {
          console.error('Failed to fetch metrics:', metricsError);
        }

        if (metricsData?.documents) {
          setStats({
            totalDocuments: metricsData.documents.total_documents || 0,
            totalSize: metricsData.documents.total_storage_bytes || 0,
            ocrProcessed: metricsData.documents.documents_with_ocr || 0,
            searchablePages: metricsData.documents.documents_with_ocr || 0,
          });
        } else {
          const totalSize = docs.reduce((sum, doc) => sum + (doc.file_size || 0), 0);
          const ocrProcessed = docs.filter((doc) => doc.has_ocr_text || doc.ocr_text).length;
          setStats({
            totalDocuments: docs.length,
            totalSize,
            ocrProcessed,
            searchablePages: docs.length,
          });
        }
      } catch (error) {
        console.error('Unexpected error in dashboard data fetch:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchDashboardData();
  }, []);

  const ocrPct =
    stats.totalDocuments > 0 ? Math.round((stats.ocrProcessed / stats.totalDocuments) * 100) : 0;

  const recentDocs = documents.slice(0, 5);

  return (
    <Box>
      <PageHeader
        kicker={
          user?.username
            ? `${t('common.welcomeBack', { username: user.username })}`
            : t('navigation.dashboard')
        }
        title={
          <>
            {t('navigation.dashboard')} —{' '}
            <Box component="span" sx={{ color: 'var(--accent-60)', fontWeight: 800 }}>
              {loading ? '…' : stats.totalDocuments.toLocaleString()}
            </Box>{' '}
            documents
          </>
        }
        subtitle={
          loading
            ? t('dashboard.greeting')
            : `${formatBytes(stats.totalSize)} indexed across the library. ${stats.ocrProcessed} of ${stats.totalDocuments} processed with OCR.`
        }
        actions={
          <Button variant="contained" startIcon={<UploadIcon />} onClick={() => navigate('/upload')}>
            {t('navigation.upload')}
          </Button>
        }
      />

      {/* Stat strip */}
      <Grid container spacing={2} sx={{ mb: 'var(--s-8)' }}>
        <Grid item xs={12} sm={6} lg={3}>
          <StatCard
            label={t('dashboard.stats.totalDocuments.title')}
            value={loading ? '—' : stats.totalDocuments.toLocaleString()}
            delta={
              loading
                ? undefined
                : stats.totalDocuments > 0
                  ? t('dashboard.stats.totalDocuments.trend', { count: stats.totalDocuments })
                  : t('dashboard.stats.totalDocuments.trendEmpty')
            }
            trend={stats.totalDocuments > 0 ? 'neutral' : 'neutral'}
          />
        </Grid>
        <Grid item xs={12} sm={6} lg={3}>
          <StatCard
            label={t('dashboard.stats.storageUsed.title')}
            value={loading ? '—' : formatBytes(stats.totalSize).split(' ')[0]}
            unit={loading ? undefined : formatBytes(stats.totalSize).split(' ')[1]}
            delta={
              loading
                ? undefined
                : stats.totalSize > 0
                  ? t('dashboard.stats.storageUsed.trend', { size: formatBytes(stats.totalSize) })
                  : t('dashboard.stats.storageUsed.trendEmpty')
            }
            trend="neutral"
          />
        </Grid>
        <Grid item xs={12} sm={6} lg={3}>
          <StatCard
            label={t('dashboard.stats.ocrProcessed.title')}
            value={loading ? '—' : stats.ocrProcessed.toLocaleString()}
            delta={
              loading
                ? undefined
                : stats.totalDocuments > 0
                  ? t('dashboard.stats.ocrProcessed.trend', { percentage: ocrPct })
                  : t('dashboard.stats.ocrProcessed.trendEmpty')
            }
            trend={ocrPct >= 80 ? 'up' : 'neutral'}
          />
        </Grid>
        <Grid item xs={12} sm={6} lg={3}>
          <StatCard
            label={t('dashboard.stats.searchable.title')}
            value={loading ? '—' : stats.searchablePages.toLocaleString()}
            delta={
              loading
                ? undefined
                : stats.searchablePages > 0
                  ? t('dashboard.stats.searchable.trend', { count: stats.searchablePages })
                  : t('dashboard.stats.searchable.trendEmpty')
            }
            trend="neutral"
          />
        </Grid>
      </Grid>

      {/* Recently added */}
      <Panel flush>
        <PanelHead
          title={t('dashboard.recentDocuments.title')}
          subtitle={`Last ${recentDocs.length} files`}
          action={
            <Box
              component="button"
              onClick={() => navigate('/documents')}
              sx={{
                background: 'transparent',
                border: 'none',
                cursor: 'pointer',
                fontFamily: 'var(--font-sans)',
                fontWeight: 500,
                fontSize: 12,
                color: 'var(--accent-60)',
                '&:hover': { color: 'var(--accent-70)' },
              }}
            >
              {t('dashboard.recentDocuments.viewAll')} →
            </Box>
          }
        />
        {recentDocs.length === 0 ? (
          <Box sx={{ p: 'var(--s-6)' }}>
            <EmptyState
              icon={<DocumentIcon sx={{ fontSize: 32 }} />}
              title={t('dashboard.recentDocuments.noDocuments')}
              description={t('dashboard.recentDocuments.uploadFirst')}
              action={
                <Button variant="contained" onClick={() => navigate('/upload')}>
                  {t('navigation.upload')}
                </Button>
              }
            />
          </Box>
        ) : (
          <Box sx={{ overflowX: 'auto' }}>
            <Box
              component="table"
              sx={{
                width: '100%',
                borderCollapse: 'collapse',
                fontFamily: 'var(--font-sans)',
                fontSize: 13,
                lineHeight: 1.3,
              }}
            >
              <thead>
                <tr>
                  <Box
                    component="th"
                    sx={{
                      textAlign: 'left',
                      fontFamily: 'var(--font-sans)',
                      fontWeight: 600,
                      fontSize: 10,
                      letterSpacing: 'var(--tracking-caps)',
                      textTransform: 'uppercase',
                      color: 'var(--fg-3)',
                      padding: '12px 20px',
                      borderBottom: '1px solid var(--line-1)',
                      background: 'var(--bg-2)',
                    }}
                  >
                    Document
                  </Box>
                  <Box component="th" sx={thBase}>
                    Size
                  </Box>
                  <Box component="th" sx={thBase}>
                    Added
                  </Box>
                  <Box component="th" sx={thBase}>
                    Status
                  </Box>
                  <Box component="th" sx={{ ...thBase, textAlign: 'right' }}>
                    {' '}
                  </Box>
                </tr>
              </thead>
              <tbody>
                {recentDocs.map((doc) => {
                  const Icon = getFileIcon(doc.mime_type);
                  const status = getStatus(doc);
                  const name = doc.original_filename || doc.filename || 'Unknown document';
                  return (
                    <Box
                      component="tr"
                      key={doc.id}
                      sx={{
                        cursor: 'pointer',
                        '&:hover td': { background: 'var(--accent-05)' },
                      }}
                      onClick={() => navigate(`/documents/${doc.id}`)}
                    >
                      <Box
                        component="td"
                        sx={{
                          ...tdBase,
                          color: 'var(--fg-0)',
                          fontWeight: 500,
                          display: 'flex',
                          alignItems: 'center',
                          gap: 'var(--s-3)',
                          minWidth: 0,
                        }}
                      >
                        <Icon sx={{ color: 'var(--accent-50)', fontSize: 18, flexShrink: 0 }} />
                        <Box
                          sx={{
                            overflow: 'hidden',
                            textOverflow: 'ellipsis',
                            whiteSpace: 'nowrap',
                            minWidth: 0,
                          }}
                        >
                          {name}
                        </Box>
                      </Box>
                      <Box component="td" sx={tdBase}>
                        <MetaText>{formatBytes(doc.file_size || 0)}</MetaText>
                      </Box>
                      <Box component="td" sx={tdBase}>
                        <MetaText>{formatDate(doc.created_at)}</MetaText>
                      </Box>
                      <Box component="td" sx={tdBase}>
                        <Pill variant={status.variant}>{status.label}</Pill>
                      </Box>
                      <Box component="td" sx={{ ...tdBase, textAlign: 'right' }}>
                        <Box
                          sx={{ display: 'inline-flex', gap: 0.5 }}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <Tooltip title="View">
                            <IconButton
                              size="small"
                              onClick={() => navigate(`/documents/${doc.id}`)}
                            >
                              <ViewIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                          <Tooltip title="Download">
                            <IconButton
                              size="small"
                              onClick={async () => {
                                try {
                                  await documentService.downloadFile(
                                    doc.id,
                                    doc.original_filename || doc.filename,
                                  );
                                } catch (error) {
                                  console.error('Download failed:', error);
                                }
                              }}
                            >
                              <DownloadIcon fontSize="small" />
                            </IconButton>
                          </Tooltip>
                        </Box>
                      </Box>
                    </Box>
                  );
                })}
              </tbody>
            </Box>
          </Box>
        )}
      </Panel>
    </Box>
  );
};

const thBase = {
  textAlign: 'left',
  fontFamily: 'var(--font-sans)',
  fontWeight: 600,
  fontSize: 10,
  letterSpacing: 'var(--tracking-caps)',
  textTransform: 'uppercase',
  color: 'var(--fg-3)',
  padding: '12px 20px',
  borderBottom: '1px solid var(--line-1)',
  background: 'var(--bg-2)',
} as const;

const tdBase = {
  padding: '13px 20px',
  borderBottom: '1px solid var(--line-1)',
  color: 'var(--fg-1)',
  verticalAlign: 'middle',
  transition: 'background var(--dur-fast) var(--ease-out)',
} as const;

export default Dashboard;
