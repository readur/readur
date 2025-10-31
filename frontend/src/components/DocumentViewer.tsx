import React, { useState, useEffect, useRef } from 'react';
import {
  Box,
  Typography,
  CircularProgress,
  Alert,
  Paper,
  IconButton,
  useTheme,
  useMediaQuery,
} from '@mui/material';
import {
  ZoomIn as ZoomInIcon,
  ZoomOut as ZoomOutIcon,
  RestartAlt as ResetIcon,
} from '@mui/icons-material';
import { documentService } from '../services/api';

interface DocumentViewerProps {
  documentId: string;
  filename: string;
  mimeType: string;
}

const DocumentViewer: React.FC<DocumentViewerProps> = ({
  documentId,
  filename,
  mimeType,
}) => {
  const [loading, setLoading] = useState<boolean>(true);
  const [error, setError] = useState<string | null>(null);
  const [documentUrl, setDocumentUrl] = useState<string | null>(null);

  useEffect(() => {
    loadDocument();
    
    // Cleanup URL when component unmounts
    return () => {
      if (documentUrl) {
        window.URL.revokeObjectURL(documentUrl);
      }
    };
  }, [documentId]);

  const loadDocument = async (): Promise<void> => {
    try {
      setLoading(true);
      setError(null);
      
      const response = await documentService.view(documentId);
      const url = window.URL.createObjectURL(new Blob([response.data], { type: mimeType }));
      setDocumentUrl(url);
    } catch (err) {
      console.error('Failed to load document:', err);
      setError('Failed to load document for viewing');
    } finally {
      setLoading(false);
    }
  };

  const renderDocumentContent = (): React.ReactElement => {
    if (!documentUrl) return <></>;

    // Handle images
    if (mimeType.startsWith('image/')) {
      return <ImageViewer documentUrl={documentUrl} filename={filename} />;
    }

    // Handle PDFs
    if (mimeType === 'application/pdf') {
      return (
        <Box sx={{ height: '70vh', width: '100%' }}>
          <iframe
            src={documentUrl}
            width="100%"
            height="100%"
            style={{ border: 'none', borderRadius: '8px' }}
            title={filename}
          />
        </Box>
      );
    }

    // Handle text files
    if (mimeType.startsWith('text/')) {
      return (
        <TextFileViewer documentUrl={documentUrl} filename={filename} />
      );
    }

    // For other file types, show a message
    return (
      <Box sx={{ textAlign: 'center', py: 8 }}>
        <Typography variant="h6" color="text.secondary" sx={{ mb: 2 }}>
          Preview not available
        </Typography>
        <Typography variant="body2" color="text.secondary">
          This file type ({mimeType}) cannot be previewed in the browser.
        </Typography>
        <Typography variant="body2" color="text.secondary" sx={{ mt: 1 }}>
          Please download the file to view its contents.
        </Typography>
      </Box>
    );
  };

  if (loading) {
    return (
      <Box
        sx={{
          display: 'flex',
          flexDirection: 'column',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '60vh',
        }}
      >
        <CircularProgress sx={{ mb: 2 }} />
        <Typography variant="body2" color="text.secondary">
          Loading document...
        </Typography>
      </Box>
    );
  }

  if (error) {
    return (
      <Box sx={{ p: 3 }}>
        <Alert severity="error">{error}</Alert>
      </Box>
    );
  }

  return (
    <Box sx={{ height: '100%', overflow: 'auto' }}>
      {renderDocumentContent()}
    </Box>
  );
};

