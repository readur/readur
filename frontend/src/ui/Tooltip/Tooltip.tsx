import type { ReactNode } from 'react';
import {
  OverlayArrow,
  Tooltip as RACTooltip,
  TooltipTrigger as RACTooltipTrigger,
  type TooltipProps as RACTooltipProps,
  type TooltipTriggerComponentProps,
} from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Tooltip.module.css';

export type TooltipProps = Omit<RACTooltipProps, 'children' | 'className'> & {
  children?: ReactNode;
  className?: string;
};

/** Tooltip bubble. Rendered in a portal by React Aria. Use inside `TooltipTrigger`. */
export function Tooltip({ children, className, offset = 6, ...rest }: TooltipProps) {
  return (
    <RACTooltip {...rest} offset={offset} className={cx(styles.tooltip, className)}>
      <OverlayArrow className={styles.arrow}>
        <svg width={10} height={6} viewBox="0 0 10 6" aria-hidden="true">
          <path d="M0 0 L5 6 L10 0" />
        </svg>
      </OverlayArrow>
      {children}
    </RACTooltip>
  );
}

/** Wraps a focusable trigger and a `Tooltip`. Opens after 500ms on hover, immediately on keyboard focus. */
export function TooltipTrigger({ delay = 500, ...rest }: TooltipTriggerComponentProps) {
  return <RACTooltipTrigger delay={delay} {...rest} />;
}
