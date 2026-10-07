import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Notice } from './Notice';

describe('Notice', () => {
  it('danger is an alert carrying its tone', () => {
    render(<Notice tone="danger" title="Can’t reach dav.home.lan">Connection refused</Notice>);
    expect(screen.getByRole('alert')).toHaveAttribute('data-tone', 'danger');
  });

  it('other tones are a status by default', () => {
    render(<Notice tone="warning" title="Low OCR confidence" />);
    expect(screen.getByRole('status')).toHaveAttribute('data-tone', 'warning');
  });

  it('can opt out of a live role and announce a hidden prefix', () => {
    const { container } = render(<Notice tone="ok" live="off" prefix="Done:" title="Sync complete" />);
    expect(container.firstElementChild).not.toHaveAttribute('role');
    expect(screen.getByText('Done:')).toHaveClass('visually-hidden');
  });

  it('renders an action and a dismiss button', () => {
    render(<Notice title="x" action={<button type="button">Review</button>} onDismiss={() => {}} />);
    expect(screen.getByRole('button', { name: 'Review' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
  });
});