// Component for viewing images with touch gestures
const ImageViewer: React.FC<{ documentUrl: string; filename: string }> = ({
  documentUrl,
  filename,
}) => {
  const theme = useTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));
  const [scale, setScale] = useState(1);
  const [position, setPosition] = useState({ x: 0, y: 0 });
  const imageContainerRef = useRef<HTMLDivElement>(null);
  const imageRef = useRef<HTMLImageElement>(null);
  const lastTouchDistanceRef = useRef<number | null>(null);
  const lastTapTimeRef = useRef<number>(0);

  // Reset zoom and position
  const handleReset = () => {
    setScale(1);
    setPosition({ x: 0, y: 0 });
  };

  // Zoom in
  const handleZoomIn = () => {
    setScale((prev) => Math.min(prev + 0.5, 5));
  };

  // Zoom out
  const handleZoomOut = () => {
    setScale((prev) => Math.max(prev - 0.5, 0.5));
  };

  // Handle double tap to zoom
  const handleDoubleClick = (e: React.MouseEvent) => {
    const now = Date.now();
    const timeSinceLastTap = now - lastTapTimeRef.current;

    if (timeSinceLastTap < 300) {
      // Double tap detected
      if (scale === 1) {
        setScale(2);
      } else {
        handleReset();
      }
    }
    lastTapTimeRef.current = now;
  };

  // Handle pinch-to-zoom
  const handleTouchStart = (e: React.TouchEvent) => {
    if (e.touches.length === 2) {
      const distance = Math.hypot(
        e.touches[0].clientX - e.touches[1].clientX,
        e.touches[0].clientY - e.touches[1].clientY
      );
      lastTouchDistanceRef.current = distance;
    }
  };

  const handleTouchMove = (e: React.TouchEvent) => {
    if (e.touches.length === 2 && lastTouchDistanceRef.current) {
      e.preventDefault();
      const distance = Math.hypot(
        e.touches[0].clientX - e.touches[1].clientX,
        e.touches[0].clientY - e.touches[1].clientY
      );
      const scaleDelta = distance / lastTouchDistanceRef.current;
      setScale((prev) => Math.max(0.5, Math.min(5, prev * scaleDelta)));
      lastTouchDistanceRef.current = distance;
    }
  };

  const handleTouchEnd = () => {
    lastTouchDistanceRef.current = null;
  };

  // Handle wheel zoom
  const handleWheel = (e: React.WheelEvent) => {
    e.preventDefault();
    const delta = e.deltaY > 0 ? -0.1 : 0.1;
    setScale((prev) => Math.max(0.5, Math.min(5, prev + delta)));
  };

  return (
    <Box sx={{ position: 'relative', width: '100%', minHeight: '60vh' }}>
      {/* Zoom Controls */}
      {isMobile && (
        <Box
          sx={{
            position: 'absolute',
            top: 16,
            right: 16,
            zIndex: 10,
            display: 'flex',
            gap: 1,
            background: 'rgba(0, 0, 0, 0.6)',
            borderRadius: 2,
            padding: '4px',
            backdropFilter: 'blur(10px)',
          }}
        >
          <IconButton
            size="small"
            onClick={handleZoomOut}
            disabled={scale <= 0.5}
            sx={{
              color: 'white',
              '&:disabled': { color: 'rgba(255,255,255,0.3)' },
            }}
          >
            <ZoomOutIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={handleReset}
            sx={{ color: 'white' }}
          >
            <ResetIcon fontSize="small" />
          </IconButton>
          <IconButton
            size="small"
            onClick={handleZoomIn}
            disabled={scale >= 5}
            sx={{
              color: 'white',
              '&:disabled': { color: 'rgba(255,255,255,0.3)' },
            }}
          >
            <ZoomInIcon fontSize="small" />
          </IconButton>
        </Box>
      )}

      {/* Image Container */}
      <Box
        ref={imageContainerRef}
        onClick={handleDoubleClick}
        onTouchStart={handleTouchStart}
        onTouchMove={handleTouchMove}
        onTouchEnd={handleTouchEnd}
        onWheel={handleWheel}
        sx={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '60vh',
          p: 2,
          overflow: 'auto',
          cursor: scale > 1 ? 'move' : 'zoom-in',
          touchAction: 'none',
          userSelect: 'none',
          WebkitUserSelect: 'none',
        }}
      >
        <img
          ref={imageRef}
          src={documentUrl}
          alt={filename}
          draggable={false}
          style={{
            maxWidth: scale === 1 ? '100%' : 'none',
            maxHeight: scale === 1 ? '100%' : 'none',
            objectFit: 'contain',
            borderRadius: '8px',
            boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
            transform: `scale(${scale}) translate(${position.x}px, ${position.y}px)`,
            transition: scale === 1 ? 'transform 0.3s ease-out' : 'none',
            transformOrigin: 'center center',
          }}
        />
      </Box>

      {/* Zoom Indicator */}
      {isMobile && scale !== 1 && (
        <Box
          sx={{
            position: 'absolute',
            bottom: 16,
            left: '50%',
            transform: 'translateX(-50%)',
            background: 'rgba(0, 0, 0, 0.6)',
            color: 'white',
            padding: '6px 16px',
            borderRadius: 2,
            fontSize: '0.875rem',
            fontWeight: 500,
            backdropFilter: 'blur(10px)',
          }}
        >
          {Math.round(scale * 100)}%
        </Box>
      )}
    </Box>
  );
};

// Component for viewing text files
const TextFileViewer: React.FC<{ documentUrl: string; filename: string }> = ({
  documentUrl,
  filename,
}) => {
  const [textContent, setTextContent] = useState<string>('');
  const [loading, setLoading] = useState<boolean>(true);

  useEffect(() => {
    loadTextContent();
  }, [documentUrl]);

  const loadTextContent = async (): Promise<void> => {
    try {
      const response = await fetch(documentUrl);
      const text = await response.text();
      setTextContent(text);
    } catch (err) {
      console.error('Failed to load text content:', err);
      setTextContent('Failed to load text content');
    } finally {
      setLoading(false);
    }
  };

  if (loading) {
    return (
      <Box sx={{ display: 'flex', justifyContent: 'center', p: 3 }}>
        <CircularProgress size={24} />
      </Box>
    );
  }

  return (
    <Paper
      sx={{
        p: 3,
        m: 2,
        backgroundColor: (theme) => theme.palette.mode === 'light' ? 'grey.50' : 'grey.900',
        border: '1px solid',
        borderColor: 'divider',
        borderRadius: 2,
        maxHeight: '70vh',
        overflow: 'auto',
      }}
    >
      <Typography
        variant="body2"
        sx={{
          fontFamily: 'monospace',
          whiteSpace: 'pre-wrap',
          lineHeight: 1.6,
          color: 'text.primary',
        }}
      >
        {textContent}
      </Typography>
    </Paper>
  );
};

export default DocumentViewer;