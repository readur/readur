import { LoaderCircle } from 'lucide-react';
import { cx } from '../shared/FieldParts';
import styles from './Spinner.module.css';

export interface SpinnerProps {
  /** Pixel size. Default 14. */
  size?: number;
  /** Accessible name; when set the spinner is a status region, otherwise it is hidden from AT. */
  label?: string;
  className?: string;
}

/** The one loading indicator: a rotating loader. Static under reduced motion. */
export function Spinner({ size = 14, label, className }: SpinnerProps) {
  const a11y = label ? { role: 'status' as const, 'aria-label': label } : { 'aria-hidden': true as const };
  return (
    <span className={cx(styles.spinner, className)} {...a11y}>
      <LoaderCircle width={size} height={size} strokeWidth={2.25} />
    </span>
  );
}
