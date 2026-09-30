import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../upload/UploadSection', () => ({ UploadSection: () => <p>upload section</p> }));
vi.mock('../connections/ConnectionsSection', () => ({ ConnectionsSection: () => <p>connections section</p> }));
vi.mock('../watch/WatchSection', () => ({ WatchSection: () => <p>watch section</p> }));
vi.mock('../attention/AttentionSection', () => ({ AttentionSection: () => <p>attention section</p> }));
vi.mock('../ignored/IgnoredSection', () => ({ IgnoredSection: () => <p>ignored section</p> }));
vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import IntakePage, { parseSection } from '../IntakePage';
import { failedList, renderIntake, resetIntakeState, settle, source, failedDoc } from './intakeTestUtils';
import { ocrService, ok, queueService, serveDefaults, sourcesService } from './intakeMocks';

const location = () => screen.getByRole('status', { name: 'location', hidden: true }).textContent;

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  sourcesService.list.mockImplementation(() => ok([source('a'), source('b'), source('c')]));
  ocrService.listFailedDocuments.mockImplementation(() => ok(failedList([failedDoc('f1'), failedDoc('f2')])));
  queueService.getStats.mockImplementation(() => ok({ pending_count: 10, processing_count: 4, failed_count: 0, completed_today: 1 }));
});

describe('Intake page', () => {
  it.each([
    ['upload', 'upload section', 'Add documents'],
    ['connections', 'connections section', 'Connections'],
    ['watch', 'watch section', 'Watch folder'],
    ['attention', 'attention section', /Needs attention/],
    ['ignored', 'ignored section', 'Ignored'],
  ])('?section=%s shows its section and selects its tab', async (section, text, tab) => {
    renderIntake(<IntakePage />, { path: `/intake?section=${section}` });
    await settle();
    expect(screen.getByText(text)).toBeInTheDocument();
    const tabs = screen.getByRole('tablist', { name: 'Intake sections' });
    expect(within(tabs).getByRole('tab', { name: tab })).toHaveAttribute('aria-selected', 'true');
    expect(within(tabs).getAllByRole('tab').filter((t) => t.getAttribute('aria-selected') === 'true')).toHaveLength(1);
  });

  it.each(['/intake', '/intake?section=bogus'])('%s falls back to Add documents', async (path) => {
    renderIntake(<IntakePage />, { path });
    await settle();
    expect(screen.getByText('upload section')).toBeInTheDocument();
  });

  it('has one h1 and writes the chosen tab to ?section=', async () => {
    const user = userEvent.setup();
    renderIntake(<IntakePage />, { path: '/intake?section=upload' });
    await settle();
    expect(screen.getAllByRole('heading', { level: 1 })).toHaveLength(1);
    expect(screen.getByRole('heading', { level: 1, name: 'Intake' })).toBeInTheDocument();
    await user.click(screen.getByRole('tab', { name: 'Connections' }));
    expect(screen.getByText('connections section')).toBeInTheDocument();
    expect(screen.queryByText('upload section')).not.toBeInTheDocument();
    expect(location()).toBe('/intake?section=connections');
    await settle();
  });

  it('switches tabs with the arrow keys and keeps the URL in step', async () => {
    const user = userEvent.setup();
    renderIntake(<IntakePage />, { path: '/intake?section=watch' });
    await settle();
    screen.getByRole('tab', { name: 'Watch folder' }).focus();
    await user.keyboard('{ArrowRight}');
    expect(location()).toBe('/intake?section=attention');
    expect(screen.getByText('attention section')).toBeInTheDocument();
    await settle();
  });

  it('drops section-specific filters when switching sections', async () => {
    const user = userEvent.setup();
    renderIntake(<IntakePage />, { path: '/intake?section=ignored&sourceId=s1' });
    await settle();
    await user.click(screen.getByRole('tab', { name: 'Watch folder' }));
    expect(location()).toBe('/intake?section=watch');
    await settle();
  });

  it('summarizes connections, attention and processing in the header', async () => {
    renderIntake(<IntakePage />);
    await settle();
    expect(await screen.findByText('3 connections · 2 need attention · 14 processing')).toBeInTheDocument();
  });

  it('badges the Needs attention tab with the count', async () => {
    renderIntake(<IntakePage />);
    await settle();
    const tab = screen.getByRole('tab', { name: /Needs attention/ });
    await waitFor(() => expect(tab).toHaveAccessibleName(/^Needs attention, ?2$/));
  });

  it('shows no badge and keeps the rest of the header when a count fails', async () => {
    ocrService.listFailedDocuments.mockImplementation(() => Promise.reject(new Error('down')));
    renderIntake(<IntakePage />);
    await settle();
    expect(await screen.findByText('3 connections · 14 processing')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Needs attention' })).toBeInTheDocument();
  });

  it('parses unknown sections as upload', async () => {
    expect(parseSection('connections')).toBe('connections');
    expect(parseSection(null)).toBe('upload');
    expect(parseSection('nope')).toBe('upload');
  });
});
