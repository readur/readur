import { useState } from 'react';
import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import i18next from 'i18next';
import { I18nextProvider, initReactI18next } from 'react-i18next';
import { BulkActionBar } from './BulkActionBar';

function setup(count = 3) {
  const onDownload = vi.fn();
  const onDelete = vi.fn();
  const onClear = vi.fn();
  const utils = render(
    <BulkActionBar
      count={count}
      onClear={onClear}
      actions={[
        { id: 'download', label: 'Download', onPress: onDownload },
        { id: 'delete', label: 'Delete', tone: 'danger', onPress: onDelete },
      ]}
    />,
  );
  return { ...utils, onDownload, onDelete, onClear };
}

describe('BulkActionBar', () => {
  it('renders a named toolbar with the count and actions', () => {
    setup(3);
    const bar = screen.getByRole('toolbar', { name: 'Bulk actions' });
    expect(bar).toHaveTextContent('3 selected');
    expect(screen.getByRole('status')).toHaveTextContent('3 selected');
    expect(within(bar).getByRole('button', { name: 'Download' })).toBeInTheDocument();
    expect(within(bar).getByRole('button', { name: 'Delete' })).toBeInTheDocument();
    expect(within(bar).getByRole('button', { name: 'Clear selection' })).toBeInTheDocument();
  });

  it('is hidden when nothing is selected', () => {
    setup(0);
    expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
  });

  it('keeps its live region mounted at zero so the first selection is announced', () => {
    const props = { actions: [], onClear: () => {} };
    const { rerender } = render(<BulkActionBar count={0} {...props} />);
    const status = screen.getByRole('status');
    expect(status).toHaveAttribute('aria-live', 'polite');
    expect(status).toBeEmptyDOMElement();
    rerender(<BulkActionBar count={1} {...props} />);
    expect(screen.getByRole('status')).toBe(status);
    expect(status).toHaveTextContent('1 selected');
    rerender(<BulkActionBar count={0} {...props} />);
    expect(screen.getByRole('status')).toBe(status);
    expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
  });

  it('moves between actions with arrow keys and activates with Enter', async () => {
    const user = userEvent.setup();
    const { onDelete, onClear } = setup(2);
    await user.tab();
    expect(screen.getByRole('button', { name: 'Download' })).toHaveFocus();
    await user.keyboard('{ArrowRight}');
    expect(screen.getByRole('button', { name: 'Delete' })).toHaveFocus();
    await user.keyboard('{Enter}');
    expect(onDelete).toHaveBeenCalledTimes(1);
    await user.keyboard('{ArrowRight}');
    await user.keyboard('{Enter}');
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it('accepts a custom accessible name', () => {
    render(<BulkActionBar aria-label="Document actions" count={1} actions={[]} onClear={() => {}} />);
    expect(screen.getByRole('toolbar', { name: 'Document actions' })).toBeInTheDocument();
  });

  it('renders the count as one translated sentence, in the order the language uses', async () => {
    const i18n = i18next.createInstance();
    await i18n.use(initReactI18next).init({
      lng: 'xx',
      resources: { xx: { translation: { ui: { bulk: { count_one: '<w>Picked:</w> <n>{{count}}</n> file', count_other: '<w>Picked:</w> <n>{{count}}</n> files' } } } } },
      interpolation: { escapeValue: false },
    });
    render(
      <I18nextProvider i18n={i18n}>
        <BulkActionBar count={2} actions={[]} onClear={() => {}} />
      </I18nextProvider>,
    );
    expect(screen.getByRole('status')).toHaveTextContent('Picked: 2 files');
    expect(screen.getByRole('toolbar')).toHaveTextContent('Picked: 2 files');
  });

  it('moves focus to the page heading when "Clear selection" removes the bar', async () => {
    const user = userEvent.setup();
    function Page() {
      const [count, setCount] = useState(2);
      return (
        <main>
          <h1>Documents</h1>
          <BulkActionBar count={count} actions={[]} onClear={() => setCount(0)} />
        </main>
      );
    }
    render(<Page />);
    screen.getByRole('button', { name: 'Clear selection' }).focus();
    await user.keyboard('{Enter}');
    expect(screen.queryByRole('toolbar')).not.toBeInTheDocument();
    expect(screen.getByRole('heading', { level: 1, name: 'Documents' })).toHaveFocus();
  });
});
