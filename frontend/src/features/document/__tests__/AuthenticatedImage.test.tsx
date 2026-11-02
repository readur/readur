import { render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AuthenticatedImage } from '../AuthenticatedImage';

describe('AuthenticatedImage', () => {
  beforeEach(() => {
    let n = 0;
    URL.createObjectURL = vi.fn(() => `blob:image-${++n}`);
    URL.revokeObjectURL = vi.fn();
  });

  it('reloads the image when the resource key changes, not when the loader does', async () => {
    const loadA = vi.fn().mockResolvedValue({ data: new Blob(['a']) });
    const loadB = vi.fn().mockResolvedValue({ data: new Blob(['b']) });

    const { rerender } = render(
      <AuthenticatedImage load={loadA} resourceKey="doc-a" alt="processed" unavailableText="unavailable" />,
    );
    await waitFor(() => expect(screen.getByAltText('processed')).toHaveAttribute('src', 'blob:image-1'));

    // A new closure for the same resource does not refetch.
    rerender(<AuthenticatedImage load={() => loadA()} resourceKey="doc-a" alt="processed" unavailableText="unavailable" />);
    expect(loadA).toHaveBeenCalledTimes(1);

    rerender(<AuthenticatedImage load={loadB} resourceKey="doc-b" alt="processed" unavailableText="unavailable" />);
    await waitFor(() => expect(screen.getByAltText('processed')).toHaveAttribute('src', 'blob:image-2'));
    expect(loadB).toHaveBeenCalledTimes(1);
    expect(URL.revokeObjectURL).toHaveBeenCalledWith('blob:image-1');
  });

  it('shows a loading state, then the message when the image is unavailable', async () => {
    const failing = vi.fn().mockRejectedValue(new Error('404'));
    render(<AuthenticatedImage load={failing} resourceKey="doc-a" alt="processed" unavailableText="unavailable" />);
    expect(screen.getByRole('status', { name: 'processed' })).toBeInTheDocument();
    expect(await screen.findByText('unavailable')).toBeInTheDocument();
  });

  it('clears a previous failure when a different resource loads', async () => {
    const failing = vi.fn().mockRejectedValue(new Error('404'));
    const working = vi.fn().mockResolvedValue({ data: new Blob(['ok']) });
    const { rerender } = render(
      <AuthenticatedImage load={failing} resourceKey="doc-a" alt="processed" unavailableText="unavailable" />,
    );
    await screen.findByText('unavailable');

    rerender(<AuthenticatedImage load={working} resourceKey="doc-b" alt="processed" unavailableText="unavailable" />);
    await waitFor(() => expect(screen.getByAltText('processed')).toBeInTheDocument());
    expect(screen.queryByText('unavailable')).not.toBeInTheDocument();
  });
});
