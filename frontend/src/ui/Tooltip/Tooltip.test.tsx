import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { Button } from '../Button';
import { Tooltip, TooltipTrigger } from './Tooltip';

const setup = () =>
  render(
    <TooltipTrigger>
      <Button>Hover me</Button>
      <Tooltip>Helpful hint</Tooltip>
    </TooltipTrigger>,
  );

describe('Tooltip', () => {
  it('is not rendered until the trigger is engaged', () => {
    setup();
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });

  it('shows on keyboard focus and describes the trigger', async () => {
    const user = userEvent.setup();
    setup();
    await user.tab();
    const tip = await screen.findByRole('tooltip');
    expect(tip).toHaveTextContent('Helpful hint');
    expect(screen.getByRole('button', { name: 'Hover me' })).toHaveAccessibleDescription('Helpful hint');
  });

  it('hides on Escape', async () => {
    const user = userEvent.setup();
    setup();
    await user.tab();
    await screen.findByRole('tooltip');
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });
});
