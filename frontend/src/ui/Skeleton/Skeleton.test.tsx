import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Skeleton } from './Skeleton';

describe('Skeleton', () => {
  it('exposes a named status region when labelled', () => {
    render(<Skeleton label="Loading documents" lines={3} />);
    expect(screen.getByRole('status', { name: 'Loading documents' })).toBeInTheDocument();
  });

  it('is hidden from assistive tech when unlabelled', () => {
    render(<Skeleton />);
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
  });

  it('renders the requested number of lines', () => {
    render(<Skeleton label="Loading" lines={4} />);
    expect(screen.getByRole('status', { name: 'Loading' }).children).toHaveLength(4);
  });
});
