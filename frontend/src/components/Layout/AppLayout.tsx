import React, { useState } from 'react';
import {
  AppBar,
  Box,
  CssBaseline,
  Drawer,
  IconButton,
  Toolbar,
  Menu,
  MenuItem,
  Divider,
  useTheme as useMuiTheme,
  useMediaQuery,
  Badge,
  Avatar,
} from '@mui/material';
import {
  Menu as MenuIcon,
  Dashboard as DashboardIcon,
  CloudUpload as UploadIcon,
  Search as SearchIcon,
  Folder as FolderIcon,
  Settings as SettingsIcon,
  Notifications as NotificationsIcon,
  AccountCircle as AccountIcon,
  Logout as LogoutIcon,
  Description as DocumentIcon,
  Storage as StorageIcon,
  Error as ErrorIcon,
  Label as LabelIcon,
  Block as BlockIcon,
  Api as ApiIcon,
  ManageAccounts as ManageIcon,
  BugReport as BugReportIcon,
} from '../../design/icons';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import { useNotifications } from '../../contexts/NotificationContext';
import GlobalSearchBar from '../GlobalSearchBar';
import ThemeToggle from '../ThemeToggle/ThemeToggle';
import NotificationPanel from '../Notifications/NotificationPanel';
import LanguageSwitcher from '../LanguageSwitcher';
import BottomNavigation from './BottomNavigation';
import { usePWA } from '../../hooks/usePWA';
import { useTranslation } from 'react-i18next';

const drawerWidth = 248;

interface NavigationItem {
  textKey: string;
  icon: React.ComponentType<any>;
  path: string;
}

interface NavigationSection {
  labelKey: string;
  items: NavigationItem[];
}

const getNavigationSections = (): NavigationSection[] => [
  {
    labelKey: 'navigation.sections.library',
    items: [
      { textKey: 'navigation.dashboard', icon: DashboardIcon, path: '/dashboard' },
      { textKey: 'navigation.documents', icon: DocumentIcon, path: '/documents' },
      { textKey: 'navigation.search', icon: SearchIcon, path: '/search' },
      { textKey: 'navigation.labels', icon: LabelIcon, path: '/labels' },
    ],
  },
  {
    labelKey: 'navigation.sections.ingest',
    items: [
      { textKey: 'navigation.upload', icon: UploadIcon, path: '/upload' },
      { textKey: 'navigation.sources', icon: StorageIcon, path: '/sources' },
      { textKey: 'navigation.watchFolder', icon: FolderIcon, path: '/watch' },
      { textKey: 'navigation.ignoredFiles', icon: BlockIcon, path: '/ignored-files' },
    ],
  },
  {
    labelKey: 'navigation.sections.system',
    items: [
      { textKey: 'navigation.documentManagement', icon: ManageIcon, path: '/documents/management' },
      { textKey: 'navigation.settings', icon: SettingsIcon, path: '/settings' },
      { textKey: 'navigation.debug', icon: BugReportIcon, path: '/debug' },
    ],
  },
];

interface AppLayoutProps {
  children: React.ReactNode;
}

