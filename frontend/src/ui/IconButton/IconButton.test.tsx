import { createRef } from 'react';
import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Add } from '../icons';
import { IconButton } from './IconButton';

describe('IconButton', () => {
  it("hands the caller's ref the button, as an object or a callback", () => {
    const objectRef = createRef<HTMLButtonElement>();
    const callbackRef = vi.fn();
    render(
      <>
        <IconButton label="One" icon={<Add />} ref={objectRef} />
        <IconButton label="Two" icon={<Add />} ref={callbackRef} disabledReason="Not now" />
      </>,
    );
    expect(objectRef.current).toBe(screen.getByRole('button', { name: 'One' }));
    const two = screen.getByRole('button', { name: 'Two' });
    expect(callbackRef).toHaveBeenCalledWith(two);
    // Its own use of the node still works alongside the caller's.
    expect(two).toHaveAttribute('aria-disabled', 'true');
  });

  it('uses the label as its accessible name', () => {
    render(<IconButton label="Add tag" icon={<Add />} />);
    expect(screen.getByRole('button', { name: 'Add tag' })).toBeInTheDocument();
  });

  it('activates with Enter and shows the tooltip on keyboard focus', async () => {
    const onPress = vi.fn();
    const user = userEvent.setup();
    render(<IconButton label="Add tag" icon={<Add />} onPress={onPress} />);
    await user.tab();
    expect(await screen.findByRole('tooltip')).toHaveTextContent('Add tag');
    await user.keyboard('{Enter}');
    expect(onPress).toHaveBeenCalledTimes(1);
  });

  it('renders as a filled soft square regardless of variant', () => {
    render(<IconButton label="Download" icon={<span />} />);
    expect(screen.getByRole('button', { name: 'Download' })).toHaveAttribute('data-icon-button');
  });
});

describe('IconButton with a disabled reason', () => {
  it('looks muted, ignores presses and says why in its tooltip', async () => {
    const user = userEvent.setup();
    const onPress = vi.fn();
    render(<IconButton label="Delete ada" disabledReason="You can't delete your own account" icon={<span />} onPress={onPress} />);
    const button = screen.getByRole('button', { name: 'Delete ada' });
    expect(button).toHaveAttribute('aria-disabled', 'true');
    await user.click(button);
    expect(onPress).not.toHaveBeenCalled();
    await user.click(document.body);
    await user.tab();
    expect(button).toHaveFocus();
    expect(await screen.findByRole('tooltip')).toHaveTextContent("You can't delete your own account");
  });
});
