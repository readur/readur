import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { EmptyState } from './EmptyState';

describe('EmptyState', () => {
  it('renders title as a heading plus description', () => {
    render(<EmptyState title="No documents" description="Upload one to begin" />);
    expect(screen.getByRole('heading', { name: 'No documents' })).toBeInTheDocument();
    expect(screen.getByText('Upload one to begin')).toBeInTheDocument();
  });

  it('renders an actionable, keyboard-operable action', async () => {
    const onPress = vi.fn();
    const user = userEvent.setup();
    render(<EmptyState title="Empty" action={<Button onPress={onPress}>Upload</Button>} />);
    await user.tab();
    await user.keyboard('{Enter}');
    expect(onPress).toHaveBeenCalledTimes(1);
  });
});
