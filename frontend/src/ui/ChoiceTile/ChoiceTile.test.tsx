import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it } from 'vitest';
import { ChoiceGroup, ChoiceTile } from './ChoiceTile';

describe('ChoiceTile', () => {
  it('is a labelled checkbox group of tiles', async () => {
    render(
      <ChoiceGroup label="OCR options" defaultValue={['ocr']}>
        <ChoiceTile value="ocr" label="Run OCR on upload" description="Extract text as soon as a file arrives" />
        <ChoiceTile value="rotate" label="Auto-rotate pages" />
      </ChoiceGroup>,
    );
    expect(screen.getByRole('group', { name: 'OCR options' })).toBeInTheDocument();
    const rotate = screen.getByRole('checkbox', { name: 'Auto-rotate pages' });
    expect(screen.getByRole('checkbox', { name: /Run OCR on upload/ })).toBeChecked();
    await userEvent.click(rotate);
    expect(rotate).toBeChecked();
  });
});
