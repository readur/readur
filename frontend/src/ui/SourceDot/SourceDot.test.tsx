import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { SourceDot } from './SourceDot';
import { sourceHue } from '../../lib/sourceColor';

describe('SourceDot', () => {
  it('hides a bare dot from assistive tech and colours it by slot', () => {
    const { container } = render(<SourceDot sourceId="abc" />);
    const dot = container.querySelector('[data-slot]');
    expect(dot).toHaveAttribute('aria-hidden', 'true');
    expect(dot).toHaveAttribute('data-slot', String(sourceHue('abc').index));
  });

  it('names a dot when given a label', () => {
    render(<SourceDot sourceId={null} kind="watch" label="Watch folder" />);
    expect(screen.getByRole('img', { name: 'Watch folder' })).toHaveAttribute('data-slot', '2');
  });

  it('shows the source name in a badge in its slot colour', () => {
    render(
      <SourceDot sourceId={null} kind="upload" variant="badge" size="sm">
        Uploads
      </SourceDot>,
    );
    expect(screen.getByText('Uploads').parentElement).toHaveAttribute('data-slot', '1');
  });
});
