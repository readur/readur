import { useState } from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Button } from '../Button';
import { Dialog } from './Dialog';

function Harness({ isDismissable }: { isDismissable?: boolean }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onPress={() => setOpen(true)}>Open</Button>
      <Dialog
        title="Delete document"
        isOpen={open}
        onOpenChange={setOpen}
        isDismissable={isDismissable}
        actions={<Button onPress={() => setOpen(false)}>Cancel</Button>}
      >
        Are you sure?
      </Dialog>
    </>
  );
}

describe('Dialog', () => {
  it('renders closed by default and opens with an accessible name from its title', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Open' }));
    expect(await screen.findByRole('dialog', { name: 'Delete document' })).toBeInTheDocument();
    expect(screen.getByText('Are you sure?')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Delete document' })).toBeInTheDocument();
  });

  it('closes on Escape and returns focus to the trigger', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const trigger = screen.getByRole('button', { name: 'Open' });
    await user.click(trigger);
    await screen.findByRole('dialog');
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it('keeps focus inside while open', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole('button', { name: 'Open' }));
    const dialog = await screen.findByRole('dialog');
    await user.tab();
    await user.tab();
    expect(dialog).toContainElement(document.activeElement as HTMLElement);
  });

  it('activates footer actions with Enter', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    await user.click(screen.getByRole('button', { name: 'Open' }));
    await screen.findByRole('dialog');
    await user.tab();
    await user.keyboard('{Enter}');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('shows an icon tile and a help link', () => {
    render(
      <Dialog
        isOpen
        title="Retry OCR for 3 documents"
        icon={<span data-testid="dialog-icon" />}
        helpLink={<a href="#help">What does this do?</a>}
        actions={<button type="button">Retry</button>}
      >
        body
      </Dialog>,
    );
    expect(screen.getByRole('dialog', { name: 'Retry OCR for 3 documents' })).toBeInTheDocument();
    expect(screen.getByTestId('dialog-icon').closest('[data-icon-tile]')).not.toBeNull();
    expect(screen.getByRole('link', { name: 'What does this do?' })).toBeInTheDocument();
  });
});
