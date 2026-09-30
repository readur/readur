import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { StatusMark, type StatusState } from './StatusMark';

const WORDS: Array<[StatusState, string, string]> = [
  ['pending', '○', 'PENDING'],
  ['processing', '◐', 'OCR'],
  ['completed', '■', 'INDEXED'],
  ['failed', '▲', 'FAILED'],
  ['healthy', '■', 'HEALTHY'],
  ['syncing', '◐', 'SYNCING'],
  ['warning', '◆', 'CHECK'],
  ['error', '▲', 'ERROR'],
  ['disabled', '—', 'OFF'],
];

describe('StatusMark', () => {
  it.each(WORDS)('renders %s with its glyph and word', (state, glyph, word) => {
    const { container } = render(<StatusMark state={state} />);
    expect(screen.getByText(word)).toBeInTheDocument();
    const mark = screen.getByText(glyph);
    expect(mark).toHaveAttribute('aria-hidden', 'true');
    // The accessible text is the word only.
    expect(container.textContent).toBe(`${glyph}${word}`);
  });

  it.each([
    ['completed', 'ok'],
    ['processing', 'active'],
    ['pending', 'neutral'],
    ['failed', 'danger'],
    ['warning', 'warn'],
  ] as const)('tints %s as %s', (state, tone) => {
    const { container } = render(<StatusMark state={state} />);
    expect(container.firstElementChild).toHaveAttribute('data-tone', tone);
  });

  it('renders known progress as OCR n/m', () => {
    render(<StatusMark state="processing" progress={{ current: 3, total: 12 }} />);
    expect(screen.getByText('OCR 3/12')).toBeInTheDocument();
  });

  it('ignores progress for states other than processing', () => {
    render(<StatusMark state="completed" progress={{ current: 3, total: 12 }} />);
    expect(screen.getByText('INDEXED')).toBeInTheDocument();
    expect(screen.queryByText(/3\/12/)).not.toBeInTheDocument();
  });

  it('is not an interactive or image role and adds no tab stop', async () => {
    const user = userEvent.setup();
    render(
      <>
        <StatusMark state="failed" />
        <button type="button">after</button>
      </>,
    );
    expect(screen.queryByRole('img')).not.toBeInTheDocument();
    await user.tab();
    expect(screen.getByRole('button', { name: 'after' })).toHaveFocus();
  });

  it('contributes its word to the accessible name of a container', () => {
    render(
      <button type="button">
        <StatusMark state="failed" />
      </button>,
    );
    expect(screen.getByRole('button', { name: 'FAILED' })).toBeInTheDocument();
  });
});
