import React, { useState, useEffect } from 'react';
import {
  Box,
  Grid,
  Card,
  CardContent,
  Typography,
  LinearProgress,
  Chip,
  Avatar,
  List,
  ListItem,
  ListItemAvatar,
  ListItemText,
  ListItemSecondaryAction,
  IconButton,
  Fab,
  Paper,
  useTheme,
  alpha,
} from '@mui/material';
import {
  CloudUpload as UploadIcon,
  Article as DocumentIcon,
  Search as SearchIcon,
  TrendingUp as TrendingUpIcon,
  CloudDone as StorageIcon,
  AutoAwesome as OcrIcon,
  FindInPage as SearchableIcon,
  Add as AddIcon,
  GetApp as DownloadIcon,
  Visibility as ViewIcon,
  Delete as DeleteIcon,
  InsertDriveFile as FileIcon,
  PictureAsPdf as PdfIcon,
  Image as ImageIcon,
  TextSnippet as TextIcon,
} from '@mui/icons-material';
import { useNavigate } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import api from '../../services/api';

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

interface StatsCardProps {
  title: string;
  value: string | number;
  subtitle: string;
  icon: React.ComponentType<any>;
  color: string;
  trend?: string;
}

interface RecentDocumentsProps {
  documents: Document[];
}

interface QuickAction {
  title: string;
  description: string;
  icon: React.ComponentType<any>;
  color: string;
  path: string;
}

// Stats Card Component
const StatsCard: React.FC<StatsCardProps> = ({ title, value, subtitle, icon: Icon, color, trend }) => {
  const theme = useTheme();
  
  return (
    <Card
      elevation={0}
      sx={{
        background: `linear-gradient(135deg, ${color} 0%, ${alpha(color, 0.85)} 100%)`,
        color: 'white',
        position: 'relative',
        overflow: 'hidden',
        borderRadius: 3,
        transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
        cursor: 'pointer',
        border: '1px solid rgba(255,255,255,0.1)',
        backdropFilter: 'blur(20px)',
        '&:hover': {
          transform: 'translateY(-4px)',
          boxShadow: `0 20px 40px ${alpha(color, 0.3)}`,
        },
        '&::before': {
          content: '""',
          position: 'absolute',
          top: 0,
          right: 0,
          width: '120px',
          height: '120px',
          background: 'linear-gradient(135deg, rgba(255,255,255,0.15) 0%, rgba(255,255,255,0.05) 100%)',
          borderRadius: '50%',
          transform: 'translate(40px, -40px)',
        },
        '&::after': {
          content: '""',
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0.05) 100%)',
          backdropFilter: 'blur(10px)',
        },
      }}
    >
      <CardContent sx={{ position: 'relative', zIndex: 1, p: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'flex-start', justifyContent: 'space-between' }}>
          <Box sx={{ flex: 1 }}>
            <Typography variant="h3" sx={{ 
              fontWeight: 800, 
              mb: 1.5,
              letterSpacing: '-0.025em',
              fontSize: { xs: '1.75rem', sm: '2.125rem' },
            }}>
              {value}
            </Typography>
            <Typography variant="h6" sx={{ 
              opacity: 0.85, 
              mb: 0.5,
              fontWeight: 600,
              letterSpacing: '0.025em',
            }}>
              {title}
            </Typography>
            <Typography variant="body2" sx={{ 
              opacity: 0.75,
              fontWeight: 500,
              fontSize: '0.875rem',
            }}>
              {subtitle}
            </Typography>
            {trend && (
              <Box sx={{ display: 'flex', alignItems: 'center', mt: 1.5 }}>
                <Box sx={{
                  p: 0.5,
                  borderRadius: 1,
                  background: 'rgba(255,255,255,0.2)',
                  backdropFilter: 'blur(10px)',
                  display: 'flex',
                  alignItems: 'center',
                  mr: 1,
                }}>
                  <TrendingUpIcon sx={{ fontSize: 14 }} />
                </Box>
                <Typography variant="caption" sx={{ 
                  opacity: 0.8,
                  fontWeight: 600,
                  fontSize: '0.75rem',
                  letterSpacing: '0.025em',
                }}>
                  {trend}
                </Typography>
              </Box>
            )}
          </Box>
          <Box sx={{
            width: 64,
            height: 64,
            borderRadius: 3,
            background: 'linear-gradient(135deg, rgba(255,255,255,0.25) 0%, rgba(255,255,255,0.15) 100%)',
            backdropFilter: 'blur(20px)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            border: '1px solid rgba(255,255,255,0.2)',
            boxShadow: '0 8px 32px rgba(0,0,0,0.1)',
          }}>
            <Icon sx={{ 
              fontSize: 32,
              filter: 'drop-shadow(0 2px 4px rgba(0,0,0,0.1))',
            }} />
          </Box>
        </Box>
      </CardContent>
    </Card>
  );
};

