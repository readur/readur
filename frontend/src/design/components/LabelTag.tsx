import React, { ReactNode, MouseEvent } from 'react';

interface LabelTagProps {
  children: ReactNode;
  /** Colored dot (hex or CSS color). Hidden if undefined. */
  color?: string;
  /** Click handler for filtering. */
  onClick?: (e: MouseEvent<HTMLSpanElement>) => void;
  /** Dashed border variant — used for "+ Add" affordance. */
  dashed?: boolean;
}

const LabelTag: React.FC<LabelTagProps> = ({ children, color, onClick, dashed = false }) => (
  <span
    onClick={onClick}
    style={{
      fontFamily: 'var(--font-sans)',
      fontWeight: 500,
      fontSize: 'var(--fs-meta)',
      lineHeight: 1.2,
      padding: '5px 10px',
      borderRadius: 'var(--r-2)',
      background: 'var(--bg-1)',
      color: dashed ? 'var(--fg-3)' : 'var(--fg-1)',
      border: `1px ${dashed ? 'dashed' : 'solid'} var(--line-1)`,
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      cursor: onClick ? 'pointer' : 'default',
      whiteSpace: 'nowrap',
      transition: 'background var(--dur-fast) var(--ease-out)',
    }}
    onMouseEnter={
      onClick
        ? (e) => {
            e.currentTarget.style.background = 'var(--bg-2)';
          }
        : undefined
    }
    onMouseLeave={
      onClick
        ? (e) => {
            e.currentTarget.style.background = 'var(--bg-1)';
          }
        : undefined
    }
  >
    {color && (
      <span
        style={{
          width: 8,
          height: 8,
          borderRadius: '50%',
          background: color,
          flexShrink: 0,
        }}
      />
    )}
    {children}
  </span>
);

export default LabelTag;
