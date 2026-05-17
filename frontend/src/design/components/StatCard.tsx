import React, { ReactNode } from 'react';

interface StatCardProps {
  /** Small uppercase tracked label above the value. */
  label: ReactNode;
  /** The number, set in large display weight. */
  value: ReactNode;
  /** Optional unit (e.g., "GB", "docs") — small mono, sits beside the value. */
  unit?: ReactNode;
  /** Delta or supporting line below — auto-colored by `trend`. */
  delta?: ReactNode;
  /** Color the delta line. `up` = success, `down` = error, `neutral` = muted. */
  trend?: 'up' | 'down' | 'neutral';
  /** Override the value's color (e.g., red for failed counts). */
  valueColor?: string;
}

const trendColor = {
  up: 'var(--ok-60)',
  down: 'var(--err-60)',
  neutral: 'var(--fg-3)',
};

const StatCard: React.FC<StatCardProps> = ({
  label,
  value,
  unit,
  delta,
  trend = 'neutral',
  valueColor,
}) => (
  <div
    style={{
      padding: 'var(--s-5) 22px',
      background: 'var(--bg-1)',
      border: '1px solid var(--line-1)',
      borderRadius: 'var(--r-4)',
      boxShadow: 'var(--shadow-xs)',
      display: 'flex',
      flexDirection: 'column',
      minWidth: 0,
    }}
  >
    <div className="rd-label">{label}</div>
    <div style={{ marginTop: 'var(--s-3)', display: 'flex', alignItems: 'baseline' }}>
      <span
        style={{
          fontFamily: 'var(--font-sans)',
          fontWeight: 800,
          fontSize: 32,
          lineHeight: 1,
          color: valueColor || 'var(--fg-0)',
          letterSpacing: '-0.02em',
        }}
      >
        {value}
      </span>
      {unit && (
        <span
          style={{
            fontFamily: 'var(--font-mono)',
            fontWeight: 500,
            fontSize: 'var(--fs-meta)',
            color: 'var(--fg-3)',
            marginLeft: 6,
          }}
        >
          {unit}
        </span>
      )}
    </div>
    {delta && (
      <div
        style={{
          fontFamily: 'var(--font-sans)',
          fontWeight: 500,
          fontSize: 'var(--fs-micro)',
          lineHeight: 1.3,
          color: trendColor[trend],
          marginTop: 'var(--s-2)',
        }}
      >
        {delta}
      </div>
    )}
  </div>
);

export default StatCard;
