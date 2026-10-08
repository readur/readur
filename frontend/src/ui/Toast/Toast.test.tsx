import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
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

describe('Toast pauses while hovered', () => {
  it('stays while the pointer is on it, its timer bar paused, and leaves after the rest of its time', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    setup(1000);
    await user.click(screen.getByRole('button', { name: 'Show' }));
    const title = await screen.findByText('Saved');
    const toast = title.closest('[data-tone], [class*="toast"]') as HTMLElement;
    const region = screen.getByRole('region', { name: 'Notifications' });

    act(() => {
      toast.dispatchEvent(new PointerEvent('pointerover', { bubbles: true, pointerType: 'mouse' }));
    });
    expect(region).toHaveAttribute('data-paused', 'true');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(screen.getByText('Saved')).toBeInTheDocument();

    act(() => {
      toast.dispatchEvent(new PointerEvent('pointerout', { bubbles: true, pointerType: 'mouse', relatedTarget: document.body }));
    });
    expect(region).not.toHaveAttribute('data-paused');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(1500);
    });
    expect(screen.queryByText('Saved')).not.toBeInTheDocument();
  });

  it('pauses the drain bar with the timer', () => {
    const css = readFileSync(resolve(__dirname, 'Toast.module.css'), 'utf8');
    expect(css).toMatch(/\.region\[data-paused\][^{]*\.timer\s*\{[^}]*animation-play-state:\s*paused/);
  });
});

describe('Progress toasts', () => {
  function Progress() {
    const toast = useToast();
    return (
      <>
        <Button onPress={() => toast.progress('sync-1', { title: 'Syncing Nextcloud', description: '240 of 412 files', value: 58 })}>Start</Button>
        <Button onPress={() => toast.progress('sync-1', { title: 'Syncing Nextcloud', description: '300 of 412 files', value: 73 })}>Advance</Button>
        <Button onPress={() => toast.dismiss('sync-1')}>Finish</Button>
      </>
    );
  }
  const renderProgress = () => render(<ToastProvider><Progress /></ToastProvider>);

  it('shows one toast with a progress bar and updates it in place', async () => {
    const user = userEvent.setup();
    renderProgress();
    await user.click(screen.getByRole('button', { name: 'Start' }));
    expect(await screen.findByText('240 of 412 files')).toBeInTheDocument();
    expect(screen.getByRole('progressbar', { name: 'Syncing Nextcloud' })).toHaveAttribute('aria-valuenow', '58');
    await user.click(screen.getByRole('button', { name: 'Advance' }));
    expect(await screen.findByText('300 of 412 files')).toBeInTheDocument();
    expect(screen.getAllByText('Syncing Nextcloud')).toHaveLength(1);
    expect(screen.getByRole('progressbar', { name: 'Syncing Nextcloud' })).toHaveAttribute('aria-valuenow', '73');
  });

  it('does not time out, and goes when dismissed', async () => {
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    renderProgress();
    await user.click(screen.getByRole('button', { name: 'Start' }));
    await screen.findByText('240 of 412 files');
    await act(async () => {
      await vi.advanceTimersByTimeAsync(20_000);
    });
    expect(screen.getByText('Syncing Nextcloud')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Finish' }));
    await act(async () => {
      await vi.advanceTimersByTimeAsync(500);
    });
    expect(screen.queryByText('Syncing Nextcloud')).not.toBeInTheDocument();
  });

  it('stays closed after the person closes it, even as progress continues', async () => {
    const user = userEvent.setup();
    renderProgress();
    await user.click(screen.getByRole('button', { name: 'Start' }));
    await screen.findByText('240 of 412 files');
    await user.click(screen.getByRole('button', { name: 'Close' }));
    await user.click(screen.getByRole('button', { name: 'Advance' }));
    expect(screen.queryByText('300 of 412 files')).not.toBeInTheDocument();
  });
});
