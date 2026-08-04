import React, { ReactNode } from 'react';

interface PageHeaderProps {
  /** Small uppercase tracked label rendered above the title. */
  kicker?: ReactNode;
  /** Main page title. Accepts ReactNode so callers can emphasize words. */
  title: ReactNode;
  /** Sub-line set in body type, max ~640px wide. */
  subtitle?: ReactNode;
  /** Right-aligned action cluster (buttons, menus). */
  actions?: ReactNode;
  /** Override the title font-size token. Defaults to display-md (36px). */
  size?: 'md' | 'lg';
}

const PageHeader: React.FC<PageHeaderProps> = ({
  kicker,
  title,
  subtitle,
  actions,
  size = 'md',
}) => {
  const titleSize = size === 'lg' ? 'var(--fs-display-lg)' : 'var(--fs-display-md)';
  return (
    <header
      style={{
        marginBottom: 'var(--s-8)',
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'flex-end',
        gap: 'var(--s-6)',
        flexWrap: 'wrap',
      }}
    >
      <div style={{ minWidth: 0 }}>
        {kicker && <div className="rd-kicker">{kicker}</div>}
        <h1
          style={{
            fontFamily: 'var(--font-sans)',
            fontWeight: 800,
            fontSize: titleSize,
            lineHeight: 'var(--lh-tight)',
            letterSpacing: 'var(--tracking-display)',
            color: 'var(--fg-0)',
            margin: kicker ? '8px 0 0' : 0,
          }}
        >
          {title}
        </h1>
        {subtitle && (
          <p
            style={{
              fontFamily: 'var(--font-sans)',
              fontSize: 'var(--fs-body)',
              lineHeight: 'var(--lh-body)',
              color: 'var(--fg-2)',
              margin: '10px 0 0',
              maxWidth: 640,
            }}
          >
            {subtitle}
          </p>
        )}
      </div>
      {actions && (
        <div style={{ display: 'flex', gap: 'var(--s-2)', flexShrink: 0 }}>{actions}</div>
      )}
    </header>
  );
};

export default PageHeader;
