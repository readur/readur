import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { OutcomeBar, ProgressBar } from './Progress';

describe('ProgressBar', () => {
  it('is a clamped, named progressbar', () => {
    render(<ProgressBar value={140} label="Files progress" showValue />);
    expect(screen.getByRole('progressbar', { name: 'Files progress' })).toHaveAttribute('aria-valuenow', '100');
  });

  it('rounds and clamps below zero', () => {
    render(<ProgressBar value={-3.4} label="Upload" />);
    expect(screen.getByRole('progressbar', { name: 'Upload' })).toHaveAttribute('aria-valuenow', '0');
  });
});

describe('OutcomeBar', () => {
  it('summarises segments and shows a legend', () => {
    render(
      <OutcomeBar
        label="Processing pipeline"
        segments={[
          { id: 'done', label: 'Done', value: 214, tone: 'ok' },
          { id: 'failed', label: 'Failed', value: 2, tone: 'danger' },
          { id: 'queued', label: 'Queued', value: 160, tone: 'neutral' },
        ]}
      />,
    );
    expect(screen.getByRole('img', { name: 'Processing pipeline: Done 214, Failed 2, Queued 160' })).toBeInTheDocument();
    expect(screen.getByText('214')).toBeInTheDocument();
  });
});