const AppLayout: React.FC<AppLayoutProps> = ({ children }) => {
  const theme = useMuiTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));
  const isPWA = usePWA();
  const [mobileOpen, setMobileOpen] = useState<boolean>(false);
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const [notificationAnchorEl, setNotificationAnchorEl] = useState<null | HTMLElement>(null);
  const navigate = useNavigate();
  const location = useLocation();
  const { user, logout } = useAuth();
  const { unreadCount } = useNotifications();
  const { t } = useTranslation();

  const sections = getNavigationSections();
  const allItems = sections.flatMap((s) => s.items);
  const activeItem = allItems.find((item) => item.path === location.pathname);

  const handleDrawerToggle = (): void => setMobileOpen(!mobileOpen);
  const handleProfileMenuOpen = (e: React.MouseEvent<HTMLElement>): void => setAnchorEl(e.currentTarget);
  const handleProfileMenuClose = (): void => setAnchorEl(null);
  const handleLogout = (): void => {
    logout();
    handleProfileMenuClose();
    navigate('/login');
  };
  const handleNotificationClick = (e: React.MouseEvent<HTMLElement>): void => {
    setNotificationAnchorEl(notificationAnchorEl ? null : e.currentTarget);
  };
  const handleNotificationClose = (): void => setNotificationAnchorEl(null);

  const drawer = (
    <Box
      sx={{
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--bg-1)',
        borderRight: '1px solid var(--line-1)',
      }}
    >
      {/* Brand block */}
      <Box
        sx={{
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--s-3)',
          padding: '20px 22px 18px',
          borderBottom: '1px solid var(--line-1)',
        }}
      >
        <Box
          sx={{
            width: 36,
            height: 36,
            borderRadius: 'var(--r-2)',
            background: 'var(--accent-grad)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            boxShadow: 'var(--shadow-sm)',
            overflow: 'hidden',
            flexShrink: 0,
          }}
        >
          <Box
            component="img"
            src="/readur-32.png"
            srcSet="/readur-32.png 1x, /readur-64.png 2x"
            alt={t('common.appName')}
            sx={{ width: 24, height: 24, objectFit: 'contain' }}
            onError={(e: React.SyntheticEvent<HTMLImageElement>) => {
              e.currentTarget.style.display = 'none';
              const parent = e.currentTarget.parentElement;
              if (parent) {
                parent.innerHTML = '<span style="color:#fff;font-weight:800;font-size:16px;letter-spacing:-.04em;">R</span>';
              }
            }}
          />
        </Box>
        <Box sx={{ minWidth: 0 }}>
          <Box
            sx={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 800,
              fontSize: 18,
              lineHeight: 1,
              color: 'var(--fg-0)',
              letterSpacing: '-0.025em',
            }}
          >
            {t('common.appName')}
          </Box>
          <Box
            sx={{
              fontFamily: 'var(--font-mono)',
              fontWeight: 600,
              fontSize: 9,
              color: 'var(--accent-60)',
              letterSpacing: '0.1em',
              marginTop: '4px',
              textTransform: 'uppercase',
            }}
          >
            v2.9
          </Box>
        </Box>
      </Box>

      {/* Navigation sections */}
      <Box
        sx={{
          flex: 1,
          overflowY: 'auto',
          padding: '14px 12px',
          display: 'flex',
          flexDirection: 'column',
          gap: 'var(--s-1)',
        }}
      >
        {sections.map((section) => (
          <React.Fragment key={section.labelKey}>
            <Box
              className="rd-label"
              sx={{ padding: '12px 12px 6px' }}
            >
              {t(section.labelKey, section.labelKey.split('.').pop())}
            </Box>
            {section.items.map((item) => {
              const Icon = item.icon;
              const isActive = location.pathname === item.path;
              return (
                <Box
                  key={item.textKey}
                  onClick={() => {
                    navigate(item.path);
                    if (isMobile) setMobileOpen(false);
                  }}
                  sx={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 'var(--s-3)',
                    padding: '9px 12px',
                    fontFamily: 'var(--font-sans)',
                    fontSize: 13.5,
                    fontWeight: isActive ? 600 : 500,
                    color: isActive ? 'var(--accent-70)' : 'var(--fg-1)',
                    background: isActive ? 'var(--accent-05)' : 'transparent',
                    borderRadius: 'var(--r-2)',
                    cursor: 'pointer',
                    lineHeight: 1,
                    transition: 'background var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out)',
                    '&:hover': {
                      background: isActive ? 'var(--accent-05)' : 'var(--bg-2)',
                      color: isActive ? 'var(--accent-70)' : 'var(--fg-0)',
                    },
                    '& svg': {
                      color: isActive ? 'var(--accent-60)' : 'var(--fg-2)',
                      fontSize: 18,
                    },
                  }}
                >
                  <Icon />
                  <Box sx={{ flex: 1, minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>
                    {t(item.textKey)}
                  </Box>
                </Box>
              );
            })}
          </React.Fragment>
        ))}
      </Box>

      {/* User footer */}
      <Box
        sx={{
          padding: '14px 16px',
          borderTop: '1px solid var(--line-1)',
          display: 'flex',
          alignItems: 'center',
          gap: 'var(--s-3)',
        }}
      >
        <Avatar
          sx={{
            width: 28,
            height: 28,
            background: 'var(--accent-grad)',
            fontFamily: 'var(--font-sans)',
            fontWeight: 700,
            fontSize: 12,
          }}
        >
          {user?.username?.charAt(0).toUpperCase()}
        </Avatar>
        <Box sx={{ minWidth: 0, flex: 1 }}>
          <Box
            sx={{
              fontFamily: 'var(--font-sans)',
              fontWeight: 600,
              fontSize: 12,
              color: 'var(--fg-0)',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {user?.username}
          </Box>
          <Box
            sx={{
              fontFamily: 'var(--font-mono)',
              fontSize: 10,
              color: 'var(--fg-3)',
              marginTop: '2px',
              overflow: 'hidden',
              textOverflow: 'ellipsis',
              whiteSpace: 'nowrap',
            }}
          >
            {user?.email}
          </Box>
        </Box>
      </Box>
    </Box>
  );

  // Icon button shared style — replaces the old gradient/glass chips
  const iconBtnSx = {
    width: 34,
    height: 34,
    border: '1px solid transparent',
    borderRadius: 'var(--r-2)',
    color: 'var(--fg-2)',
    background: 'transparent',
    transition: 'background var(--dur-fast) var(--ease-out), color var(--dur-fast) var(--ease-out)',
    '&:hover': {
      background: 'var(--bg-2)',
      color: 'var(--fg-0)',
    },
  } as const;

  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />

      {/* Topbar */}
      <AppBar
        position="fixed"
        elevation={0}
        sx={{
          width: { md: `calc(100% - ${drawerWidth}px)` },
          ml: { md: `${drawerWidth}px` },
          background: 'var(--bg-1)',
          color: 'var(--fg-0)',
          borderBottom: '1px solid var(--line-1)',
          backgroundImage: 'none',
          boxShadow: 'none',
        }}
      >
        <Toolbar sx={{ minHeight: '56px !important', gap: 'var(--s-3)' }}>
          <IconButton
            edge="start"
            onClick={handleDrawerToggle}
            sx={{ ...iconBtnSx, display: { md: 'none' } }}
            aria-label="open navigation"
          >
            <MenuIcon />
          </IconButton>

          {/* Breadcrumb */}
          {!isPWA && (
            <Box
              sx={{
                fontFamily: 'var(--font-sans)',
                fontWeight: 600,
                fontSize: 11,
                color: 'var(--fg-3)',
                letterSpacing: 'var(--tracking-caps)',
                textTransform: 'uppercase',
                display: { xs: 'none', sm: 'flex' },
                alignItems: 'center',
                gap: 'var(--s-2)',
                flexShrink: 0,
              }}
            >
              <span>{t('common.appName')}</span>
              <span style={{ color: 'var(--fg-4)' }}>/</span>
              <span style={{ color: 'var(--fg-0)' }}>
                {activeItem ? t(activeItem.textKey) : t('navigation.dashboard')}
              </span>
            </Box>
          )}

          {/* Global search */}
          <Box
            sx={{
              flex: '1 1 auto',
              display: 'flex',
              justifyContent: 'center',
              minWidth: 0,
              maxWidth: 520,
              marginLeft: { xs: 0, sm: 'var(--s-6)' },
            }}
          >
            <GlobalSearchBar />
          </Box>

          {/* Right-side actions */}
          <Box sx={{ marginLeft: 'auto', display: 'flex', alignItems: 'center', gap: 'var(--s-1)' }}>
            <IconButton
              onClick={handleNotificationClick}
              sx={{ ...iconBtnSx, display: isPWA ? 'none' : 'inline-flex' }}
              aria-label="notifications"
            >
              <Badge
                badgeContent={unreadCount}
                color="error"
                sx={{
                  '& .MuiBadge-badge': {
                    fontWeight: 600,
                    fontSize: '0.7rem',
                  },
                }}
              >
                <NotificationsIcon sx={{ fontSize: 18 }} />
              </Badge>
            </IconButton>

            <Box sx={{ display: isPWA ? 'none' : { xs: 'none', sm: 'flex' } }}>
              <LanguageSwitcher size="small" color="inherit" />
            </Box>

            <Box sx={{ display: 'flex' }}>
              <ThemeToggle size="small" color="inherit" />
            </Box>

            <IconButton onClick={handleProfileMenuOpen} sx={iconBtnSx} aria-label="profile menu">
              <AccountIcon sx={{ fontSize: 18 }} />
            </IconButton>
          </Box>

          <Menu
            anchorEl={anchorEl}
            open={Boolean(anchorEl)}
            onClose={handleProfileMenuClose}
            onClick={handleProfileMenuClose}
            transformOrigin={{ horizontal: 'right', vertical: 'top' }}
            anchorOrigin={{ horizontal: 'right', vertical: 'bottom' }}
            slotProps={{
              paper: {
                sx: { mt: 1, minWidth: 200 },
              },
            }}
          >
            <MenuItem onClick={() => navigate('/profile')}>
              <AccountIcon fontSize="small" sx={{ mr: 2, color: 'var(--fg-2)' }} />
              {t('auth.profile')}
            </MenuItem>
            <MenuItem onClick={() => navigate('/settings')}>
              <SettingsIcon fontSize="small" sx={{ mr: 2, color: 'var(--fg-2)' }} />
              {t('settings.title')}
            </MenuItem>
            <MenuItem onClick={() => navigate('/debug')}>
              <BugReportIcon fontSize="small" sx={{ mr: 2, color: 'var(--fg-2)' }} />
              {t('settings.debug')}
            </MenuItem>
            <Divider />
            <MenuItem onClick={() => window.open('/swagger-ui', '_blank')}>
              <ApiIcon fontSize="small" sx={{ mr: 2, color: 'var(--fg-2)' }} />
              {t('settings.apiDocumentation')}
            </MenuItem>
            <Divider />
            <MenuItem onClick={handleLogout}>
              <LogoutIcon fontSize="small" sx={{ mr: 2, color: 'var(--fg-2)' }} />
              {t('auth.logout')}
            </MenuItem>
          </Menu>
        </Toolbar>
      </AppBar>

      {/* Navigation Drawer */}
      <Box
        component="nav"
        sx={{ width: { md: drawerWidth }, flexShrink: { md: 0 } }}
      >
        <Drawer
          variant="temporary"
          open={mobileOpen}
          onClose={handleDrawerToggle}
          ModalProps={{ keepMounted: true }}
          sx={{
            display: { xs: 'block', md: 'none' },
            '& .MuiDrawer-paper': { boxSizing: 'border-box', width: drawerWidth },
          }}
        >
          {drawer}
        </Drawer>
        <Drawer
          variant="permanent"
          sx={{
            display: { xs: 'none', md: 'block' },
            '& .MuiDrawer-paper': { boxSizing: 'border-box', width: drawerWidth },
          }}
          open
        >
          {drawer}
        </Drawer>
      </Box>

      {/* Main content */}
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          width: { md: `calc(100% - ${drawerWidth}px)` },
          minHeight: '100vh',
          background: 'var(--bg-0)',
        }}
      >
        <Toolbar sx={{ minHeight: '56px !important' }} />
        <Box
          sx={{
            padding: 'var(--s-6) var(--s-8)',
            paddingBottom: isPWA && isMobile
              ? 'calc(64px + var(--s-6) + 8px + env(safe-area-inset-bottom, 0px))'
              : 'var(--s-12)',
          }}
        >
          {children}
        </Box>
      </Box>

      {/* Notification panel */}
      <NotificationPanel
        anchorEl={notificationAnchorEl}
        onClose={handleNotificationClose}
      />

      {/* Bottom navigation (PWA only) */}
      <BottomNavigation />
    </Box>
  );
};

export default AppLayout;
