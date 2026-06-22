import React, { ReactNode, CSSProperties } from 'react';

interface PanelProps {
  children: ReactNode;
  /** Disable internal padding (useful for tables that fill edge-to-edge). */
  flush?: boolean;
  /** Make the panel emphasize its border on hover (for clickable cards). */
  interactive?: boolean;
  className?: string;
  style?: CSSProperties;
}

export const Panel: React.FC<PanelProps> = ({
  children,
  flush = false,
  interactive = false,
  className,
  style,
}) => (
  <div
    className={className}
    style={{
      background: 'var(--bg-1)',
      border: '1px solid var(--line-1)',
      borderRadius: 'var(--r-4)',
      overflow: 'hidden',
      boxShadow: 'var(--shadow-xs)',
      padding: flush ? 0 : 'var(--s-5)',
      transition: interactive
        ? 'box-shadow var(--dur-base) var(--ease-out), border-color var(--dur-base) var(--ease-out)'
        : undefined,
      ...style,
    }}
    onMouseEnter={
      interactive
        ? (e) => {
            e.currentTarget.style.boxShadow = 'var(--shadow-md)';
            e.currentTarget.style.borderColor = 'var(--accent-50)';
          }
        : undefined
    }
    onMouseLeave={
      interactive
        ? (e) => {
            e.currentTarget.style.boxShadow = 'var(--shadow-xs)';
            e.currentTarget.style.borderColor = 'var(--line-1)';
          }
        : undefined
    }
  >
    {children}
  </div>
);

interface PanelHeadProps {
  title: ReactNode;
  subtitle?: ReactNode;
  action?: ReactNode;
}

export const PanelHead: React.FC<PanelHeadProps> = ({ title, subtitle, action }) => (
  <div
    style={{
      padding: 'var(--s-4) var(--s-5)',
      borderBottom: '1px solid var(--line-1)',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: 'var(--s-4)',
    }}
  >
    <div style={{ minWidth: 0 }}>
      <div
        style={{
          fontFamily: 'var(--font-sans)',
          fontWeight: 700,
          fontSize: 14,
          color: 'var(--fg-0)',
          letterSpacing: '-0.005em',
        }}
      >
        {title}
      </div>
      {subtitle && (
        <div
          style={{
            fontFamily: 'var(--font-mono)',
            fontSize: 'var(--fs-micro)',
            color: 'var(--fg-3)',
            marginTop: 3,
            letterSpacing: 'var(--tracking-caps)',
            textTransform: 'uppercase',
          }}
        >
          {subtitle}
        </div>
      )}
    </div>
    {action && <div style={{ flexShrink: 0 }}>{action}</div>}
  </div>
);

export default Panel;
