import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { HumanReason } from '../shared/HumanReason';

const RAW = "OCR failed for '/app/uploads/scan-01.pdf': tesseract exited with status 1";

describe('HumanReason', () => {
  it('renders nothing for an empty message', () => {
    const { container } = render(<HumanReason raw="  " />);
    expect(container).toBeEmptyDOMElement();
  });

  it('gives a sentence and keeps the raw text behind Technical details', async () => {
    const user = userEvent.setup();
    render(<HumanReason raw={RAW} extra="/app/uploads/scan-01.pdf" />);
    expect(screen.getByText('OCR failed')).toBeInTheDocument();
    const summary = screen.getByText('Technical details');
    expect(screen.getByText(/tesseract exited/).closest('details')).not.toHaveAttribute('open');
    await user.click(summary);
    expect(screen.getByText(/tesseract exited/).closest('details')).toHaveAttribute('open');
  });

  it('shows only the sentence when asked', () => {
    render(<HumanReason raw={RAW} summaryOnly />);
    expect(screen.getByText('OCR failed')).toBeInTheDocument();
    expect(screen.queryByText('Technical details')).not.toBeInTheDocument();
  });

  it('falls back to the failure code when the text says nothing known', () => {
    render(<HumanReason raw="boom" code="file_too_large" summaryOnly />);
    expect(screen.getByText('The file is too large')).toBeInTheDocument();
  });
});
