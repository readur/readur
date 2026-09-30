import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../upload/UploadSection', () => ({ UploadSection: () => <p>upload section</p> }));
vi.mock('../connections/ConnectionsSection', () => ({ ConnectionsSection: () => <p>connections section</p> }));
vi.mock('../watch/WatchSection', () => ({ WatchSection: () => <p>watch section</p> }));
vi.mock('../attention/AttentionSection', () => ({ AttentionSection: () => <p>attention section</p> }));
vi.mock('../ignored/IgnoredSection', () => ({ IgnoredSection: () => <p>ignored section</p> }));
vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import IntakePage, { parseSection, revealSelectedTab } from '../IntakePage';
import { ocrDoc, ocrList, renderIntake, resetIntakeState, settle, source } from './intakeTestUtils';
import { documentService, ok, queueService, serveDefaults, sourcesService } from './intakeMocks';

const location = () => screen.getByRole('status', { name: 'location', hidden: true }).textContent;

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  sourcesService.list.mockImplementation(() => ok([source('a'), source('b'), source('c')]));
  documentService.getFailedOcrDocuments.mockImplementation(() => ok(ocrList([ocrDoc('f1'), ocrDoc('f2')])));
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

  it('shows the needs-attention count as the headline figure, else the connections count', async () => {
    renderIntake(<IntakePage />);
    await settle();
    const h1 = screen.getByRole('heading', { level: 1, name: 'Intake' });
    expect(within(h1.parentElement as HTMLElement).getByText('2 need attention')).toBeInTheDocument();
  });

  it('falls back to the connections count when nothing needs attention', async () => {
    documentService.getFailedOcrDocuments.mockImplementation(() => ok(ocrList([])));
    renderIntake(<IntakePage />);
    await settle();
    const h1 = screen.getByRole('heading', { level: 1, name: 'Intake' });
    expect(within(h1.parentElement as HTMLElement).getByText('3 connections')).toBeInTheDocument();
  });

  it('badges the Needs attention tab with the count', async () => {
    renderIntake(<IntakePage />);
    await settle();
    const tab = screen.getByRole('tab', { name: /Needs attention/ });
    await waitFor(() => expect(tab).toHaveAccessibleName(/^Needs attention, ?2$/));
  });

  it('shows no badge and keeps the rest of the header when a count fails', async () => {
    documentService.getFailedOcrDocuments.mockImplementation(() => Promise.reject(new Error('down')));
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

describe('revealSelectedTab', () => {
  const rect = (left: number, width: number) =>
    ({ left, right: left + width, width, top: 0, bottom: 40, height: 40, x: left, y: 0, toJSON: () => ({}) }) as DOMRect;

  function tabRow(tabLeft: number, tabWidth: number) {
    const scroller = document.createElement('div');
    const tab = document.createElement('div');
    tab.setAttribute('role', 'tab');
    tab.setAttribute('aria-selected', 'true');
    scroller.append(tab);
    scroller.getBoundingClientRect = () => rect(16, 358);
    tab.getBoundingClientRect = () => rect(tabLeft, tabWidth);
    scroller.scrollLeft = 100;
    return scroller;
  }

  it('scrolls right until a tab cut off at the right edge is fully visible', () => {
    const scroller = tabRow(352, 120);
    revealSelectedTab(scroller);
    expect(scroller.scrollLeft).toBe(100 + (352 + 120 - 374));
  });

  it('scrolls left for a tab cut off at the left edge', () => {
    const scroller = tabRow(-20, 100);
    revealSelectedTab(scroller);
    expect(scroller.scrollLeft).toBe(100 - 36);
  });

  it('leaves a visible tab alone, and does nothing without a row', () => {
    const scroller = tabRow(40, 100);
    revealSelectedTab(scroller);
    expect(scroller.scrollLeft).toBe(100);
    expect(() => revealSelectedTab(null)).not.toThrow();
  });
});
