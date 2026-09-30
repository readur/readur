import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Dialog as RACDialog } from 'react-aria-components';
import { Button } from '../Button';
import { Popover, PopoverTrigger } from './Popover';

const setup = () =>
  render(
    <PopoverTrigger>
      <Button>Info</Button>
      <Popover>
        <RACDialog aria-label="More info">Details here</RACDialog>
      </Popover>
    </PopoverTrigger>,
  );

describe('Popover', () => {
  it('is hidden until triggered, then shows a named dialog', async () => {
    const user = userEvent.setup();
    setup();
    expect(screen.queryByText('Details here')).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Info' }));
    expect(await screen.findByRole('dialog', { name: 'More info' })).toBeInTheDocument();
  });

  it('opens with Enter and closes with Escape', async () => {
    const user = userEvent.setup();
    setup();
    await user.tab();
    await user.keyboard('{Enter}');
    await screen.findByText('Details here');
    await user.keyboard('{Escape}');
    expect(screen.queryByText('Details here')).not.toBeInTheDocument();
  });
});
