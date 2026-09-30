import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Switch } from './Switch';

describe('Switch', () => {
  it('renders with an accessible name and description text', () => {
    render(<Switch label="Auto OCR" description="Run on upload" />);
    expect(screen.getByRole('switch', { name: /Auto OCR/ })).not.toBeChecked();
    expect(screen.getByText('Run on upload')).toBeInTheDocument();
  });

  it('toggles with Space', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<Switch label="Auto OCR" onChange={onChange} />);
    await user.tab();
    await user.keyboard(' ');
    expect(onChange).toHaveBeenCalledWith(true);
    expect(screen.getByRole('switch', { name: 'Auto OCR' })).toBeChecked();
  });

  it('does not toggle when disabled', async () => {
    const user = userEvent.setup();
    render(<Switch label="Auto OCR" isDisabled />);
    await user.click(screen.getByRole('switch', { name: 'Auto OCR' }));
    expect(screen.getByRole('switch', { name: 'Auto OCR' })).not.toBeChecked();
  });
});
