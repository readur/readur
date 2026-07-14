import React, { ReactNode, MouseEvent } from 'react';

interface FilterChipProps {
  children: ReactNode;
  active?: boolean;
  onClick?: (e: MouseEvent<HTMLButtonElement>) => void;
  /** Show a trailing × to indicate dismissable filter. */
  onRemove?: () => void;
}

const FilterChip: React.FC<FilterChipProps> = ({
  children,
  active = false,
  onClick,
  onRemove,
}) => (
  <button
    type="button"
    onClick={onClick}
    style={{
      fontFamily: 'var(--font-sans)',
      fontWeight: active ? 600 : 500,
      fontSize: 12,
      lineHeight: 1,
      padding: '7px 12px',
      borderRadius: 'var(--r-2)',
      background: active ? 'var(--accent-05)' : 'var(--bg-1)',
      color: active ? 'var(--accent-70)' : 'var(--fg-1)',
      border: `1px solid ${active ? 'var(--accent-20)' : 'var(--line-1)'}`,
      cursor: 'pointer',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      transition: 'background var(--dur-fast) var(--ease-out), border-color var(--dur-fast) var(--ease-out)',
    }}
    onMouseEnter={(e) => {
      if (!active) e.currentTarget.style.background = 'var(--bg-2)';
    }}
    onMouseLeave={(e) => {
      if (!active) e.currentTarget.style.background = 'var(--bg-1)';
    }}
  >
    {children}
    {onRemove && (
      <span
        role="button"
        aria-label="Remove filter"
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        style={{ opacity: 0.7, fontSize: 14, lineHeight: 1, marginLeft: 2 }}
      >
        ×
      </span>
    )}
  </button>
);

export default FilterChip;
