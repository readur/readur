import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import ServerSection from '../server/ServerSection';
import { apiMock, httpError, ok, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const CONFIG = {
  max_file_size_mb: 50,
  concurrent_ocr_jobs: 4,
  ocr_timeout_seconds: 300,
  memory_limit_mb: 512,
  cpu_priority: 'normal',
  server_host: '0.0.0.0',
  server_port: 8000,
  jwt_secret_set: true,
  upload_path: './uploads',
  watch_folder: './watch',
  ocr_language: 'eng',
  allowed_file_types: ['pdf', 'png'],
  watch_interval_seconds: 30,
  file_stability_check_ms: 500,
  max_file_age_hours: null,
  enable_background_ocr: false,
  version: '2.7.0',
  build_info: 'abc123',
};

const render = () => renderSettings(<ServerSection />, { path: '/settings/server' });

beforeEach(() => {
  apiMock.get.mockResolvedValue(ok(CONFIG));
});

describe('ServerSection', () => {
  it('summarises each area and reads /settings/config', async () => {
    render();
    expect(await screen.findByText('50 MB · ./uploads')).toBeInTheDocument();
    expect(screen.getByText('0.0.0.0:8000 · v2.7.0')).toBeInTheDocument();
    expect(apiMock.get).toHaveBeenCalledWith('/settings/config');
  });

  it('shows every configuration value when unfolded', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Edit File Upload Configuration' }));
    const upload = screen.getByRole('region', { name: 'File Upload Configuration' });
    expect(within(upload).getByText('./watch')).toBeInTheDocument();
    expect(within(upload).getByText('pdf')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Edit OCR Processing Configuration' }));
    const ocr = screen.getByRole('region', { name: 'OCR Processing Configuration' });
    expect(within(ocr).getByText('Disabled')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Edit Server Information' }));
    const info = screen.getByRole('region', { name: 'Server Information' });
    expect(within(info).getByText('Configured')).toBeInTheDocument();
    expect(within(info).getByText('abc123')).toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Edit Watch Folder Configuration' }));
    const watch = screen.getByRole('region', { name: 'Watch Folder Configuration' });
    expect(within(watch).getByText('500ms')).toBeInTheDocument();
    expect(within(watch).queryByText('Max File Age')).not.toBeInTheDocument();
  });

  it('refreshes on request', async () => {
    const user = userEvent.setup();
    render();
    await user.click(await screen.findByRole('button', { name: 'Refresh Configuration' }));
    await waitFor(() => expect(apiMock.get).toHaveBeenCalledTimes(2));
  });

  it('explains a forbidden load', async () => {
    apiMock.get.mockRejectedValue(httpError(403));
    render();
    expect(await screen.findByText('Admin access required to view server configuration')).toBeInTheDocument();
    expect(screen.getByText('Failed to load server configuration. Admin access may be required.')).toBeInTheDocument();
  });
});
