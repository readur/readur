import React from 'react';
import { IconButton, Tooltip, Box } from '@mui/material';
import { Brightness4, Brightness7 } from '../../design/icons';
import { useTheme } from '../../contexts/ThemeContext';

interface ThemeToggleProps {
  size?: 'small' | 'medium' | 'large';
  color?: 'inherit' | 'primary' | 'secondary' | 'default';
}

const ThemeToggle: React.FC<ThemeToggleProps> = ({ 
  size = 'medium', 
  color = 'inherit' 
}) => {
  const { mode, toggleTheme } = useTheme();

  return (
    <Tooltip title={`Switch to ${mode === 'light' ? 'dark' : 'light'} mode`}>
      <IconButton
        onClick={toggleTheme}
        color={color}
        size={size}
        sx={{
          transition: 'all 0.3s ease-in-out',
          color: mode === 'light' ? '#6366f1' : '#fbbf24',
          '&:hover': {
            transform: 'rotate(180deg)',
            color: mode === 'light' ? '#4f46e5' : '#f59e0b',
          },
        }}
      >
        <Box
          sx={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            transition: 'transform 0.3s ease-in-out',
          }}
        >
          {mode === 'light' ? <Brightness4 /> : <Brightness7 />}
        </Box>
      </IconButton>
    </Tooltip>
  );
};

export default ThemeToggle;