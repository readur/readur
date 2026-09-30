import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { ToastProvider, useToast } from './Toast';

function Trigger({ timeout, tone = 'success' }: { timeout?: number; tone?: 'info' | 'success' | 'danger' }) {
  const toast = useToast();
  return (
    <Button onPress={() => toast.show({ title: 'Saved', description: 'All good', tone, timeout })}>
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

  it.each([
    ['info', 'Info:'],
    ['success', 'Success:'],
    ['danger', 'Error:'],
  ] as const)('exposes the %s tone as text to assistive tech', async (tone, prefix) => {
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <Trigger tone={tone} />
      </ToastProvider>,
    );
    await user.click(screen.getByRole('button', { name: 'Show' }));
    await screen.findByText('All good');
    expect(screen.getByText('Saved')).toHaveTextContent(`${prefix} Saved`);
  });

  it('announces danger toasts assertively (role=alert)', async () => {
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <Trigger tone="danger" />
      </ToastProvider>,
    );
    await user.click(screen.getByRole('button', { name: 'Show' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Error: Saved');
  });

  it('announces non-danger toasts politely (role=status)', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('button', { name: 'Show' }));
    await screen.findByText('All good');
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    expect(screen.getByRole('status')).toHaveTextContent('All good');
  });
});
