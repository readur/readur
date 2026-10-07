import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { Facts, YesNo } from './Facts';

describe('Facts', () => {
  it('renders a titled definition list', () => {
    render(
      <Facts
        title="Details"
        items={[
          { label: 'Pages', value: '14', mono: true },
          { label: 'Language', value: 'English' },
        ]}
      />,
    );
    expect(screen.getByText('Details')).toBeInTheDocument();
    expect(screen.getByText('Pages').tagName).toBe('DT');
    expect(screen.getByText('14')).toHaveAttribute('data-mono');
    expect(screen.getByText('English')).not.toHaveAttribute('data-mono');
  });

  it('YesNo carries a word and a shape', () => {
    render(<YesNo value yes="Yes" no="No" />);
    expect(screen.getByText(/Yes/).closest('[data-value]')).toHaveAttribute('data-value', 'yes');
  });
});
