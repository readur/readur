import type { CSSProperties } from 'react';
import styles from './Skeleton.module.css';

export interface SkeletonProps {
  width?: number | string;
  height?: number | string;
  /** Render this many stacked bars (the last one shorter). Default 1. */
  lines?: number;
  /** Optional accessible name. When set the placeholder is exposed as a status region; otherwise it is hidden from AT. */
  label?: string;
  className?: string;
}

export function Skeleton({ width = '100%', height = 16, lines = 1, label, className }: SkeletonProps) {
  const bar = (w: number | string, key: number) => {
    const style: CSSProperties = { width: w, height };
    return <span key={key} className={styles.bar} style={style} />;
  };
  const a11y = label ? { role: 'status', 'aria-label': label } : { 'aria-hidden': true as const };
  if (lines <= 1) {
    return (
      <span className={className} {...a11y} style={{ display: 'block' }}>
        {bar(width, 0)}
      </span>
    );
  }
  return (
    <span className={[styles.stack, className].filter(Boolean).join(' ')} {...a11y}>
      {Array.from({ length: lines }, (_, i) => bar(i === lines - 1 ? '60%' : width, i))}
    </span>
  );
}
