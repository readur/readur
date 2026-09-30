import { useLayoutEffect, useRef, type ReactNode, type Ref } from 'react';
import { Button as RACButton, type ButtonProps as RACButtonProps } from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Button.module.css';

export type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger';
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
      className={cx(styles.button, styles[variant], styles[size], className)}
    >
      {isPending ? (
        <span className={styles.spinner} aria-hidden="true" />
      ) : icon ? (
        <span className={styles.icon} aria-hidden="true">
          {icon}
        </span>
      ) : null}
      {children}
    </RACButton>
  );
}
