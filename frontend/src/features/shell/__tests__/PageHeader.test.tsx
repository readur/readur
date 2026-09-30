import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { PageHeader } from '../PageHeader';

describe('PageHeader', () => {
  it('renders the title as the h1 with kicker, meta and actions', () => {
    render(
      <PageHeader
        title="Library"
        kicker="Documents"
        meta="1,204 documents"
        actions={<button type="button">Upload</button>}
        headingId="lib-title"
      />,
    );
    const h1 = screen.getByRole('heading', { level: 1, name: 'Library' });
    expect(h1).toHaveAttribute('id', 'lib-title');
    expect(screen.getByText('Documents')).toBeInTheDocument();
    expect(screen.getByText('1,204 documents')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Upload' })).toBeInTheDocument();
  });

  it('renders only the title when nothing else is given', () => {
    render(<PageHeader title="Board" />);
    expect(screen.getByRole('heading', { level: 1, name: 'Board' })).toBeInTheDocument();
    expect(screen.getAllByRole('heading')).toHaveLength(1);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('shows one headline figure beside the title when given', () => {
    render(<PageHeader title="Library" figure="660" />);
    expect(screen.getByText('660')).toBeInTheDocument();
  });
});
