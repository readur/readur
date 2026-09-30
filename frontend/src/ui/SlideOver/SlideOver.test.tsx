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
