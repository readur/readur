import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { StatusMark, type StatusState } from './StatusMark';

const WORDS: Array<[StatusState, string]> = [
  ['pending', 'Pending'],
  ['processing', 'OCR'],
  ['completed', 'Indexed'],
  ['failed', 'Failed'],
  ['healthy', 'Healthy'],
  ['syncing', 'Syncing'],
  ['warning', 'Check'],
  ['error', 'Error'],
  ['disabled', 'Off'],
];

describe('StatusMark', () => {
  it.each(WORDS)('renders %s with a decorative lead mark and its word', (state, word) => {
    const { container } = render(<StatusMark state={state} />);
    expect(screen.getByText(word)).toBeInTheDocument();
    const lead = container.querySelector('[data-lead]');
    expect(lead).toHaveAttribute('aria-hidden', 'true');
    // The text is the word only.
    expect(container.textContent).toBe(word);
  });

  it('spins for in-progress states and shows a bar with progress', () => {
    const { container } = render(<StatusMark state="processing" progress={{ current: 3, total: 12 }} />);
    expect(container.querySelector('[data-lead] svg')).not.toBeNull();
    expect(container.querySelector('[data-bar]')).not.toBeNull();
    expect(container).toHaveTextContent('OCR 3/12');
  });

  it('spins while syncing', () => {
    const { container } = render(<StatusMark state="syncing" />);
    expect(container.querySelector('[data-lead] svg')).not.toBeNull();
  });

  it('uses a dot for settled states', () => {
    const { container } = render(<StatusMark state="completed" />);
    expect(container.querySelector('[data-lead] svg')).toBeNull();
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
    expect(screen.getByText('Indexed')).toBeInTheDocument();
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
    expect(screen.getByRole('button', { name: 'Failed' })).toBeInTheDocument();
  });
});
