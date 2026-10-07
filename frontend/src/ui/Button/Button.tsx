import { useLayoutEffect, useRef, type ReactNode, type Ref } from 'react';
import { Button as RACButton, type ButtonProps as RACButtonProps } from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import { Spinner } from '../Spinner';
import styles from './Button.module.css';

/**
 * `primary` is the filled accent action; `secondary` a quiet filled surface; `ghost` has no
 * chrome; `danger` is destructive text; `danger-solid` is the filled confirm inside a dialog.
 */
export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'danger-solid';

const VARIANT_CLASS: Record<ButtonVariant, string> = {
  primary: styles.primary,
  secondary: styles.secondary,
  ghost: styles.ghost,
  danger: styles.danger,
  'danger-solid': styles.dangerSolid,
};
export type ButtonSize = 'sm' | 'md';

export interface ButtonProps extends Omit<RACButtonProps, 'children' | 'className'> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  /** Optional leading icon. */
  icon?: ReactNode;
  /** Shows a spinner, sets aria-busy and ignores presses. */
  isPending?: boolean;
  children?: ReactNode;
  className?: string;
  ref?: Ref<HTMLButtonElement>;
}

export function Button({
  variant = 'secondary',
  size = 'md',
  icon,
  isPending = false,
  children,
  className,
  ref,
  ...rest
}: ButtonProps) {
  const innerRef = useRef<HTMLButtonElement | null>(null);
  // RAC does not forward aria-busy to the DOM, so set it directly.
  useLayoutEffect(() => {
    const el = innerRef.current;
    if (!el) return;
    if (isPending) el.setAttribute('aria-busy', 'true');
    else el.removeAttribute('aria-busy');
  }, [isPending]);
  const setRef = (el: HTMLButtonElement | null) => {
    innerRef.current = el;
    if (typeof ref === 'function') ref(el);
    else if (ref) (ref as { current: HTMLButtonElement | null }).current = el;
  };
  return (
    <RACButton
      {...rest}
      ref={setRef}
      isPending={isPending}
      className={cx(styles.button, VARIANT_CLASS[variant], styles[size], className)}
    >
      {isPending ? (
        <Spinner size={14} />
      ) : icon ? (
        <span className={styles.icon} aria-hidden="true">
          {icon}
        </span>
      ) : null}
      {children}
    </RACButton>
  );
}
