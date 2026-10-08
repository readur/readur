import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SearchTrigger } from '../SearchTrigger';

describe('SearchTrigger', () => {
  it.each([false, true])('announces its keyboard shortcuts (compact: %s)', (compact) => {
    render(<SearchTrigger onOpen={() => {}} compact={compact} />);
    expect(screen.getByRole('button', { name: 'Search documents' })).toHaveAttribute('aria-keyshortcuts', 'Meta+K Control+K');
  });
});
