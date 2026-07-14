import React, { ReactNode } from 'react';

interface KeyboardKbdProps {
  children: ReactNode;
}

/** Visual key chip — the kind that sits beside a search input ("⌘K"). */
const KeyboardKbd: React.FC<KeyboardKbdProps> = ({ children }) => (
  <kbd
    style={{
      fontFamily: 'var(--font-mono)',
      fontWeight: 500,
      fontSize: 10,
      lineHeight: 1,
      color: 'var(--fg-3)',
      border: '1px solid var(--line-2)',
      borderRadius: 5,
      padding: '3px 6px',
      background: 'var(--bg-1)',
      whiteSpace: 'nowrap',
    }}
  >
    {children}
  </kbd>
);

export default KeyboardKbd;
