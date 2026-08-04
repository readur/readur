import React, { ReactNode, CSSProperties } from 'react';

interface MetaTextProps {
  children: ReactNode;
  as?: 'span' | 'div' | 'td';
  /** Render with `var(--fg-2)` instead of `var(--fg-3)` — slightly darker for emphasis. */
  emphasis?: boolean;
  style?: CSSProperties;
  className?: string;
}

/**
 * Monospace tabular-figure text for metadata: file sizes, dates, durations,
 * counts, hashes, paths. Always uses `font-variant-numeric: tabular-nums`
 * so numbers line up in tables.
 */
const MetaText: React.FC<MetaTextProps> = ({
  children,
  as: Component = 'span',
  emphasis = false,
  style,
  className,
}) => (
  <Component
    className={className}
    style={{
      fontFamily: 'var(--font-mono)',
      fontSize: 'var(--fs-meta)',
      color: emphasis ? 'var(--fg-2)' : 'var(--fg-3)',
      fontFeatureSettings: '"tnum"',
      fontVariantNumeric: 'tabular-nums',
      ...style,
    }}
  >
    {children}
  </Component>
);

export default MetaText;
