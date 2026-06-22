import React, { ReactNode } from 'react';

interface SnippetProps {
  children: ReactNode;
}

/**
 * Search-result snippet. Wraps the matched text with an accent-tinted
 * background and a leading rule. Use `<mark>` inside for the actual matches.
 */
const Snippet: React.FC<SnippetProps> = ({ children }) => (
  <div
    style={{
      fontFamily: 'var(--font-sans)',
      fontSize: 14,
      lineHeight: 1.6,
      color: 'var(--fg-1)',
      padding: '10px 14px',
      background: 'var(--accent-05)',
      borderLeft: '3px solid var(--accent-60)',
      borderRadius: '0 var(--r-2) var(--r-2) 0',
      marginTop: 10,
    }}
  >
    {children}
  </div>
);

export default Snippet;