// Recent Documents Component
const RecentDocuments: React.FC<RecentDocumentsProps> = ({ documents = [] }) => {
  const navigate = useNavigate();
  const theme = useTheme();

  const getFileIcon = (mimeType?: string): React.ComponentType<any> => {
    if (mimeType?.includes('pdf')) return PdfIcon;
    if (mimeType?.includes('image')) return ImageIcon;
    if (mimeType?.includes('text')) return TextIcon;
    return FileIcon;
  };

  const formatFileSize = (bytes?: number): string => {
    if (!bytes) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  const formatDate = (dateString?: string): string => {
    if (!dateString) return 'Unknown';
    return new Date(dateString).toLocaleDateString('en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  };

  return (
    <Card elevation={0} sx={{
      background: theme.palette.mode === 'light'
        ? 'linear-gradient(180deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.95) 100%)'
        : 'linear-gradient(180deg, rgba(40,40,40,0.95) 0%, rgba(25,25,25,0.95) 100%)',
      backdropFilter: 'blur(20px)',
      border: theme.palette.mode === 'light'
        ? '1px solid rgba(226,232,240,0.5)'
        : '1px solid rgba(255,255,255,0.1)',
      borderRadius: 3,
    }}>
      <CardContent sx={{ p: 3 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', mb: 3 }}>
          <Typography variant="h6" sx={{ 
            fontWeight: 700,
            letterSpacing: '-0.025em',
            background: theme.palette.mode === 'light'
              ? 'linear-gradient(135deg, #1e293b 0%, #6366f1 100%)'
              : 'linear-gradient(135deg, #f8fafc 0%, #a855f7 100%)',
            backgroundClip: 'text',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
          }}>
            Recent Documents
          </Typography>
          <Chip
            label="View All"
            onClick={() => navigate('/documents')}
            sx={{ 
              cursor: 'pointer',
              background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
              border: '1px solid rgba(99,102,241,0.3)',
              fontWeight: 600,
              transition: 'all 0.2s ease-in-out',
              '&:hover': {
                background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                color: 'white',
                transform: 'translateY(-2px)',
                boxShadow: '0 8px 24px rgba(99,102,241,0.2)',
              },
            }}
          />
        </Box>
        
        {documents.length === 0 ? (
          <Box sx={{ textAlign: 'center', py: 4 }}>
            <Box sx={{
              width: 64,
              height: 64,
              borderRadius: 3,
              background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              mx: 'auto',
              mb: 2,
            }}>
              <DocumentIcon sx={{ fontSize: 32, color: '#6366f1' }} />
            </Box>
            <Typography variant="body1" sx={{ 
              color: 'text.secondary',
              fontWeight: 500,
              mb: 1,
            }}>
              No documents yet
            </Typography>
            <Typography variant="body2" color="text.secondary">
              Upload your first document to get started
            </Typography>
          </Box>
        ) : (
          <List sx={{ p: 0 }}>
            {documents.slice(0, 5).map((doc, index) => {
              const FileIconComponent = getFileIcon(doc.mime_type);
              
              return (
                <ListItem
                  key={doc.id || index}
                  sx={{
                    px: 0,
                    py: 1.5,
                    borderBottom: index < Math.min(documents.length, 5) - 1 ? 1 : 0,
                    borderColor: 'divider',
                  }}
                >
                  <ListItemAvatar>
                    <Avatar
                      sx={{
                        bgcolor: 'primary.main',
                        color: 'primary.contrastText',
                      }}
                    >
                      <FileIconComponent />
                    </Avatar>
                  </ListItemAvatar>
                  <ListItemText
                    sx={{ 
                      pr: 8, // Add padding-right to prevent overlap with secondary action
                    }}
                    primary={
                      <Typography
                        variant="subtitle2"
                        sx={{
                          fontWeight: 500,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                          maxWidth: '100%',
                        }}
                      >
                        {doc.original_filename || doc.filename || 'Unknown Document'}
                      </Typography>
                    }
                    secondary={
                      <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mt: 0.5 }}>
                        <Typography variant="caption" color="text.secondary">
                          {formatFileSize(doc.file_size)}
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          •
                        </Typography>
                        <Typography variant="caption" color="text.secondary">
                          {formatDate(doc.created_at)}
                        </Typography>
                      </Box>
                    }
                  />
                  <ListItemSecondaryAction>
                    <Box sx={{ display: 'flex', gap: 0.5 }}>
                      <IconButton size="small" onClick={() => navigate(`/documents/${doc.id}`)}>
                        <ViewIcon fontSize="small" />
                      </IconButton>
                      <IconButton size="small">
                        <DownloadIcon fontSize="small" />
                      </IconButton>
                    </Box>
                  </ListItemSecondaryAction>
                </ListItem>
              );
            })}
          </List>
        )}
      </CardContent>
    </Card>
  );
};

// Quick Actions Component
const QuickActions: React.FC = () => {
  const navigate = useNavigate();
  const theme = useTheme();
  
  const actions: QuickAction[] = [
    {
      title: 'Upload Documents',
      description: 'Add new files for OCR processing',
      icon: UploadIcon,
      color: '#6366f1',
      path: '/upload',
    },
    {
      title: 'Search Library',
      description: 'Find documents by content or metadata',
      icon: SearchIcon,
      color: '#10b981',
      path: '/search',
    },
    {
      title: 'Browse Documents',
      description: 'View and manage your document library',
      icon: SearchableIcon,
      color: '#f59e0b',
      path: '/documents',
    },
  ];

  return (
    <Card elevation={0} sx={{
      background: theme.palette.mode === 'light'
        ? 'linear-gradient(180deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.95) 100%)'
        : 'linear-gradient(180deg, rgba(40,40,40,0.95) 0%, rgba(25,25,25,0.95) 100%)',
      backdropFilter: 'blur(20px)',
      border: theme.palette.mode === 'light'
        ? '1px solid rgba(226,232,240,0.5)'
        : '1px solid rgba(255,255,255,0.1)',
      borderRadius: 3,
    }}>
      <CardContent sx={{ p: 3 }}>
        <Typography variant="h6" sx={{ 
          fontWeight: 700,
          letterSpacing: '-0.025em',
          background: theme.palette.mode === 'light'
            ? 'linear-gradient(135deg, #1e293b 0%, #6366f1 100%)'
            : 'linear-gradient(135deg, #f8fafc 0%, #a855f7 100%)',
          backgroundClip: 'text',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
          mb: 3,
        }}>
          Quick Actions
        </Typography>
        <Grid container spacing={2}>
          {actions.map((action) => (
            <Grid item xs={12} key={action.title}>
              <Paper
                elevation={0}
                sx={{
                  p: 2.5,
                  cursor: 'pointer',
                  border: theme.palette.mode === 'light'
                    ? '1px solid rgba(226,232,240,0.5)'
                    : '1px solid rgba(255,255,255,0.1)',
                  borderRadius: 3,
                  background: theme.palette.mode === 'light'
                    ? 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)'
                    : 'linear-gradient(135deg, rgba(50,50,50,0.8) 0%, rgba(30,30,30,0.6) 100%)',
                  backdropFilter: 'blur(10px)',
                  transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
                  '&:hover': {
                    borderColor: action.color,
                    background: `linear-gradient(135deg, ${alpha(action.color, 0.08)} 0%, ${alpha(action.color, 0.04)} 100%)`,
                    transform: 'translateY(-4px)',
                    boxShadow: `0 12px 32px ${alpha(action.color, 0.15)}`,
                  },
                }}
                onClick={() => navigate(action.path)}
              >
                <Box sx={{ display: 'flex', alignItems: 'center', gap: 2.5 }}>
                  <Box sx={{
                    width: 48,
                    height: 48,
                    borderRadius: 3,
                    background: `linear-gradient(135deg, ${action.color} 0%, ${alpha(action.color, 0.8)} 100%)`,
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    color: 'white',
                    boxShadow: `0 8px 24px ${alpha(action.color, 0.3)}`,
                  }}>
                    <action.icon sx={{ fontSize: 24 }} />
                  </Box>
                  <Box sx={{ flex: 1 }}>
                    <Typography variant="subtitle2" sx={{ 
                      fontWeight: 700,
                      letterSpacing: '0.025em',
                      mb: 0.5,
                    }}>
                      {action.title}
                    </Typography>
                    <Typography variant="body2" sx={{
                      color: 'text.secondary',
                      fontWeight: 500,
                      fontSize: '0.875rem',
                    }}>
                      {action.description}
                    </Typography>
                  </Box>
                </Box>
              </Paper>
            </Grid>
          ))}
        </Grid>
      </CardContent>
    </Card>
  );
};

const Dashboard: React.FC = () => {
  const theme = useTheme();
  const navigate = useNavigate();
  const { user } = useAuth();
  const [documents, setDocuments] = useState<Document[]>([]);
  const [stats, setStats] = useState<DashboardStats>({
    totalDocuments: 0,
    totalSize: 0,
    ocrProcessed: 0,
    searchablePages: 0,
  });
  const [loading, setLoading] = useState<boolean>(true);
  const [metrics, setMetrics] = useState<any>(null);

  useEffect(() => {
    const fetchDashboardData = async (): Promise<void> => {
      try {
        // Fetch both documents and metrics
        const [docsResponse, metricsResponse] = await Promise.all([
          api.get<Document[]>('/documents'),
          api.get<any>('/metrics')
        ]);
        
        const docs = docsResponse.data || [];
        setDocuments(docs);
        
        const metricsData = metricsResponse.data;
        setMetrics(metricsData);
        
        // Use backend metrics if available, otherwise fall back to client calculation
        if (metricsData?.documents) {
          setStats({
            totalDocuments: metricsData.documents.total_documents || 0,
            totalSize: metricsData.documents.total_storage_bytes || 0,
            ocrProcessed: metricsData.documents.documents_with_ocr || 0,
            searchablePages: metricsData.documents.documents_with_ocr || 0,
          });
        } else {
          // Fallback to client-side calculation
          const totalSize = docs.reduce((sum, doc) => sum + (doc.file_size || 0), 0);
          const ocrProcessed = docs.filter(doc => doc.ocr_text).length;
          
          setStats({
            totalDocuments: docs.length,
            totalSize,
            ocrProcessed,
            searchablePages: docs.length,
          });
        }
      } catch (error) {
        console.error('Failed to fetch dashboard data:', error);
      } finally {
        setLoading(false);
      }
    };

    fetchDashboardData();
  }, []);

  const formatBytes = (bytes: number): string => {
    if (!bytes) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  };

  return (
    <Box>
      {/* Welcome Header */}
      <Box sx={{ mb: 4 }}>
        <Typography variant="h4" sx={{ 
          fontWeight: 800, 
          mb: 1,
          letterSpacing: '-0.025em',
          background: theme.palette.mode === 'light'
            ? 'linear-gradient(135deg, #1e293b 0%, #6366f1 100%)'
            : 'linear-gradient(135deg, #f8fafc 0%, #a855f7 100%)',
          backgroundClip: 'text',
          WebkitBackgroundClip: 'text',
          WebkitTextFillColor: 'transparent',
        }}>
          Welcome back, {user?.username}! 👋
        </Typography>
        <Typography variant="h6" sx={{
          color: 'text.secondary',
          fontWeight: 500,
          letterSpacing: '0.025em',
        }}>
          Here's what's happening with your documents today.
        </Typography>
      </Box>

      {/* Stats Cards */}
      <Grid container spacing={3} sx={{ mb: 4 }}>
        <Grid item xs={12} sm={6} lg={3}>
          <StatsCard
            title="Total Documents"
            value={loading ? '...' : stats.totalDocuments}
            subtitle="Files in your library"
            icon={DocumentIcon}
            color="#6366f1"
            trend="+12% this month"
          />
        </Grid>
        <Grid item xs={12} sm={6} lg={3}>
          <StatsCard
            title="Storage Used"
            value={loading ? '...' : formatBytes(stats.totalSize)}
            subtitle="Total file size"
            icon={StorageIcon}
            color="#10b981"
            trend="+2.4 GB this week"
          />
        </Grid>
        <Grid item xs={12} sm={6} lg={3}>
          <StatsCard
            title="OCR Processed"
            value={loading ? '...' : stats.ocrProcessed}
            subtitle="Text extracted documents"
            icon={OcrIcon}
            color="#f59e0b"
            trend={stats.totalDocuments > 0 ? `${Math.round((stats.ocrProcessed / stats.totalDocuments) * 100)}% completion` : '0% completion'}
          />
        </Grid>
        <Grid item xs={12} sm={6} lg={3}>
          <StatsCard
            title="Searchable"
            value={loading ? '...' : stats.searchablePages}
            subtitle="Ready for search"
            icon={SearchableIcon}
            color="#8b5cf6"
            trend="100% indexed"
          />
        </Grid>
      </Grid>

      {/* Main Content */}
      <Grid container spacing={3}>
        <Grid item xs={12} lg={8}>
          <RecentDocuments documents={documents} />
        </Grid>
        <Grid item xs={12} lg={4}>
          <QuickActions />
        </Grid>
      </Grid>

      {/* Floating Action Button */}
      <Fab
        color="primary"
        sx={{
          position: 'fixed',
          bottom: 32,
          right: 32,
          width: 64,
          height: 64,
          background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
          border: '1px solid rgba(255,255,255,0.2)',
          backdropFilter: 'blur(20px)',
          boxShadow: '0 16px 40px rgba(99,102,241,0.3)',
          transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
          '&:hover': {
            background: 'linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%)',
            transform: 'translateY(-4px) scale(1.05)',
            boxShadow: '0 20px 50px rgba(99,102,241,0.4)',
          },
          '&::before': {
            content: '""',
            position: 'absolute',
            top: 0,
            left: 0,
            right: 0,
            bottom: 0,
            borderRadius: '50%',
            background: 'linear-gradient(135deg, rgba(255,255,255,0.2) 0%, rgba(255,255,255,0.1) 100%)',
            backdropFilter: 'blur(10px)',
          },
        }}
        onClick={() => navigate('/upload')}
      >
        <AddIcon sx={{ 
          fontSize: 28,
          position: 'relative',
          zIndex: 1,
          filter: 'drop-shadow(0 2px 4px rgba(0,0,0,0.1))',
        }} />
      </Fab>
    </Box>
  );
};

export default Dashboard;