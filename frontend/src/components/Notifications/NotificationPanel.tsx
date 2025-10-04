import React from 'react';
import {
  Box,
  Paper,
  Typography,
  IconButton,
  List,
  ListItem,
  ListItemText,
  ListItemIcon,
  Divider,
  Button,
  Chip,
  Stack,
  useTheme,
} from '@mui/material';
import {
  CheckCircle as SuccessIcon,
  Error as ErrorIcon,
  Info as InfoIcon,
  Warning as WarningIcon,
  Close as CloseIcon,
  Delete as DeleteIcon,
  DoneAll as DoneAllIcon,
} from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { useNotifications } from '../../contexts/NotificationContext';
import { NotificationType } from '../../types/notification';
import { formatDistanceToNow } from 'date-fns';

interface NotificationPanelProps {
  anchorEl: HTMLElement | null;
  onClose: () => void;
}

const NotificationPanel: React.FC<NotificationPanelProps> = ({ anchorEl, onClose }) => {
  const theme = useTheme();
  const { t } = useTranslation();
  const { notifications, unreadCount, markAsRead, markAllAsRead, clearNotification, clearAll } = useNotifications();

  const getIcon = (type: NotificationType) => {
    switch (type) {
      case 'success':
        return <SuccessIcon sx={{ color: theme.palette.success.main }} />;
      case 'error':
        return <ErrorIcon sx={{ color: theme.palette.error.main }} />;
      case 'warning':
        return <WarningIcon sx={{ color: theme.palette.warning.main }} />;
      case 'info':
      default:
        return <InfoIcon sx={{ color: theme.palette.info.main }} />;
    }
  };

  if (!anchorEl) return null;

  const rect = anchorEl.getBoundingClientRect();

  return (
    <Paper
      elevation={8}
      sx={{
        position: 'fixed',
        top: rect.bottom + 8,
        right: 16,
        width: 400,
        maxHeight: '70vh',
        display: 'flex',
        flexDirection: 'column',
        borderRadius: 3,
        overflow: 'hidden',
        background: theme.palette.mode === 'light'
          ? 'linear-gradient(135deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.95) 100%)'
          : 'linear-gradient(135deg, rgba(30,30,30,0.95) 0%, rgba(20,20,20,0.95) 100%)',
        backdropFilter: 'blur(20px)',
        border: theme.palette.mode === 'light'
          ? '1px solid rgba(0,0,0,0.05)'
          : '1px solid rgba(255,255,255,0.05)',
      }}
    >
      {/* Header */}
      <Box
        sx={{
          p: 2,
          borderBottom: `1px solid ${theme.palette.divider}`,
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
        }}
      >
        <Stack direction="row" spacing={2} alignItems="center">
          <Typography variant="h6" fontWeight={600}>
            {t('notifications.title')}
          </Typography>
          {unreadCount > 0 && (
            <Chip
              label={unreadCount}
              size="small"
              sx={{
                background: 'linear-gradient(135deg, #ef4444 0%, #f97316 100%)',
                color: 'white',
                fontWeight: 600,
              }}
            />
          )}
        </Stack>
        <Stack direction="row" spacing={0.5}>
          {notifications.length > 0 && (
            <>
              <IconButton
                size="small"
                onClick={markAllAsRead}
                title={t('notifications.markAllAsRead')}
                sx={{ opacity: 0.7, '&:hover': { opacity: 1 } }}
              >
                <DoneAllIcon fontSize="small" />
              </IconButton>
              <IconButton
                size="small"
                onClick={clearAll}
                title={t('notifications.clearAll')}
                sx={{ opacity: 0.7, '&:hover': { opacity: 1 } }}
              >
                <DeleteIcon fontSize="small" />
              </IconButton>
            </>
          )}
          <IconButton size="small" onClick={onClose}>
            <CloseIcon fontSize="small" />
          </IconButton>
        </Stack>
      </Box>

      {/* Notifications List */}
      <Box sx={{ flex: 1, overflow: 'auto' }}>
        {notifications.length === 0 ? (
          <Box
            sx={{
              p: 4,
              textAlign: 'center',
              color: 'text.secondary',
            }}
          >
            <Typography variant="body2">{t('notifications.noNotifications')}</Typography>
          </Box>
        ) : (
          <List sx={{ p: 0 }}>
            {notifications.map((notification, index) => (
              <React.Fragment key={notification.id}>
                <ListItem
                  sx={{
                    py: 1.5,
                    px: 2,
                    background: !notification.read
                      ? theme.palette.mode === 'light'
                        ? 'rgba(99,102,241,0.05)'
                        : 'rgba(99,102,241,0.1)'
                      : 'transparent',
                    '&:hover': {
                      background: theme.palette.mode === 'light'
                        ? 'rgba(0,0,0,0.02)'
                        : 'rgba(255,255,255,0.02)',
                    },
                    cursor: 'pointer',
                  }}
                  onClick={() => markAsRead(notification.id)}
                  secondaryAction={
                    <IconButton
                      edge="end"
                      size="small"
                      onClick={(e) => {
                        e.stopPropagation();
                        clearNotification(notification.id);
                      }}
                      sx={{ opacity: 0.5, '&:hover': { opacity: 1 } }}
                    >
                      <CloseIcon fontSize="small" />
                    </IconButton>
                  }
                >
                  <ListItemIcon sx={{ minWidth: 40 }}>
                    {getIcon(notification.type)}
                  </ListItemIcon>
                  <ListItemText
                    primary={
                      <Typography variant="body2" fontWeight={!notification.read ? 600 : 400}>
                        {notification.title}
                      </Typography>
                    }
                    secondary={
                      <Box>
                        <Typography variant="body2" color="text.secondary" sx={{ mb: 0.5 }}>
                          {notification.message}
                        </Typography>
                        <Typography variant="caption" color="text.disabled">
                          {formatDistanceToNow(notification.timestamp, { addSuffix: true })}
                        </Typography>
                      </Box>
                    }
                  />
                </ListItem>
                {index < notifications.length - 1 && <Divider component="li" />}
              </React.Fragment>
            ))}
          </List>
        )}
      </Box>
    </Paper>
  );
};

export default NotificationPanel;