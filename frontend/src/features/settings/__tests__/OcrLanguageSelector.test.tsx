import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ocrService } from '../../../services/api';
import OcrLanguageSelector from '../ocr/OcrLanguageSelector';
import { httpError } from './settingsTestUtils';

const LANGS = {
  data: {
    available_languages: [
      { code: 'eng', name: 'English', installed: true },
      { code: 'deu', name: 'German', installed: true },
    ],
    current_user_language: 'deu',
  },
};

beforeEach(() => {
  vi.spyOn(ocrService, 'getAvailableLanguages').mockResolvedValue(LANGS as never);
});

describe('OcrLanguageSelector', () => {
  it('shows a loading state, then a labelled select', async () => {
    render(<OcrLanguageSelector value="eng" onChange={vi.fn()} />);
    expect(screen.getByRole('status')).toHaveTextContent('Loading languages...');
    expect(await screen.findByRole('combobox', { name: 'OCR Language' })).toHaveValue('eng');
    expect(screen.getByRole('option', { name: /German \(deu\) · Current/ })).toBeInTheDocument();
  });

  it('defaults to the current user language when no value is given', async () => {
    const onChange = vi.fn();
    render(<OcrLanguageSelector onChange={onChange} />);
    await waitFor(() => expect(onChange).toHaveBeenCalledWith('deu'));
  });

  it('reports a new choice and notes it will become the default', async () => {
    const onChange = vi.fn();
    const user = userEvent.setup();
    const { rerender } = render(<OcrLanguageSelector value="deu" onChange={onChange} label="Retry language" />);
    await user.selectOptions(await screen.findByRole('combobox', { name: 'Retry language' }), 'eng');
    expect(onChange).toHaveBeenCalledWith('eng');
    rerender(<OcrLanguageSelector value="eng" onChange={onChange} label="Retry language" />);
    expect(screen.getByText(/Selecting "English" will update your default language/)).toBeInTheDocument();
  });

  it('shows helper text as the description', async () => {
    render(<OcrLanguageSelector value="eng" onChange={vi.fn()} helperText="Leave empty to use your default" />);
    expect(await screen.findByRole('combobox')).toHaveAccessibleDescription('Leave empty to use your default');
  });

  it('falls back to English with a retry when languages fail to load', async () => {
    vi.mocked(ocrService.getAvailableLanguages).mockRejectedValueOnce(httpError(500, { message: 'tesseract missing' }));
    const onChange = vi.fn();
    const user = userEvent.setup();
    render(<OcrLanguageSelector onChange={onChange} />);
    expect(await screen.findByRole('alert')).toHaveTextContent('tesseract missing');
    expect(screen.getByRole('combobox')).toBeDisabled();
    expect(onChange).toHaveBeenCalledWith('eng');
    await user.click(screen.getByRole('button', { name: 'Retry' }));
    expect(await screen.findByRole('option', { name: /German/ })).toBeInTheDocument();
  });
});
