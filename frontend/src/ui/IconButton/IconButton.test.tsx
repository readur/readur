import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Add } from '../icons';
import { IconButton } from './IconButton';

describe('IconButton', () => {
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
