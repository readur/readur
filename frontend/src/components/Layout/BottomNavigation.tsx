import React from 'react';
import {
  BottomNavigation as MuiBottomNavigation,
  BottomNavigationAction,
  Paper,
} from '@mui/material';
import {
  Dashboard as DashboardIcon,
  CloudUpload as UploadIcon,
  Label as LabelIcon,
  Settings as SettingsIcon,
} from '../../design/icons';
import { useNavigate, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { usePWA } from '../../hooks/usePWA';

const BottomNavigation: React.FC = () => {
  const navigate = useNavigate();
  const location = useLocation();
  const { t } = useTranslation();
  const isPWA = usePWA();

  if (!isPWA) return null;

  const getNavValue = (pathname: string): string => {
    if (pathname === '/dashboard') return 'dashboard';
    if (pathname === '/upload') return 'upload';
    if (pathname === '/labels') return 'labels';
    if (pathname === '/settings' || pathname === '/profile') return 'settings';
    return 'dashboard';
  };

  const handleNavigation = (_event: React.SyntheticEvent, newValue: string) => {
    const map: Record<string, string> = {
      dashboard: '/dashboard',
      upload: '/upload',
      labels: '/labels',
      settings: '/settings',
    };
    const target = map[newValue];
    if (target) navigate(target);
  };

  return (
    <Paper
      elevation={0}
      sx={{
        position: 'fixed',
        bottom: 0,
        left: 0,
        right: 0,
        zIndex: 1100,
        display: { xs: 'block', md: 'none' },
        background: 'var(--bg-1)',
        borderTop: '1px solid var(--line-1)',
        borderRadius: 0,
        boxShadow: 'none',
        paddingBottom: 'calc(8px + env(safe-area-inset-bottom, 0px))',
      }}
    >
      <MuiBottomNavigation
        value={getNavValue(location.pathname)}
        onChange={handleNavigation}
        sx={{
          background: 'transparent',
          height: 64,
          '& .MuiBottomNavigationAction-root': {
            color: 'var(--fg-3)',
            minWidth: 'auto',
            padding: '8px 12px',
            transition: 'color var(--dur-fast) var(--ease-out)',
            '& .MuiBottomNavigationAction-label': {
              fontFamily: 'var(--font-sans)',
              fontSize: '0.7rem',
              fontWeight: 500,
              letterSpacing: '0.025em',
              marginTop: 4,
            },
            '& .MuiSvgIcon-root': {
              fontSize: '1.25rem',
            },
            '&.Mui-selected': {
              color: 'var(--accent-70)',
              '& .MuiBottomNavigationAction-label': {
                fontWeight: 600,
                color: 'var(--accent-70)',
                fontSize: '0.7rem',
              },
            },
            '@media (pointer: coarse)': {
              minHeight: 56,
              '&:active': {
                background: 'var(--bg-2)',
              },
            },
          },
        }}
      >
        <BottomNavigationAction
          label={t('navigation.dashboard')}
          value="dashboard"
          icon={<DashboardIcon />}
        />
        <BottomNavigationAction
          label={t('navigation.upload')}
          value="upload"
          icon={<UploadIcon />}
        />
        <BottomNavigationAction
          label={t('navigation.labels')}
          value="labels"
          icon={<LabelIcon />}
        />
        <BottomNavigationAction
          label={t('settings.title')}
          value="settings"
          icon={<SettingsIcon />}
        />
      </MuiBottomNavigation>
    </Paper>
  );
};

export default BottomNavigation;
