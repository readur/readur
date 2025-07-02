import React, { useState, useEffect } from 'react';
import {
  Card,
  CardContent,
  Typography,
  Button,
  Box,
  Alert,
  LinearProgress,
  Chip,
  Stack,
  Divider,
  Tooltip,
  IconButton,
} from '@mui/material';
import {
  Lightbulb as LightbulbIcon,
  Refresh as RefreshIcon,
  TrendingUp as TrendingUpIcon,
  Info as InfoIcon,
} from '@mui/icons-material';
import { documentService, OcrRetryRecommendation, BulkOcrRetryResponse } from '../services/api';

interface RetryRecommendationsProps {
  onRetrySuccess?: (result: BulkOcrRetryResponse) => void;
  onRetryClick?: (recommendation: OcrRetryRecommendation) => void;
}

export const RetryRecommendations: React.FC<RetryRecommendationsProps> = ({
  onRetrySuccess,
  onRetryClick,
}) => {
  const [recommendations, setRecommendations] = useState<OcrRetryRecommendation[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [retryingRecommendation, setRetryingRecommendation] = useState<string | null>(null);

  const loadRecommendations = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await documentService.getRetryRecommendations();
      setRecommendations(response.data.recommendations);
    } catch (err: any) {
      setError(err.response?.data?.message || 'Failed to load retry recommendations');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadRecommendations();
  }, []);

  const handleRetryRecommendation = async (recommendation: OcrRetryRecommendation) => {
    if (onRetryClick) {
      onRetryClick(recommendation);
      return;
    }

    setRetryingRecommendation(recommendation.reason);
    try {
      const response = await documentService.bulkRetryOcr({
        mode: 'filter',
        filter: recommendation.filter,
        preview_only: false,
      });
      
      if (onRetrySuccess) {
        onRetrySuccess(response.data);
      }
      
      // Reload recommendations after successful retry
      loadRecommendations();
    } catch (err: any) {
      setError(err.response?.data?.message || 'Failed to execute retry');
    } finally {
      setRetryingRecommendation(null);
    }
  };

  const getSuccessRateColor = (rate: number) => {
    if (rate >= 0.7) return 'success';
    if (rate >= 0.4) return 'warning';
    return 'error';
  };

  const getSuccessRateLabel = (rate: number) => {
    const percentage = Math.round(rate * 100);
    if (percentage >= 70) return `${percentage}% (High)`;
    if (percentage >= 40) return `${percentage}% (Medium)`;
    return `${percentage}% (Low)`;
  };

  if (loading && recommendations.length === 0) {
    return (
      <Card>
        <CardContent>
          <Box display="flex" alignItems="center" gap={1} mb={2}>
            <LightbulbIcon color="primary" />
            <Typography variant="h6">Retry Recommendations</Typography>
          </Box>
          <LinearProgress />
          <Typography variant="body2" color="text.secondary" mt={1}>
            Analyzing failure patterns...
          </Typography>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardContent>
        <Box display="flex" alignItems="center" justifyContent="space-between" mb={2}>
          <Box display="flex" alignItems="center" gap={1}>
            <LightbulbIcon color="primary" />
            <Typography variant="h6">Retry Recommendations</Typography>
            <Tooltip title="AI-powered suggestions based on failure patterns and recent improvements">
              <IconButton size="small">
                <InfoIcon fontSize="small" />
              </IconButton>
            </Tooltip>
          </Box>
          <Button
            startIcon={<RefreshIcon />}
            onClick={loadRecommendations}
            disabled={loading}
            size="small"
          >
            Refresh
          </Button>
        </Box>

        {error && (
          <Alert severity="error" sx={{ mb: 2 }}>
            {error}
          </Alert>
        )}

        {recommendations.length === 0 && !loading ? (
          <Alert severity="info">
            <Typography variant="body2">
              No retry recommendations available. This usually means:
            </Typography>
            <ul style={{ margin: '8px 0', paddingLeft: '20px' }}>
              <li>All failed documents have already been retried multiple times</li>
              <li>No clear patterns in failure reasons that suggest likely success</li>
              <li>No documents with failure types that commonly succeed on retry</li>
            </ul>
          </Alert>
        ) : (
          <Stack spacing={2}>
            {recommendations.map((recommendation, index) => (
              <Card key={recommendation.reason} variant="outlined">
                <CardContent>
                  <Box display="flex" justifyContent="space-between" alignItems="flex-start" mb={1}>
                    <Typography variant="h6" component="div">
                      {recommendation.title}
                    </Typography>
                    <Chip
                      icon={<TrendingUpIcon />}
                      label={getSuccessRateLabel(recommendation.estimated_success_rate)}
                      color={getSuccessRateColor(recommendation.estimated_success_rate) as any}
                      size="small"
                    />
                  </Box>

                  <Typography variant="body2" color="text.secondary" paragraph>
                    {recommendation.description}
                  </Typography>

                  <Box display="flex" alignItems="center" gap={2} mb={2}>
                    <Typography variant="body2">
                      <strong>{recommendation.document_count}</strong> documents
                    </Typography>
                    <Divider orientation="vertical" flexItem />
                    <Typography variant="body2" color="text.secondary">
                      Pattern: {recommendation.reason.replace(/_/g, ' ')}
                    </Typography>
                  </Box>

                  {/* Filter Summary */}
                  <Box mb={2}>
                    <Typography variant="body2" color="text.secondary" gutterBottom>
                      Criteria:
                    </Typography>
                    <Box display="flex" flexWrap="wrap" gap={0.5}>
                      {recommendation.filter.failure_reasons?.map((reason) => (
                        <Chip
                          key={reason}
                          label={reason.replace(/_/g, ' ')}
                          size="small"
                          variant="outlined"
                        />
                      ))}
                      {recommendation.filter.mime_types?.map((type) => (
                        <Chip
                          key={type}
                          label={type.split('/')[1].toUpperCase()}
                          size="small"
                          variant="outlined"
                          color="secondary"
                        />
                      ))}
                      {recommendation.filter.max_file_size && (
                        <Chip
                          label={`< ${Math.round(recommendation.filter.max_file_size / (1024 * 1024))}MB`}
                          size="small"
                          variant="outlined"
                          color="primary"
                        />
                      )}
                    </Box>
                  </Box>

                  <Button
                    variant="contained"
                    color="primary"
                    onClick={() => handleRetryRecommendation(recommendation)}
                    disabled={retryingRecommendation !== null}
                    startIcon={retryingRecommendation === recommendation.reason ? 
                      <LinearProgress sx={{ width: 20, height: 20 }} /> : 
                      <RefreshIcon />
                    }
                    fullWidth
                  >
                    {retryingRecommendation === recommendation.reason
                      ? 'Retrying...'
                      : `Retry ${recommendation.document_count} Documents`
                    }
                  </Button>
                </CardContent>
              </Card>
            ))}
          </Stack>
        )}

        {loading && recommendations.length > 0 && (
          <LinearProgress sx={{ mt: 2 }} />
        )}
      </CardContent>
    </Card>
  );
};