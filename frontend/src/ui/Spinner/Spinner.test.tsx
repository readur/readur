import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Spinner } from './Spinner';

describe('Spinner', () => {
  it('is hidden from AT without a label', () => {
    const { container } = render(<Spinner />);
    expect(container.firstElementChild).toHaveAttribute('aria-hidden', 'true');
  });
  it('is a named status with a label', () => {
    render(<Spinner label="Loading documents" />);
    expect(screen.getByRole('status', { name: 'Loading documents' })).toBeInTheDocument();
  });
});
