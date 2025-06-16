import React, { useState } from 'react';
import {
  AppBar,
  Box,
  CssBaseline,
  Drawer,
  IconButton,
  List,
  ListItem,
  ListItemButton,
  ListItemIcon,
  ListItemText,
  Toolbar,
  Typography,
  Avatar,
  Menu,
  MenuItem,
  Divider,
  useTheme as useMuiTheme,
  useMediaQuery,
  Badge,
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
} from '@mui/icons-material';
import { useNavigate, useLocation } from 'react-router-dom';
import { useAuth } from '../../contexts/AuthContext';
import { useNotifications } from '../../contexts/NotificationContext';
import GlobalSearchBar from '../GlobalSearchBar';
import ThemeToggle from '../ThemeToggle/ThemeToggle';
import NotificationPanel from '../Notifications/NotificationPanel';

const drawerWidth = 280;

interface NavigationItem {
  text: string;
  icon: React.ComponentType<any>;
  path: string;
}

interface AppLayoutProps {
  children: React.ReactNode;
}

interface User {
  username?: string;
  email?: string;
}

const navigationItems: NavigationItem[] = [
  { text: 'Dashboard', icon: DashboardIcon, path: '/dashboard' },
  { text: 'Upload', icon: UploadIcon, path: '/upload' },
  { text: 'Documents', icon: DocumentIcon, path: '/documents' },
  { text: 'Search', icon: SearchIcon, path: '/search' },
  { text: 'Sources', icon: StorageIcon, path: '/sources' },
  { text: 'Watch Folder', icon: FolderIcon, path: '/watch' },
];

