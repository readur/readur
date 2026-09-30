import type { ReactNode } from 'react';
import { Button, type ButtonProps } from '../Button';
import { Tooltip, TooltipTrigger } from '../Tooltip';
import { cx } from '../shared/FieldParts';
import styles from './IconButton.module.css';

export interface IconButtonProps extends Omit<ButtonProps, 'icon' | 'children' | 'aria-label'> {
  /** Required. Becomes the aria-label and the tooltip text. */
  label: string;
  icon: ReactNode;
}

export function IconButton({ label, icon, variant = 'ghost', size = 'md', className, ...rest }: IconButtonProps) {
  return (
    <TooltipTrigger>
      <Button
        {...rest}
        variant={variant}
        size={size}
        aria-label={label}
        className={cx(styles.iconButton, styles[size], className)}
      >
        <span className={styles.glyph} aria-hidden="true">
          {icon}
        </span>
      </Button>
      <Tooltip>{label}</Tooltip>
    </TooltipTrigger>
  );
}
