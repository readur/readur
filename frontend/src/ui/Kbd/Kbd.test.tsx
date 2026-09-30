import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Kbd } from './Kbd';

describe('Kbd', () => {
  it('renders the key text', () => {
    render(<Kbd>Esc</Kbd>);
    expect(screen.getByText('Esc')).toBeInTheDocument();
  });

  it('supports an accessible label override', () => {
    render(<Kbd aria-label="Escape key">Esc</Kbd>);
    expect(screen.getByLabelText('Escape key')).toHaveTextContent('Esc');
  });
});
