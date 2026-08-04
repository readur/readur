import React, { ReactNode } from 'react';

export type PillVariant = 'ok' | 'warn' | 'err' | 'info' | 'brand' | 'neutral';

interface PillProps {
  variant?: PillVariant;
  /** Show the leading status dot. Default true. */
  dot?: boolean;
  children: ReactNode;
}

const palette: Record<PillVariant, { bg: string; fg: string }> = {
  ok: { bg: 'var(--ok-05)', fg: 'var(--ok-70)' },
  warn: { bg: 'var(--warn-05)', fg: 'var(--warn-70)' },
  err: { bg: 'var(--err-05)', fg: 'var(--err-70)' },
  info: { bg: 'var(--info-05)', fg: 'var(--info-70)' },
  brand: { bg: 'var(--accent-05)', fg: 'var(--accent-70)' },
  neutral: { bg: 'var(--bg-2)', fg: 'var(--fg-2)' },
};

const Pill: React.FC<PillProps> = ({ variant = 'neutral', dot = true, children }) => {
  const c = palette[variant];
  return (
    <span
      style={{
        fontFamily: 'var(--font-sans)',
        fontWeight: 500,
        fontSize: 'var(--fs-micro)',
        lineHeight: 1,
        padding: '5px 9px',
        borderRadius: 'var(--r-1)',
        background: c.bg,
        color: c.fg,
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        whiteSpace: 'nowrap',
      }}
    >
      {dot && (
        <span
          style={{
            width: 6,
            height: 6,
            borderRadius: '50%',
            background: 'currentColor',
            flexShrink: 0,
          }}
        />
      )}
      {children}
    </span>
  );
};

export default Pill;
