import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { ToastProvider, useToast } from './Toast';

function Trigger({ timeout }: { timeout?: number }) {
  const toast = useToast();
  return (
    <Button onPress={() => toast.show({ title: 'Saved', description: 'All good', tone: 'success', timeout })}>
      Show
    </Button>
  );
}

const setup = (timeout?: number) =>
  render(
    <ToastProvider>
      <Trigger timeout={timeout} />
    </ToastProvider>,
  );

afterEach(() => vi.useRealTimers());

describe('Toast', () => {
  it('shows title and description in a named region', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('button', { name: 'Show' }));
    expect(await screen.findByText('Saved')).toBeInTheDocument();
    expect(screen.getByText('All good')).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Notifications' })).toBeInTheDocument();
  });

  it('dismisses with the close button (Enter)', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('button', { name: 'Show' }));
    const close = await screen.findByRole('button', { name: 'Close' });
    close.focus();
    await user.keyboard('{Enter}');
    expect(screen.queryByText('Saved')).not.toBeInTheDocument();
  });

  it('auto-dismisses after the timeout', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    setup();
    await user.click(screen.getByRole('button', { name: 'Show' }));
    expect(await screen.findByText('Saved')).toBeInTheDocument();
    await act(async () => {
      (document.activeElement as HTMLElement | null)?.blur();
      vi.advanceTimersByTime(5100);
    });
    expect(screen.queryByText('Saved')).not.toBeInTheDocument();
  });

  it('is a no-op outside a provider', async () => {
    const user = userEvent.setup();
    render(<Trigger />);
    await user.click(screen.getByRole('button', { name: 'Show' }));
    expect(screen.queryByText('Saved')).not.toBeInTheDocument();
  });
});
