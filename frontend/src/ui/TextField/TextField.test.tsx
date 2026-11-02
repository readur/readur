import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { TextField } from './TextField';

describe('TextField', () => {
  it('renders a visible label wired to the input, with description', () => {
    render(<TextField label="Email" description="We never share it" />);
    const input = screen.getByRole('textbox', { name: 'Email' });
    expect(input).toBeInTheDocument();
    expect(input).toHaveAccessibleDescription('We never share it');
  });

  it('accepts keyboard input', async () => {
    const user = userEvent.setup();
    render(<TextField label="Name" />);
    await user.type(screen.getByRole('textbox', { name: 'Name' }), 'abc');
    expect(screen.getByRole('textbox', { name: 'Name' })).toHaveValue('abc');
  });

  it('shows the error message when invalid', () => {
    render(<TextField label="Name" isInvalid errorMessage="Required field" />);
    expect(screen.getByRole('textbox', { name: 'Name' })).toBeInvalid();
    expect(screen.getByText('Required field')).toBeInTheDocument();
  });

  it('supports required, disabled and multiline', () => {
    render(<TextField label="Notes" multiline isRequired isDisabled />);
    const box = screen.getByRole('textbox', { name: /Notes/ });
    expect(box).toBeRequired();
    expect(box).toBeDisabled();
  });
});
