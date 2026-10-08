import { screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../intake/connections/ConnectionsSection', () => ({ ConnectionsSection: () => <p>connections section</p> }));
vi.mock('../../intake/watch/WatchSection', () => ({ WatchSection: () => <p>watch section</p> }));
vi.mock('../../../services/api', async () => (await import('../../intake/__tests__/intakeMocks')).apiModule);

import SourcesPage, { parseSourcesSection } from '../SourcesPage';
import { renderIntake, resetIntakeState, settle, source } from '../../intake/__tests__/intakeTestUtils';
import { ok, serveDefaults, sourcesService } from '../../intake/__tests__/intakeMocks';

const location = () => screen.getByRole('status', { name: 'location', hidden: true }).textContent;

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  sourcesService.list.mockImplementation(() => ok([source('a'), source('b')]));
});

describe('Sources page', () => {
  it.each([
    ['connections', 'connections section', 'Connections'],
    ['watch', 'watch section', 'Watch folder'],
  ])('?section=%s shows its section and selects its tab', async (section, text, tab) => {
    renderIntake(<SourcesPage />, { path: `/sources?section=${section}` });
    await settle();
    expect(screen.getByText(text)).toBeInTheDocument();
    const tabs = screen.getByRole('tablist', { name: 'Source sections' });
    expect(within(tabs).getByRole('tab', { name: tab })).toHaveAttribute('aria-selected', 'true');
  });

  it('defaults to Connections and titles the page Sources with the connection count', async () => {
    renderIntake(<SourcesPage />, { path: '/sources' });
    await settle();
    expect(screen.getByText('connections section')).toBeInTheDocument();
    const h1 = screen.getByRole('heading', { level: 1, name: 'Sources' });
    expect(within(h1.parentElement as HTMLElement).getByText('2 connections')).toBeInTheDocument();
  });

  it('writes the chosen tab to ?section=', async () => {
    const user = userEvent.setup();
    renderIntake(<SourcesPage />, { path: '/sources?section=connections&source=a' });
    await settle();
    await user.click(screen.getByRole('tab', { name: 'Watch folder' }));
    expect(location()).toBe('/sources?section=watch');
    await settle();
  });

  it('sends the watch folder key to its own tab', async () => {
    renderIntake(<SourcesPage />, { path: '/sources?source=watch' });
    await settle();
    expect(screen.getByText('watch section')).toBeInTheDocument();
  });

  it('parses unknown sections as connections', () => {
    expect(parseSourcesSection('watch')).toBe('watch');
    expect(parseSourcesSection(null)).toBe('connections');
    expect(parseSourcesSection('upload')).toBe('connections');
  });
});
