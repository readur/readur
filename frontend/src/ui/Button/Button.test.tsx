import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Button } from './Button';

describe('Button', () => {
  it('renders with an accessible name from its label', () => {
    render(<Button variant="primary">Save</Button>);
    expect(screen.getByRole('button', { name: 'Save' })).toBeInTheDocument();
  });

  it('activates with Enter and Space', async () => {
    const onPress = vi.fn();
    const user = userEvent.setup();
    render(<Button onPress={onPress}>Go</Button>);
    await user.tab();
    await user.keyboard('{Enter}');
    await user.keyboard(' ');
    expect(onPress).toHaveBeenCalledTimes(2);
  });

  it('exposes aria-busy and ignores presses while pending', async () => {
    const onPress = vi.fn();
    const user = userEvent.setup();
    render(
      <Button isPending onPress={onPress}>
        Upload
      </Button>,
    );
    const button = screen.getByRole('button', { name: 'Upload' });
    expect(button).toHaveAttribute('aria-busy', 'true');
    await user.click(button);
    expect(onPress).not.toHaveBeenCalled();
  });

  it.each(['primary', 'secondary', 'ghost', 'danger', 'danger-solid'] as const)('renders the %s variant with a class', (variant) => {
    render(<Button variant={variant}>Act</Button>);
    const cls = screen.getByRole('button', { name: 'Act' }).className;
    expect(cls).not.toMatch(/undefined/);
    expect(cls.split(' ').length).toBeGreaterThanOrEqual(3);
  });

  it('does not activate when disabled', async () => {
    const onPress = vi.fn();
    const user = userEvent.setup();
    render(
      <Button isDisabled onPress={onPress}>
        Nope
      </Button>,
    );
    await user.click(screen.getByRole('button', { name: 'Nope' }));
    expect(onPress).not.toHaveBeenCalled();
  });

  it('shows the shared Spinner while pending', () => {
    render(<Button isPending>Save</Button>);
    const btn = screen.getByRole('button', { name: /save/i });
    expect(btn.querySelector('svg')).not.toBeNull();
  });
});
