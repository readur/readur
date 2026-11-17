import { screen, within } from '@testing-library/react';
import { useState } from 'react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CopyBlock } from '../apiKeys/CopyBlock';
import { renderSettings } from './settingsTestUtils';

/** The block's own live region (the settings shell renders a toast region too). */
const status = () => within(screen.getByTestId('block')).getByRole('status');

function Switchable() {
  const [value, setValue] = useState('first');
  return (
    <div data-testid="block">
      <CopyBlock label="Command" value={value} />
      <button type="button" onClick={() => setValue('second')}>
        Change
      </button>
    </div>
  );
}

// The one copy-to-clipboard widget behind the API key reveal dialog and the MCP snippets.
describe('CopyBlock', () => {
  it('shows the value and copies it, announcing success to screen readers', async () => {
    const user = userEvent.setup();
    renderSettings(
      <div data-testid="block">
        <CopyBlock label="MCP endpoint" value="https://readur.example.com/api/mcp" />
      </div>,
      { media: null },
    );
    expect(screen.getByLabelText('MCP endpoint')).toHaveTextContent(
      'https://readur.example.com/api/mcp',
    );

    await user.click(screen.getByRole('button', { name: 'Copy MCP endpoint' }));
    expect(await navigator.clipboard.readText()).toBe('https://readur.example.com/api/mcp');
    expect(status()).toHaveTextContent('Copied to clipboard');
    expect(screen.getByRole('button', { name: 'Copied' })).toBeInTheDocument();
  });

  it('uses copyLabel for the button when given', () => {
    renderSettings(<CopyBlock label="API key" copyLabel="Copy API key" value="readur_pat_x" />, {
      media: null,
    });
    expect(screen.getByRole('button', { name: 'Copy API key' })).toBeInTheDocument();
  });

  it('resets the copied state when the value changes', async () => {
    const user = userEvent.setup();
    renderSettings(<Switchable />, { media: null });
    await user.click(screen.getByRole('button', { name: 'Copy Command' }));
    expect(status()).toHaveTextContent('Copied to clipboard');

    await user.click(screen.getByRole('button', { name: 'Change' }));
    expect(screen.getByRole('button', { name: 'Copy Command' })).toBeInTheDocument();
    expect(status()).toHaveTextContent('');
  });

  it('tells the user to copy by hand when the clipboard is unavailable', async () => {
    const user = userEvent.setup();
    vi.spyOn(navigator.clipboard, 'writeText').mockRejectedValueOnce(new Error('denied'));
    renderSettings(<CopyBlock label="API key" copyLabel="Copy API key" value="readur_pat_x" />, {
      media: null,
    });
    await user.click(screen.getByRole('button', { name: 'Copy API key' }));
    expect(
      await screen.findByText('Failed to copy to clipboard. Please select and copy manually.'),
    ).toBeInTheDocument();
  });
});
