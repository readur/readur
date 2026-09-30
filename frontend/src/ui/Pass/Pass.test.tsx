import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { Pass, PassCell } from './Pass';

const scrollWidth = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'scrollWidth');
const clientWidth = Object.getOwnPropertyDescriptor(HTMLElement.prototype, 'clientWidth');

function restoreWidths() {
  if (scrollWidth) Object.defineProperty(HTMLElement.prototype, 'scrollWidth', scrollWidth);
  if (clientWidth) Object.defineProperty(HTMLElement.prototype, 'clientWidth', clientWidth);
}

describe('Pass', () => {
  afterEach(restoreWidths);

  it('renders every label with its value as a description list', () => {
    render(
      <Pass aria-label="Document details">
        <PassCell label="Name">invoice-2024.pdf</PassCell>
        <PassCell label="Pages" mono>
          12
        </PassCell>
        <PassCell label="Size" mono>
          1.2 MB
        </PassCell>
      </Pass>,
    );
    const list = screen.getByRole('group', { name: 'Document details' });
    expect(within(list).getAllByRole('term').map((el) => el.textContent)).toEqual(['Name', 'Pages', 'Size']);
    expect(within(list).getAllByRole('definition').map((el) => el.textContent)).toEqual([
      'invoice-2024.pdf',
      '12',
      '1.2 MB',
    ]);
  });

  it('renders node values as-is', () => {
    render(
      <Pass>
        <PassCell label="Status">
          <strong>Ready</strong>
        </PassCell>
      </Pass>,
    );
    expect(screen.getByText('Ready').tagName).toBe('STRONG');
  });

  it('supports the header variant', () => {
    render(
      <Pass variant="header" aria-label="Summary">
        <PassCell label="Documents" mono>
          1,204
        </PassCell>
      </Pass>,
    );
    expect(screen.getByRole('term')).toHaveTextContent('Documents');
    expect(screen.getByRole('definition')).toHaveTextContent('1,204');
  });

  it('adds no tab stop when values fit', async () => {
    const user = userEvent.setup();
    render(
      <>
        <Pass>
          <PassCell label="Name">short</PassCell>
        </Pass>
        <button type="button">next</button>
      </>,
    );
    await user.tab();
    expect(screen.getByRole('button', { name: 'next' })).toHaveFocus();
  });

  it('shows the full value in a tooltip when a value is truncated and focused', async () => {
    Object.defineProperty(HTMLElement.prototype, 'scrollWidth', { configurable: true, get: () => 300 });
    Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => 100 });
    const user = userEvent.setup();
    const long = 'Rechnungsverarbeitungsdokument-2024-final.pdf';
    render(
      <Pass>
        <PassCell label="Name">{long}</PassCell>
      </Pass>,
    );
    await user.tab();
    expect(screen.getByText(long, { selector: 'span' })).toHaveFocus();
    expect(await screen.findByRole('tooltip')).toHaveTextContent(long);
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();
  });
});
