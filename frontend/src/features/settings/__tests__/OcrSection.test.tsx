import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ocrService, queueService } from '../../../services/api';
import OcrSection from '../ocr/OcrSection';
import { apiMock, httpError, ok, plainUser, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const SERVER_SETTINGS = {
  preferred_languages: ['eng', 'deu'],
  primary_language: 'eng',
  ocr_language: 'eng',
  auto_detect_language_combination: false,
  concurrent_ocr_jobs: 4,
  ocr_timeout_seconds: 300,
  cpu_priority: 'normal',
  ocr_skip_enhancement: false,
  ocr_brightness_boost: 0,
  ocr_contrast_multiplier: 1,
  ocr_noise_reduction_level: 0,
  ocr_sharpening_strength: 0,
  ocr_quality_threshold_brightness: 40,
  ocr_quality_threshold_contrast: 0.15,
  ocr_quality_threshold_noise: 0.3,
  ocr_quality_threshold_sharpness: 0.15,
  ocr_morphological_operations: true,
  ocr_histogram_equalization: false,
  save_processed_images: false,
  ocr_adaptive_threshold_window_size: 15,
  ocr_max_image_width: 10000,
  ocr_max_image_height: 10000,
  ocr_upscale_factor: 1,
};

const LANGUAGES = {
  data: {
    available_languages: [
      { code: 'eng', name: 'English', installed: true },
      { code: 'deu', name: 'German', installed: true },
      { code: 'fra', name: 'French', installed: true },
    ],
    current_user_language: 'eng',
  },
};

async function openGroup(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(await screen.findByRole('button', { name: `Edit ${name}` }));
  return screen.getByRole('region', { name });
}

beforeEach(() => {
  apiMock.get.mockResolvedValue(ok(SERVER_SETTINGS));
  apiMock.put.mockResolvedValue(ok({}));
  vi.spyOn(ocrService, 'getAvailableLanguages').mockResolvedValue(LANGUAGES as never);
  vi.spyOn(queueService, 'getOcrStatus').mockResolvedValue({ data: { is_paused: false, status: 'running' } } as never);
  vi.spyOn(queueService, 'pauseOcr').mockResolvedValue({ data: {} } as never);
  vi.spyOn(queueService, 'resumeOcr').mockResolvedValue({ data: {} } as never);
});

describe('OcrSection', () => {
  it('summarises languages and processing in one line each', async () => {
    renderSettings(<OcrSection />);
    expect(await screen.findByText('ENG, DEU · auto-detect off')).toBeInTheDocument();
    expect(screen.getByText('4 jobs · 300s · normal priority')).toBeInTheDocument();
    expect(screen.getByText('brightness 0 · contrast 1× · noise none · sharpen 0')).toBeInTheDocument();
    expect(screen.getByText('10000 × 10000 px · upscale 1×')).toBeInTheDocument();
  });

  it('renders every OCR field from the previous page', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const processing = await openGroup(user, 'Processing');
    expect(within(processing).getByRole('spinbutton', { name: 'Concurrent OCR Jobs' })).toHaveValue(4);
    expect(within(processing).getByRole('spinbutton', { name: 'OCR Timeout (seconds)' })).toHaveValue(300);
    expect(within(processing).getByRole('button', { name: /CPU Priority/ })).toHaveTextContent('Normal');

    const enhancement = await openGroup(user, 'Enhancement Controls');
    expect(within(enhancement).getByRole('switch', { name: /Skip All Image Enhancement/ })).not.toBeChecked();
    for (const name of ['Brightness Boost', 'Contrast Multiplier', 'Sharpening Strength']) {
      expect(within(enhancement).getByRole('spinbutton', { name })).toBeInTheDocument();
    }
    expect(within(enhancement).getByRole('button', { name: /Noise Reduction Level/ })).toHaveTextContent('None');

    const thresholds = await openGroup(user, 'Quality thresholds');
    for (const name of ['Brightness Threshold', 'Contrast Threshold', 'Noise Threshold', 'Sharpness Threshold']) {
      expect(within(thresholds).getByRole('spinbutton', { name })).toBeInTheDocument();
    }

    const advanced = await openGroup(user, 'Advanced Processing Options');
    expect(within(advanced).getByRole('switch', { name: /Morphological Operations/ })).toBeChecked();
    expect(within(advanced).getByRole('switch', { name: /Histogram Equalization/ })).not.toBeChecked();
    expect(within(advanced).getByRole('switch', { name: /Save Processed Images/ })).not.toBeChecked();
    expect(within(advanced).getByRole('spinbutton', { name: 'Adaptive Threshold Window Size' })).toHaveValue(15);

    const size = await openGroup(user, 'Image Size and Scaling');
    for (const name of ['Max Image Width', 'Max Image Height', 'Upscale Factor']) {
      expect(within(size).getByRole('spinbutton', { name })).toBeInTheDocument();
    }

    const languages = await openGroup(user, 'Languages');
    expect(within(languages).getByRole('switch', { name: /Auto-detect language combinations/ })).not.toBeChecked();
    expect(await within(languages).findByText('OCR Languages (2/4)')).toBeInTheDocument();
  });

  it('saves languages together with the primary and legacy language', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Languages');
    await user.click(await within(region).findByRole('button', { name: /Add more languages/ }));
    await user.click(within(region).getByRole('checkbox', { name: 'French' }));
    await user.click(within(region).getByRole('button', { name: 'Set Primary French' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', {
        preferred_languages: ['eng', 'deu', 'fra'],
        primary_language: 'fra',
        ocr_language: 'fra',
      }),
    );
    expect(await screen.findByText('ENG, DEU, FRA · auto-detect off')).toBeInTheDocument();
  });

  it('discards a language draft on Cancel', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Languages');
    await user.click(await within(region).findByRole('button', { name: 'Remove German' }));
    expect(within(region).getByText('OCR Languages (1/4)')).toBeInTheDocument();
    await user.click(within(region).getByRole('button', { name: 'Cancel' }));
    expect(within(region).getByText('OCR Languages (2/4)')).toBeInTheDocument();
    expect(apiMock.put).not.toHaveBeenCalled();
  });

  it('saves processing values and validates their ranges', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Processing');
    const jobs = within(region).getByRole('spinbutton', { name: 'Concurrent OCR Jobs' });
    await user.clear(jobs);
    await user.type(jobs, '17');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(await within(region).findByText('Enter a value from 1 to 16.')).toBeInTheDocument();
    expect(apiMock.put).not.toHaveBeenCalled();
    await user.clear(jobs);
    await user.type(jobs, '8');
    await user.click(within(region).getByRole('button', { name: /CPU Priority/ }));
    await user.click(await screen.findByRole('option', { name: 'High' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', { concurrent_ocr_jobs: 8, cpu_priority: 'high' }),
    );
  });

  it('requires an odd adaptive threshold window', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Advanced Processing Options');
    const win = within(region).getByRole('spinbutton', { name: 'Adaptive Threshold Window Size' });
    await user.clear(win);
    await user.type(win, '16');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(await within(region).findByText('Enter an odd number.')).toBeInTheDocument();
    await user.clear(win);
    await user.type(win, '21');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/settings', { ocr_adaptive_threshold_window_size: 21 }));
  });

  it('saves a noise-reduction level of zero as 0', async () => {
    apiMock.get.mockResolvedValue(ok({ ...SERVER_SETTINGS, ocr_noise_reduction_level: 2 }));
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Enhancement Controls');
    await user.click(within(region).getByRole('button', { name: /Noise Reduction Level/ }));
    await user.click(await screen.findByRole('option', { name: 'None' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/settings', { ocr_noise_reduction_level: 0 }));
  });

  it('saves the skip-enhancement switch on change', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Enhancement Controls');
    await user.click(within(region).getByRole('switch', { name: /Skip All Image Enhancement/ }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/settings', { ocr_skip_enhancement: true }));
    expect(await screen.findByText('skipped · original images only')).toBeInTheDocument();
  });

  it('lets an admin pause OCR and shows the paused state', async () => {
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Processing controls');
    expect(await within(region).findByText('OCR Status: RUNNING')).toBeInTheDocument();
    vi.mocked(queueService.getOcrStatus).mockResolvedValue({ data: { is_paused: true, status: 'paused' } } as never);
    await user.click(within(region).getByRole('button', { name: 'Pause OCR Processing' }));
    expect(queueService.pauseOcr).toHaveBeenCalled();
    expect(await screen.findByText('OCR processing paused successfully')).toBeInTheDocument();
    expect(await within(region).findByText('OCR Processing Paused')).toBeInTheDocument();
    await user.click(within(region).getByRole('button', { name: 'Resume OCR Processing' }));
    expect(queueService.resumeOcr).toHaveBeenCalled();
  });

  it('explains a forbidden pause', async () => {
    vi.mocked(queueService.pauseOcr).mockRejectedValue(httpError(403));
    const user = userEvent.setup();
    renderSettings(<OcrSection />);
    const region = await openGroup(user, 'Processing controls');
    await user.click(await within(region).findByRole('button', { name: 'Pause OCR Processing' }));
    expect(await screen.findByText('Admin access required to pause OCR processing')).toBeInTheDocument();
  });

  it('hides the pause controls from non-admins and never asks for the queue status', async () => {
    renderSettings(<OcrSection />, { user: plainUser });
    await screen.findByText('ENG, DEU · auto-detect off');
    expect(screen.queryByRole('button', { name: 'Edit Processing controls' })).not.toBeInTheDocument();
    expect(queueService.getOcrStatus).not.toHaveBeenCalled();
  });
});
