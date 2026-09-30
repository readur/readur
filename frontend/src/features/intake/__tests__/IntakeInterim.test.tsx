import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
import IntakeInterim from '../IntakeInterim';

vi.mock('../../../pages/UploadPage', () => ({ default: () => <p>upload page</p> }));
vi.mock('../../../pages/SourcesPage', () => ({ default: () => <p>sources page</p> }));
vi.mock('../../../pages/WatchFolderPage', () => ({ default: () => <p>watch folder page</p> }));
vi.mock('../../../pages/DocumentManagementPage', () => ({ default: () => <p>document management page</p> }));
vi.mock('../../../pages/IgnoredFilesPage', () => ({ default: () => <p>ignored files page</p> }));

const renderAt = (path: string) =>
  render(
    <MemoryRouter initialEntries={[path]}>
      <Routes>
        <Route path="/intake" element={<IntakeInterim />} />
      </Routes>
    </MemoryRouter>,
  );

describe('IntakeInterim', () => {
  it.each([
    ['upload', 'upload page', 'Upload'],
    ['connections', 'sources page', 'Connections'],
    ['watch', 'watch folder page', 'Watch folder'],
    ['attention', 'document management page', 'Needs attention'],
    ['ignored', 'ignored files page', 'Ignored files'],
  ])('section=%s renders the right page and marks its tab', (section, page, tab) => {
    renderAt(`/intake?section=${section}`);
    expect(screen.getByText(page)).toBeInTheDocument();
    const nav = screen.getByRole('navigation', { name: 'Intake sections' });
    expect(within(nav).getByRole('link', { name: tab })).toHaveAttribute('aria-current', 'page');
    expect(within(nav).getAllByRole('link').filter((l) => l.hasAttribute('aria-current'))).toHaveLength(1);
  });

  it.each(['/intake', '/intake?section=bogus'])('%s falls back to upload', (path) => {
    renderAt(path);
    expect(screen.getByText('upload page')).toBeInTheDocument();
  });

  it('has one h1 and switches section from its nav', async () => {
    const user = userEvent.setup();
    renderAt('/intake?section=upload');
    expect(screen.getByRole('heading', { level: 1, name: 'Intake' })).toBeInTheDocument();
    await user.click(screen.getByRole('link', { name: 'Connections' }));
    expect(screen.getByText('sources page')).toBeInTheDocument();
    expect(screen.queryByText('upload page')).not.toBeInTheDocument();
  });
});
