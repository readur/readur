import React from 'react';
import {
  BottomNavigation as MuiBottomNavigation,
  BottomNavigationAction,
  Paper,
  useTheme,
} from '@mui/material';
import {
  Dashboard as DashboardIcon,
  CloudUpload as UploadIcon,
  Search as SearchIcon,
  Settings as SettingsIcon,
} from '@mui/icons-material';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';

const BottomNavigation: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const theme = useTheme();
  const { t } = useTranslation();

  // Map paths to nav values
  const getNavValue = (pathname: string): string => {
    if (pathname === '/dashboard') return 'dashboard';
    if (pathname === '/upload') return 'upload';
    if (pathname === '/search' || pathname === '/documents') return 'search';
    if (pathname === '/settings' || pathname === '/profile') return 'settings';
    return 'dashboard';
  };

  const handleNavigation = (_event: React.SyntheticEvent, newValue: string) => {
    switch (newValue) {
      case 'dashboard':
        navigate('/dashboard');
        break;
      case 'upload':
        navigate('/upload');
        break;
      case 'search':
        navigate('/documents');
        break;
      case 'settings':
        navigate('/settings');
        break;
    }
  };

  return (
    <Paper
      sx={{
        position: 'fixed',
        bottom: 0,
        left: 0,
        right: 0,
        zIndex: 1100,
        display: { xs: 'block', md: 'none' },
        background: theme.palette.mode === 'light'
          ? 'linear-gradient(180deg, rgba(255,255,255,0.98) 0%, rgba(248,250,252,0.98) 100%)'
          : 'linear-gradient(180deg, rgba(30,30,30,0.98) 0%, rgba(18,18,18,0.98) 100%)',
        backdropFilter: 'blur(20px)',
        borderTop: theme.palette.mode === 'light'
          ? '1px solid rgba(226,232,240,0.5)'
          : '1px solid rgba(255,255,255,0.1)',
        boxShadow: theme.palette.mode === 'light'
          ? '0 -4px 32px rgba(0,0,0,0.08)'
          : '0 -4px 32px rgba(0,0,0,0.3)',
        // iOS safe area support
        paddingBottom: 'env(safe-area-inset-bottom, 0px)',
      }}
      elevation={0}
    >
      <MuiBottomNavigation
        value={getNavValue(location.pathname)}
        onChange={handleNavigation}
        sx={{
          background: 'transparent',
          height: '64px',
          '& .MuiBottomNavigationAction-root': {
            color: 'text.secondary',
            minWidth: 'auto',
            padding: '8px 12px',
            gap: '4px',
            transition: 'all 0.2s ease-in-out',
            '& .MuiBottomNavigationAction-label': {
              fontSize: '0.75rem',
              fontWeight: 500,
              letterSpacing: '0.025em',
              marginTop: '4px',
              transition: 'all 0.2s ease-in-out',
              '&.Mui-selected': {
                fontSize: '0.75rem',
              },
            },
            '& .MuiSvgIcon-root': {
              fontSize: '1.5rem',
              transition: 'all 0.3s cubic-bezier(0.4, 0, 0.2, 1)',
            },
            '&.Mui-selected': {
              color: '#6366f1',
              '& .MuiSvgIcon-root': {
                transform: 'scale(1.1)',
                filter: 'drop-shadow(0 2px 8px rgba(99,102,241,0.3))',
              },
            },
            // iOS-style touch feedback
            '@media (pointer: coarse)': {
              minHeight: '56px',
              '&:active': {
                transform: 'scale(0.95)',
              },
            },
          },
        }}
      >
        <BottomNavigationAction
          label={t('navigation.dashboard')}
          value="dashboard"
          icon={<DashboardIcon />}
          sx={{
            '&.Mui-selected': {
              '& .MuiBottomNavigationAction-label': {
                background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                backgroundClip: 'text',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
                fontWeight: 600,
              },
            },
          }}
        />
        <BottomNavigationAction
          label={t('navigation.upload')}
          value="upload"
          icon={<UploadIcon />}
          sx={{
            '&.Mui-selected': {
              '& .MuiBottomNavigationAction-label': {
                background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                backgroundClip: 'text',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
                fontWeight: 600,
              },
            },
          }}
        />
        <BottomNavigationAction
          label={t('navigation.documents')}
          value="search"
          icon={<SearchIcon />}
          sx={{
            '&.Mui-selected': {
              '& .MuiBottomNavigationAction-label': {
                background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                backgroundClip: 'text',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
                fontWeight: 600,
              },
            },
          }}
        />
        <BottomNavigationAction
          label={t('settings.title')}
          value="settings"
          icon={<SettingsIcon />}
          sx={{
            '&.Mui-selected': {
              '& .MuiBottomNavigationAction-label': {
                background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
                backgroundClip: 'text',
                WebkitBackgroundClip: 'text',
                WebkitTextFillColor: 'transparent',
                fontWeight: 600,
              },
            },
          }}
        />
      </MuiBottomNavigation>
    </Paper>
  );
};

export default BottomNavigation;
