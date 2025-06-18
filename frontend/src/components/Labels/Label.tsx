import React from 'react';
import { Chip, IconButton, Box, Typography } from '@mui/material';
import { 
  Close as CloseIcon,
  Star as StarIcon,
  Archive as ArchiveIcon,
  Person as PersonIcon,
  Work as WorkIcon,
  Receipt as ReceiptIcon,
  Scale as ScaleIcon,
  LocalHospital as MedicalIcon,
  AttachMoney as DollarIcon,
  BusinessCenter as BriefcaseIcon,
  Description as DocumentIcon,
  Label as LabelIcon,
  BugReport as BugIcon,
  Build as BuildIcon
} from '@mui/icons-material';

export interface LabelData {
  id: string;
  name: string;
  description?: string;
  color: string;
  background_color?: string;
  icon?: string;
  is_system: boolean;
  document_count?: number;
  source_count?: number;
}

interface LabelProps {
  label: LabelData;
  size?: 'small' | 'medium' | 'large';
  variant?: 'filled' | 'outlined';
  showCount?: boolean;
  deletable?: boolean;
  onDelete?: (labelId: string) => void;
  onClick?: (labelId: string) => void;
  disabled?: boolean;
  className?: string;
}

const iconMap: Record<string, React.ElementType> = {
  star: StarIcon,
  archive: ArchiveIcon,
  user: PersonIcon,
  person: PersonIcon,
  work: WorkIcon,
  briefcase: BriefcaseIcon,
  receipt: ReceiptIcon,
  scale: ScaleIcon,
  medical: MedicalIcon,
  dollar: DollarIcon,
  document: DocumentIcon,
  label: LabelIcon,
  bug: BugIcon,
  build: BuildIcon,
};

const Label: React.FC<LabelProps> = ({
  label,
  size = 'medium',
  variant = 'filled',
  showCount = false,
  deletable = false,
  onDelete,
  onClick,
  disabled = false,
  className
}) => {
  const IconComponent = label.icon ? iconMap[label.icon] : null;
  
  const handleDelete = (event: React.MouseEvent) => {
    event.stopPropagation();
    if (onDelete && !disabled) {
      onDelete(label.id);
    }
  };

  const handleClick = () => {
    if (onClick && !disabled) {
      onClick(label.id);
    }
  };

  const getChipSize = () => {
    switch (size) {
      case 'small': return 'small' as const;
      case 'large': return 'medium' as const;
      default: return 'medium' as const;
    }
  };

  const chipProps = {
    label: (
      <Box display="flex" alignItems="center" gap={0.5}>
        {IconComponent && (
          <IconComponent 
            sx={{ 
              fontSize: size === 'small' ? '14px' : '16px',
              color: 'inherit'
            }} 
          />
        )}
        <Typography
          variant={size === 'small' ? 'caption' : 'body2'}
          component="span"
          sx={{ fontWeight: 500 }}
        >
          {label.name}
        </Typography>
        {showCount && (label.document_count || 0) > 0 && (
          <Typography
            variant="caption"
            component="span"
            sx={{ 
              opacity: 0.8,
              fontSize: size === 'small' ? '0.7rem' : '0.75rem'
            }}
          >
            ({label.document_count})
          </Typography>
        )}
      </Box>
    ),
    size: getChipSize(),
    variant: variant,
    clickable: !!onClick && !disabled,
    onClick: handleClick,
    disabled: disabled,
    className: className,
    sx: {
      backgroundColor: variant === 'filled' ? label.color : 'transparent',
      color: variant === 'filled' ? getContrastColor(label.color) : label.color,
      borderColor: variant === 'outlined' ? label.color : 'transparent',
      '&:hover': {
        backgroundColor: variant === 'filled' 
          ? adjustBrightness(label.color, -0.1)
          : `${label.color}20`,
      },
      '&.MuiChip-clickable:hover': {
        backgroundColor: variant === 'filled' 
          ? adjustBrightness(label.color, -0.1)
          : `${label.color}20`,
      },
      '& .MuiChip-deleteIcon': {
        color: variant === 'filled' ? getContrastColor(label.color) : label.color,
        '&:hover': {
          color: variant === 'filled' ? getContrastColor(label.color) : adjustBrightness(label.color, -0.2),
        }
      },
      transition: 'all 0.2s ease-in-out',
      cursor: onClick ? 'pointer' : 'default',
    }
  };

  if (deletable && !label.is_system) {
    return (
      <Chip
        {...chipProps}
        onDelete={handleDelete}
        deleteIcon={<CloseIcon />}
      />
    );
  }

  return <Chip {...chipProps} />;
};

// Helper function to determine if we should use light or dark text
function getContrastColor(hexColor: string): string {
  // Remove # if present
  const color = hexColor.replace('#', '');
  
  // Convert to RGB
  const r = parseInt(color.substr(0, 2), 16);
  const g = parseInt(color.substr(2, 2), 16);
  const b = parseInt(color.substr(4, 2), 16);
  
  // Calculate luminance
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  
  return luminance > 0.5 ? '#000000' : '#ffffff';
}

// Helper function to adjust brightness
function adjustBrightness(hexColor: string, factor: number): string {
  const color = hexColor.replace('#', '');
  const r = Math.max(0, Math.min(255, parseInt(color.substr(0, 2), 16) + factor * 255));
  const g = Math.max(0, Math.min(255, parseInt(color.substr(2, 2), 16) + factor * 255));
  const b = Math.max(0, Math.min(255, parseInt(color.substr(4, 2), 16) + factor * 255));
  
  return `#${Math.round(r).toString(16).padStart(2, '0')}${Math.round(g).toString(16).padStart(2, '0')}${Math.round(b).toString(16).padStart(2, '0')}`;
}

export default Label;