const AppLayout: React.FC<AppLayoutProps> = ({ children }) => {
  const theme = useMuiTheme();
  const isMobile = useMediaQuery(theme.breakpoints.down('md'));
  const [mobileOpen, setMobileOpen] = useState<boolean>(false);
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const [notificationAnchorEl, setNotificationAnchorEl] = useState<null | HTMLElement>(null);
  const navigate = useNavigate();
  const location = useLocation();
  const { user, logout } = useAuth();
  const { unreadCount } = useNotifications();

  const handleDrawerToggle = (): void => {
    setMobileOpen(!mobileOpen);
  };

  const handleProfileMenuOpen = (event: React.MouseEvent<HTMLElement>): void => {
    setAnchorEl(event.currentTarget);
  };

  const handleProfileMenuClose = (): void => {
    setAnchorEl(null);
  };

  const handleLogout = (): void => {
    logout();
    handleProfileMenuClose();
    navigate('/login');
  };

  const handleNotificationClick = (event: React.MouseEvent<HTMLElement>): void => {
    setNotificationAnchorEl(notificationAnchorEl ? null : event.currentTarget);
  };

  const handleNotificationClose = (): void => {
    setNotificationAnchorEl(null);
  };

  const drawer = (
    <Box sx={{ 
      height: '100%', 
      display: 'flex', 
      flexDirection: 'column',
      background: theme.palette.mode === 'light' 
        ? 'linear-gradient(180deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.95) 100%)'
        : 'linear-gradient(180deg, rgba(30,30,30,0.95) 0%, rgba(18,18,18,0.95) 100%)',
      backdropFilter: 'blur(20px)',
      borderRight: theme.palette.mode === 'light' 
        ? '1px solid rgba(226,232,240,0.5)'
        : '1px solid rgba(255,255,255,0.1)',
    }}>
      {/* Logo Section */}
      <Box sx={{ 
        p: 3, 
        borderBottom: theme.palette.mode === 'light' 
          ? '1px solid rgba(226,232,240,0.3)'
          : '1px solid rgba(255,255,255,0.1)',
        background: theme.palette.mode === 'light'
          ? 'linear-gradient(135deg, rgba(99,102,241,0.05) 0%, rgba(139,92,246,0.05) 100%)'
          : 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
      }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 2 }}>
          <Box
            sx={{
              width: 44,
              height: 44,
              borderRadius: 3,
              background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 50%, #ec4899 100%)',
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              color: 'white',
              fontWeight: 800,
              fontSize: '1.3rem',
              boxShadow: '0 8px 32px rgba(99,102,241,0.3)',
              position: 'relative',
              '&::before': {
                content: '""',
                position: 'absolute',
                top: 0,
                left: 0,
                right: 0,
                bottom: 0,
                borderRadius: 3,
                background: 'linear-gradient(135deg, rgba(255,255,255,0.3) 0%, rgba(255,255,255,0.1) 100%)',
                backdropFilter: 'blur(10px)',
              },
            }}
          >
            <Box sx={{ position: 'relative', zIndex: 1 }}>R</Box>
          </Box>
          <Box>
            <Typography variant="h6" sx={{ 
              fontWeight: 800, 
              color: 'text.primary',
              background: theme.palette.mode === 'light'
                ? 'linear-gradient(135deg, #1e293b 0%, #6366f1 100%)'
                : 'linear-gradient(135deg, #f8fafc 0%, #a855f7 100%)',
              backgroundClip: 'text',
              WebkitBackgroundClip: 'text',
              WebkitTextFillColor: 'transparent',
              letterSpacing: '-0.025em',
            }}>
              Readur
            </Typography>
            <Typography variant="caption" sx={{ 
              color: 'text.secondary', 
              fontWeight: 500,
              letterSpacing: '0.05em',
              textTransform: 'uppercase',
              fontSize: '0.7rem',
            }}>
              AI Document Platform
            </Typography>
          </Box>
        </Box>
      </Box>

      {/* Navigation */}
      <List sx={{ flex: 1, px: 3, py: 2 }}>
        {navigationItems.map((item) => {
          const isActive = location.pathname === item.path;
          const Icon = item.icon;
          
          return (
            <ListItem key={item.text} sx={{ px: 0, mb: 1 }}>
              <ListItemButton
                onClick={() => navigate(item.path)}
                sx={{
                  borderRadius: 3,
                  minHeight: 52,
                  px: 2.5,
                  py: 1.5,
                  background: isActive 
                    ? 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)' 
                    : 'transparent',
                  color: isActive ? 'white' : 'text.primary',
                  position: 'relative',
                  overflow: 'hidden',
                  transition: 'all 0.2s ease-in-out',
                  '&:hover': {
                    backgroundColor: isActive ? 'transparent' : 'rgba(99,102,241,0.08)',
                    transform: isActive ? 'none' : 'translateX(4px)',
                    '&::before': isActive ? {} : {
                      content: '""',
                      position: 'absolute',
                      left: 0,
                      top: 0,
                      bottom: 0,
                      width: '3px',
                      background: 'linear-gradient(180deg, #6366f1 0%, #8b5cf6 100%)',
                      borderRadius: '0 2px 2px 0',
                    },
                  },
                  '&::after': isActive ? {
                    content: '""',
                    position: 'absolute',
                    top: 0,
                    left: 0,
                    right: 0,
                    bottom: 0,
                    background: 'linear-gradient(135deg, rgba(255,255,255,0.1) 0%, rgba(255,255,255,0.05) 100%)',
                    backdropFilter: 'blur(10px)',
                  } : {},
                  '& .MuiListItemIcon-root': {
                    color: isActive ? 'white' : 'text.secondary',
                    minWidth: 36,
                    position: 'relative',
                    zIndex: 1,
                  },
                  '& .MuiListItemText-root': {
                    position: 'relative',
                    zIndex: 1,
                  },
                  ...(isActive && {
                    boxShadow: '0 8px 32px rgba(99,102,241,0.3)',
                  }),
                }}
              >
                <ListItemIcon>
                  <Icon sx={{ fontSize: '1.25rem' }} />
                </ListItemIcon>
                <ListItemText 
                  primary={item.text}
                  primaryTypographyProps={{
                    fontSize: '0.9rem',
                    fontWeight: isActive ? 600 : 500,
                    letterSpacing: '0.025em',
                  }}
                />
              </ListItemButton>
            </ListItem>
          );
        })}
      </List>

      {/* User Info */}
      <Box sx={{ 
        p: 3, 
        borderTop: theme.palette.mode === 'light'
          ? '1px solid rgba(226,232,240,0.3)'
          : '1px solid rgba(255,255,255,0.1)',
        background: theme.palette.mode === 'light'
          ? 'linear-gradient(135deg, rgba(99,102,241,0.03) 0%, rgba(139,92,246,0.03) 100%)'
          : 'linear-gradient(135deg, rgba(99,102,241,0.08) 0%, rgba(139,92,246,0.08) 100%)',
      }}>
        <Box sx={{ 
          display: 'flex', 
          alignItems: 'center', 
          gap: 2.5,
          p: 2,
          borderRadius: 3,
          background: theme.palette.mode === 'light'
            ? 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)'
            : 'linear-gradient(135deg, rgba(50,50,50,0.8) 0%, rgba(30,30,30,0.6) 100%)',
          backdropFilter: 'blur(10px)',
          border: theme.palette.mode === 'light'
            ? '1px solid rgba(255,255,255,0.3)'
            : '1px solid rgba(255,255,255,0.1)',
          boxShadow: theme.palette.mode === 'light'
            ? '0 4px 16px rgba(0,0,0,0.04)'
            : '0 4px 16px rgba(0,0,0,0.2)',
        }}>
          <Avatar
            sx={{
              width: 42,
              height: 42,
              background: 'linear-gradient(135deg, #6366f1 0%, #8b5cf6 100%)',
              fontSize: '1rem',
              fontWeight: 600,
              boxShadow: '0 4px 16px rgba(99,102,241,0.3)',
            }}
          >
            {user?.username?.charAt(0).toUpperCase()}
          </Avatar>
          <Box sx={{ flex: 1, minWidth: 0 }}>
            <Typography variant="body2" sx={{ 
              fontWeight: 600, 
              color: 'text.primary',
              letterSpacing: '0.025em',
            }}>
              {user?.username}
            </Typography>
            <Typography 
              variant="caption" 
              sx={{ 
                color: 'text.secondary',
                display: 'block',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
                whiteSpace: 'nowrap',
                fontSize: '0.75rem',
                fontWeight: 500,
              }}
            >
              {user?.email}
            </Typography>
          </Box>
        </Box>
      </Box>
    </Box>
  );

  return (
    <Box sx={{ display: 'flex' }}>
      <CssBaseline />
      
      {/* App Bar */}
      <AppBar
        position="fixed"
        sx={{
          width: { md: `calc(100% - ${drawerWidth}px)` },
          ml: { md: `${drawerWidth}px` },
          background: theme.palette.mode === 'light'
            ? 'linear-gradient(135deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.90) 100%)'
            : 'linear-gradient(135deg, rgba(30,30,30,0.95) 0%, rgba(18,18,18,0.90) 100%)',
          backdropFilter: 'blur(20px)',
          borderBottom: theme.palette.mode === 'light'
            ? '1px solid rgba(226,232,240,0.5)'
            : '1px solid rgba(255,255,255,0.1)',
          boxShadow: theme.palette.mode === 'light'
            ? '0 4px 32px rgba(0,0,0,0.04)'
            : '0 4px 32px rgba(0,0,0,0.2)',
        }}
      >
        <Toolbar>
          <IconButton
            color="inherit"
            aria-label="open drawer"
            edge="start"
            onClick={handleDrawerToggle}
            sx={{ mr: 2, display: { md: 'none' } }}
          >
            <MenuIcon />
          </IconButton>
          
          <Typography variant="h6" noWrap component="div" sx={{ 
            fontWeight: 700, 
            mr: 1,
            fontSize: '1.1rem',
            background: theme.palette.mode === 'light'
              ? 'linear-gradient(135deg, #1e293b 0%, #6366f1 100%)'
              : 'linear-gradient(135deg, #f8fafc 0%, #a855f7 100%)',
            backgroundClip: 'text',
            WebkitBackgroundClip: 'text',
            WebkitTextFillColor: 'transparent',
            letterSpacing: '-0.025em',
          }}>
            {navigationItems.find(item => item.path === location.pathname)?.text || 'Dashboard'}
          </Typography>

          {/* Global Search Bar */}
          <Box sx={{ flexGrow: 2, display: 'flex', justifyContent: 'center', mx: 1, flex: '1 1 auto' }}>
            <GlobalSearchBar />
          </Box>

          {/* Notifications */}
          <IconButton 
            onClick={handleNotificationClick}
            sx={{ 
              mr: 2,
              color: 'text.secondary',
              background: theme.palette.mode === 'light'
                ? 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)'
                : 'linear-gradient(135deg, rgba(50,50,50,0.8) 0%, rgba(30,30,30,0.6) 100%)',
              backdropFilter: 'blur(10px)',
              border: theme.palette.mode === 'light'
                ? '1px solid rgba(255,255,255,0.3)'
                : '1px solid rgba(255,255,255,0.1)',
              borderRadius: 2.5,
              width: 44,
              height: 44,
              transition: 'all 0.2s ease-in-out',
              '&:hover': {
                background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
                transform: 'translateY(-2px)',
                boxShadow: '0 8px 24px rgba(99,102,241,0.15)',
              },
            }}
          >
            <Badge 
              badgeContent={unreadCount} 
              sx={{
                '& .MuiBadge-badge': {
                  background: 'linear-gradient(135deg, #ef4444 0%, #f97316 100%)',
                  color: 'white',
                  fontWeight: 600,
                  fontSize: '0.7rem',
                },
              }}
            >
              <NotificationsIcon sx={{ fontSize: '1.25rem' }} />
            </Badge>
          </IconButton>

          {/* Theme Toggle */}
          <Box sx={{ 
            mr: 2,
            background: theme.palette.mode === 'light'
              ? 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)'
              : 'linear-gradient(135deg, rgba(50,50,50,0.8) 0%, rgba(30,30,30,0.6) 100%)',
            backdropFilter: 'blur(10px)',
            border: theme.palette.mode === 'light'
              ? '1px solid rgba(255,255,255,0.3)'
              : '1px solid rgba(255,255,255,0.1)',
            borderRadius: 2.5,
            transition: 'all 0.2s ease-in-out',
            '&:hover': {
              background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
              transform: 'translateY(-2px)',
              boxShadow: '0 8px 24px rgba(99,102,241,0.15)',
            },
          }}>
            <ThemeToggle size="medium" color="inherit" />
          </Box>

          {/* Profile Menu */}
          <IconButton
            onClick={handleProfileMenuOpen}
            sx={{ 
              color: 'text.secondary',
              background: theme.palette.mode === 'light'
                ? 'linear-gradient(135deg, rgba(255,255,255,0.8) 0%, rgba(248,250,252,0.6) 100%)'
                : 'linear-gradient(135deg, rgba(50,50,50,0.8) 0%, rgba(30,30,30,0.6) 100%)',
              backdropFilter: 'blur(10px)',
              border: theme.palette.mode === 'light'
                ? '1px solid rgba(255,255,255,0.3)'
                : '1px solid rgba(255,255,255,0.1)',
              borderRadius: 2.5,
              width: 44,
              height: 44,
              transition: 'all 0.2s ease-in-out',
              '&:hover': {
                background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
                transform: 'translateY(-2px)',
                boxShadow: '0 8px 24px rgba(99,102,241,0.15)',
              },
            }}
          >
            <AccountIcon sx={{ fontSize: '1.25rem' }} />
          </IconButton>
          
          <Menu
            anchorEl={anchorEl}
            open={Boolean(anchorEl)}
            onClose={handleProfileMenuClose}
            onClick={handleProfileMenuClose}
            PaperProps={{
              elevation: 0,
              sx: {
                overflow: 'visible',
                filter: 'drop-shadow(0px 2px 8px rgba(0,0,0,0.32))',
                mt: 1.5,
                '& .MuiAvatar-root': {
                  width: 32,
                  height: 32,
                  ml: -0.5,
                  mr: 1,
                },
                '&:before': {
                  content: '""',
                  display: 'block',
                  position: 'absolute',
                  top: 0,
                  right: 14,
                  width: 10,
                  height: 10,
                  bgcolor: 'background.paper',
                  transform: 'translateY(-50%) rotate(45deg)',
                  zIndex: 0,
                },
              },
            }}
            transformOrigin={{ horizontal: 'right', vertical: 'top' }}
            anchorOrigin={{ horizontal: 'right', vertical: 'bottom' }}
          >
            <MenuItem onClick={() => navigate('/profile')}>
              <Avatar /> Profile
            </MenuItem>
            <MenuItem onClick={() => navigate('/settings')}>
              <SettingsIcon sx={{ mr: 2 }} /> Settings
            </MenuItem>
            <Divider />
            <MenuItem onClick={handleLogout}>
              <LogoutIcon sx={{ mr: 2 }} /> Logout
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
          ModalProps={{
            keepMounted: true, // Better open performance on mobile.
          }}
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

      {/* Main Content */}
      <Box
        component="main"
        sx={{
          flexGrow: 1,
          width: { md: `calc(100% - ${drawerWidth}px)` },
          minHeight: '100vh',
          backgroundColor: 'background.default',
        }}
      >
        <Toolbar />
        <Box sx={{ p: 3 }}>
          {children}
        </Box>
      </Box>

      {/* Notification Panel */}
      <NotificationPanel 
        anchorEl={notificationAnchorEl} 
        onClose={handleNotificationClose} 
      />
    </Box>
  );
};

export default AppLayout;