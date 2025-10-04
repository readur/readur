import React, { useState } from 'react';
import {
  IconButton,
  Menu,
  MenuItem,
  ListItemIcon,
  ListItemText,
  Box,
  useTheme,
} from '@mui/material';
import {
  Language as LanguageIcon,
  Check as CheckIcon,
} from '@mui/icons-material';
import { useTranslation } from 'react-i18next';
import { supportedLanguages, SupportedLanguage } from '../../i18n/types';

interface LanguageSwitcherProps {
  size?: 'small' | 'medium' | 'large';
  color?: 'inherit' | 'default' | 'primary' | 'secondary';
}

const LanguageSwitcher: React.FC<LanguageSwitcherProps> = ({
  size = 'medium',
  color = 'inherit'
}) => {
  const { i18n } = useTranslation();
  const theme = useTheme();
  const [anchorEl, setAnchorEl] = useState<null | HTMLElement>(null);
  const open = Boolean(anchorEl);

  const handleClick = (event: React.MouseEvent<HTMLElement>) => {
    setAnchorEl(event.currentTarget);
  };

  const handleClose = () => {
    setAnchorEl(null);
  };

  const handleLanguageChange = (language: SupportedLanguage) => {
    i18n.changeLanguage(language);
    handleClose();
  };

  const currentLanguage = i18n.language as SupportedLanguage;

  return (
    <>
      <IconButton
        onClick={handleClick}
        size={size}
        sx={{
          color: color === 'inherit' ? 'text.secondary' : color,
          width: 44,
          height: 44,
          transition: 'all 0.2s ease-in-out',
        }}
        aria-label="change language"
        aria-controls={open ? 'language-menu' : undefined}
        aria-haspopup="true"
        aria-expanded={open ? 'true' : undefined}
      >
        <LanguageIcon sx={{ fontSize: '1.25rem' }} />
      </IconButton>
      <Menu
        id="language-menu"
        anchorEl={anchorEl}
        open={open}
        onClose={handleClose}
        onClick={handleClose}
        PaperProps={{
          elevation: 0,
          sx: {
            overflow: 'visible',
            filter: 'drop-shadow(0px 2px 8px rgba(0,0,0,0.32))',
            mt: 1.5,
            minWidth: 180,
            background: theme.palette.mode === 'light'
              ? 'linear-gradient(135deg, rgba(255,255,255,0.95) 0%, rgba(248,250,252,0.90) 100%)'
              : 'linear-gradient(135deg, rgba(30,30,30,0.95) 0%, rgba(18,18,18,0.90) 100%)',
            backdropFilter: 'blur(20px)',
            border: theme.palette.mode === 'light'
              ? '1px solid rgba(226,232,240,0.5)'
              : '1px solid rgba(255,255,255,0.1)',
            borderRadius: 3,
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
        {Object.entries(supportedLanguages).map(([code, name]) => (
          <MenuItem
            key={code}
            onClick={() => handleLanguageChange(code as SupportedLanguage)}
            selected={currentLanguage === code}
            sx={{
              borderRadius: 2,
              mx: 1,
              my: 0.5,
              transition: 'all 0.2s ease-in-out',
              '&:hover': {
                background: 'linear-gradient(135deg, rgba(99,102,241,0.1) 0%, rgba(139,92,246,0.1) 100%)',
              },
              '&.Mui-selected': {
                background: 'linear-gradient(135deg, rgba(99,102,241,0.15) 0%, rgba(139,92,246,0.15) 100%)',
                '&:hover': {
                  background: 'linear-gradient(135deg, rgba(99,102,241,0.2) 0%, rgba(139,92,246,0.2) 100%)',
                },
              },
            }}
          >
            <Box sx={{ display: 'flex', alignItems: 'center', width: '100%' }}>
              <ListItemText
                primary={name}
                primaryTypographyProps={{
                  fontSize: '0.9rem',
                  fontWeight: currentLanguage === code ? 600 : 500,
                }}
              />
              {currentLanguage === code && (
                <ListItemIcon sx={{ minWidth: 'auto', ml: 2 }}>
                  <CheckIcon
                    sx={{
                      fontSize: '1.1rem',
                      color: 'primary.main',
                    }}
                  />
                </ListItemIcon>
              )}
            </Box>
          </MenuItem>
        ))}
      </Menu>
    </>
  );
};

export default LanguageSwitcher;
