import { useLayoutEffect, useRef, type ReactNode } from 'react';
import { Button, type ButtonProps } from '../Button';
import { Tooltip, TooltipTrigger } from '../Tooltip';
import { cx } from '../shared/FieldParts';
import styles from './IconButton.module.css';

export interface IconButtonProps extends Omit<ButtonProps, 'icon' | 'children' | 'aria-label'> {
  /** Required. Becomes the aria-label and the tooltip text. */
  label: string;
  icon: ReactNode;
  /**
   * Shows the button muted and ignores presses, but keeps it focusable so its tooltip can say
   * why ("You can't delete your own account"). A disabled button can't show a tooltip.
   */
  disabledReason?: string;
}

export function IconButton({ label, icon, variant = 'ghost', size = 'md', className, disabledReason, onPress, ...rest }: IconButtonProps) {
  const ref = useRef<HTMLButtonElement>(null);
  const muted = Boolean(disabledReason);
  // RAC does not forward aria-disabled, so set it directly.
  useLayoutEffect(() => {
    if (muted) ref.current?.setAttribute('aria-disabled', 'true');
    else ref.current?.removeAttribute('aria-disabled');
  }, [muted]);
  return (
    <TooltipTrigger>
      <Button
        {...rest}
        ref={ref}
        onPress={muted ? undefined : onPress}
        data-muted={muted || undefined}
        variant={variant}
        size={size}
        aria-label={label}
        data-icon-button=""
        className={cx(styles.iconButton, styles[size], className)}
      >
        <span className={styles.glyph} aria-hidden="true">
          {icon}
        </span>
      </Button>
      <Tooltip>{disabledReason ?? label}</Tooltip>
    </TooltipTrigger>
  );
}
