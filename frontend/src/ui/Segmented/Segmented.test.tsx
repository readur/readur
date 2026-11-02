import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Segmented } from './Segmented';

describe('Segmented', () => {
  it('is a single-select radiogroup', async () => {
    const onChange = vi.fn();
    render(
      <Segmented
        label="Layout"
        value="grid"
        onChange={onChange}
        items={[
          { id: 'grid', label: 'Grid' },
          { id: 'table', label: 'Table' },
        ]}
      />,
    );
    expect(screen.getByRole('radiogroup', { name: 'Layout' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('radio', { name: 'Table' }));
    expect(onChange).toHaveBeenCalledWith('table');
  });
});
