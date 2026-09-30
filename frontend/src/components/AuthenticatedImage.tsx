import React, { useEffect, useState } from 'react';
import { Box, CircularProgress, Typography } from '@mui/material';

interface AuthenticatedImageProps {
  /** Fetches the image through the authenticated API client (responseType: 'blob'). */
  load: () => Promise<{ data: Blob }>;
  alt: string;
  unavailableText?: string;
  maxHeight?: number | string;
}

/**
 * Displays an image served by an authenticated API endpoint. A plain
 * `<img src="/api/...">` would not send the bearer token.
 */
const AuthenticatedImage: React.FC<AuthenticatedImageProps> = ({
  load,
  alt,
  unavailableText = 'Image not available',
  maxHeight = '300px',
}) => {
  const [url, setUrl] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);

  useEffect(() => {
    let objectUrl: string | null = null;
    let cancelled = false;

    load()
      .then((response) => {
        if (cancelled) return;
        objectUrl = window.URL.createObjectURL(response.data);
        setUrl(objectUrl);
      })
      .catch(() => {
        if (!cancelled) setFailed(true);
      });

    return () => {
      cancelled = true;
      if (objectUrl) window.URL.revokeObjectURL(objectUrl);
    };
    // `load` is expected to be an inline closure; reload only on mount.
  }, []);

  if (failed) {
    return (
      <Typography variant="body2" color="text.secondary">
        {unavailableText}
      </Typography>
    );
  }

  if (!url) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', p: 2 }}>
        <CircularProgress size={24} />
      </Box>
    );
  }

  return (
    <Box
      component="img"
      src={url}
      alt={alt}
      onError={() => setFailed(true)}
      sx={{
        maxWidth: '100%',
        maxHeight,
        objectFit: 'contain',
        border: '1px solid',
        borderColor: 'divider',
        borderRadius: 1,
      }}
    />
  );
};

export default AuthenticatedImage;
