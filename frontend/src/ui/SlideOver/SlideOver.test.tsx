import { useState } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { SlideOver } from './SlideOver';

const DOCS = ['invoice.pdf', 'receipt.png', 'contract.docx'];

function Harness({ onNavigateSpy }: { onNavigateSpy?: (d: string) => void }) {
  const [open, setOpen] = useState(false);
  const [index, setIndex] = useState(0);
  const name = DOCS[index];
  return (
    <>
      <Button onPress={() => setOpen(true)}>Open details</Button>
      <SlideOver
        title={name}
        isOpen={open}
        onOpenChange={setOpen}
        onNavigate={(dir) => {
          onNavigateSpy?.(dir);
          setIndex((i) => Math.max(0, Math.min(DOCS.length - 1, i + (dir === 'next' ? 1 : -1))));
        }}
        footer={<Button onPress={() => setOpen(false)}>Done</Button>}
      >
        {/* Keyed per record so the button is replaced when the record changes. */}
        <Button key={name} onPress={() => setIndex((i) => (i + 1) % DOCS.length)}>
          {`Next after ${name}`}
        </Button>
        <p>{`Details for ${name}`}</p>
      </SlideOver>
    </>
  );
}

describe('SlideOver', () => {
  it('opens a dialog named by its title with a close button and footer', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    const dialog = await screen.findByRole('dialog', { name: 'invoice.pdf' });
    expect(dialog).toContainElement(screen.getByRole('heading', { name: 'invoice.pdf' }));
    expect(screen.getByRole('button', { name: 'Close' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Done' })).toBeInTheDocument();
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
  });

  it('closes on Escape and returns focus to the trigger', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole('button', { name: 'Open details' });
    await user.click(trigger);
    await screen.findByRole('dialog');
    await user.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it('closes from the close button', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    await screen.findByRole('dialog');
    await user.click(screen.getByRole('button', { name: 'Close' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
  });

  it('keeps focus inside when its content changes while open', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    const dialog = await screen.findByRole('dialog');
    const next = screen.getByRole('button', { name: 'Next after invoice.pdf' });
    act(() => next.focus());
    await user.keyboard('{Enter}');
    expect(screen.getByText('Details for receipt.png')).toBeInTheDocument();
    // The same dialog node is still mounted (no remount) and still holds focus.
    expect(screen.getByRole('dialog', { name: 'receipt.png' })).toBe(dialog);
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
  });

  it('calls onNavigate for arrow keys when focus is not in a field', async () => {
    const spy = vi.fn();
    const user = userEvent.setup();
    render(<Harness onNavigateSpy={spy} />);
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    await screen.findByRole('dialog');
    act(() => screen.getByRole('button', { name: 'Close' }).focus());
    await user.keyboard('{ArrowDown}');
    expect(spy).toHaveBeenLastCalledWith('next');
    expect(screen.getByRole('dialog', { name: 'receipt.png' })).toBeInTheDocument();
    await user.keyboard('{ArrowUp}');
    expect(spy).toHaveBeenLastCalledWith('previous');
    expect(screen.getByRole('dialog', { name: 'invoice.pdf' })).toBeInTheDocument();
  });
});

describe('SlideOver content that keeps its arrow keys', () => {
  it('leaves ↑/↓ to an element marked data-own-arrows, such as a long text pane', async () => {
    const spy = vi.fn();
    const user = userEvent.setup();
    render(
      <SlideOver title="Doc" isOpen onOpenChange={() => undefined} onNavigate={spy}>
        <div data-own-arrows="" tabIndex={0} aria-label="Text">
          long text
        </div>
      </SlideOver>,
    );
    act(() => screen.getByLabelText('Text').focus());
    await user.keyboard('{ArrowDown}');
    expect(spy).not.toHaveBeenCalled();
  });
});

describe('SlideOver fill layout', () => {
  it('marks the body so its content lays itself out edge to edge', () => {
    render(
      <SlideOver title="Doc" isOpen onOpenChange={() => undefined} layout="fill">
        <p>Body</p>
      </SlideOver>,
    );
    expect(screen.getByText('Body').parentElement).toHaveAttribute('data-layout', 'fill');
  });
});

describe('SlideOver resizing', () => {
  const KEY = 'readur.slideover.test';
  const Panel = ({ resizable = true }: { resizable?: boolean }) => (
    <SlideOver title="Details" isOpen onOpenChange={() => undefined} resizable={resizable} storageKey="test">
      <p>Body</p>
    </SlideOver>
  );
  const panelWidth = () => (screen.getByRole('dialog').closest('[style]') as HTMLElement | null)?.style.getPropertyValue('--slideover-width');

  it('opens at half the window with a labelled grab handle that widens with ArrowLeft and remembers the width', async () => {
    window.localStorage.removeItem(KEY);
    const user = userEvent.setup();
    render(<Panel />);
    const handle = screen.getByRole('separator', { name: 'Resize panel' });
    expect(handle).toHaveAttribute('aria-orientation', 'vertical');
    expect(handle).toHaveAttribute('aria-valuenow', String(Math.round(window.innerWidth * 0.5)));
    expect(handle).toHaveAttribute('aria-valuemax', String(Math.round(window.innerWidth * 0.85)));
    handle.focus();
    await user.keyboard('{ArrowLeft}');
    const half = Math.round(window.innerWidth * 0.5);
    expect(handle).toHaveAttribute('aria-valuenow', String(half + 24));
    expect(panelWidth()).toBe(`${half + 24}px`);
    expect(window.localStorage.getItem(KEY)).toBe(String(half + 24));
    await user.keyboard('{ArrowRight}{ArrowRight}');
    expect(handle).toHaveAttribute('aria-valuenow', String(half - 24));
  });

  it('opens at the remembered width, kept within bounds', () => {
    window.localStorage.setItem(KEY, '600');
    const { unmount } = render(<Panel />);
    expect(screen.getByRole('separator', { name: 'Resize panel' })).toHaveAttribute('aria-valuenow', '600');
    expect(panelWidth()).toBe('600px');
    unmount();
    window.localStorage.setItem(KEY, '99999');
    render(<Panel />);
    const handle = screen.getByRole('separator', { name: 'Resize panel' });
    expect(Number(handle.getAttribute('aria-valuenow'))).toBe(Number(handle.getAttribute('aria-valuemax')));
    expect(handle).toHaveAttribute('aria-valuemin', '360');
  });

  it('follows a pointer drag on the handle', () => {
    window.localStorage.removeItem(KEY);
    render(<Panel />);
    const handle = screen.getByRole('separator', { name: 'Resize panel' });
    handle.setPointerCapture = vi.fn();
    act(() => {
      handle.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, clientX: 600, pointerId: 1, button: 0 }));
      handle.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientX: 500, pointerId: 1 }));
      handle.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientX: 500, pointerId: 1 }));
    });
    const dragged = Math.round(window.innerWidth * 0.5) + 100;
    expect(handle).toHaveAttribute('aria-valuenow', String(dragged));
    expect(window.localStorage.getItem(KEY)).toBe(String(dragged));
  });

  it('opens at the given share of the window when nothing is remembered', () => {
    window.localStorage.removeItem('readur.slideover.share');
    render(
      <SlideOver title="Details" isOpen onOpenChange={() => undefined} resizable storageKey="share" defaultShare={0.6}>
        <p>Body</p>
      </SlideOver>,
    );
    expect(screen.getByRole('separator', { name: 'Resize panel' })).toHaveAttribute(
      'aria-valuenow',
      String(Math.round(window.innerWidth * 0.6)),
    );
  });

  it('has no handle unless asked for one', () => {
    render(<Panel resizable={false} />);
    expect(screen.queryByRole('separator')).not.toBeInTheDocument();
  });
});
