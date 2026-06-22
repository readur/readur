import React, { ReactNode } from 'react';

interface EmptyStateProps {
  /** Optional icon or thumbnail rendered above the title. */
  icon?: ReactNode;
  title: ReactNode;
  description?: ReactNode;
  /** Primary action button (or any cluster). */
  action?: ReactNode;
}

const EmptyState: React.FC<EmptyStateProps> = ({ icon, title, description, action }) => (
  <div
    style={{
      padding: '64px 32px',
      textAlign: 'center',
      background: 'var(--bg-1)',
      border: '1px solid var(--line-1)',
      borderRadius: 'var(--r-4)',
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      gap: 'var(--s-3)',
    }}
  >
    {icon && <div style={{ color: 'var(--fg-3)' }}>{icon}</div>}
    <div
      style={{
        fontFamily: 'var(--font-sans)',
        fontWeight: 700,
        fontSize: 'var(--fs-h2)',
        color: 'var(--fg-0)',
        letterSpacing: '-0.01em',
      }}
    >
      {title}
    </div>
    {description && (
      <p
        style={{
          fontFamily: 'var(--font-sans)',
          fontSize: 'var(--fs-body)',
          lineHeight: 'var(--lh-body)',
          color: 'var(--fg-2)',
          margin: 0,
          maxWidth: 440,
        }}
      >
        {description}
      </p>
    )}
    {action && <div style={{ marginTop: 'var(--s-2)' }}>{action}</div>}
  </div>
);

export default EmptyState;
