import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import GeneralSection from '../general/GeneralSection';
import { apiMock, httpError, ok, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) =>
  (await import('./apiMock')).mockApiModule(importOriginal),
);

const SERVER_SETTINGS = {
  max_file_size_mb: 80,
  auto_rotate_images: false,
  enable_image_preprocessing: true,
  enable_background_ocr: true,
  fuzzy_search_threshold: 0.6,
};

async function openGroup(user: ReturnType<typeof userEvent.setup>, name: string) {
  await user.click(await screen.findByRole('button', { name: `Edit ${name}` }));
  return screen.getByRole('region', { name });
}

beforeEach(() => {
  apiMock.get.mockResolvedValue(ok(SERVER_SETTINGS));
  apiMock.put.mockResolvedValue(ok({}));
});

describe('GeneralSection', () => {
  it('loads settings and summarises each group', async () => {
    renderSettings(<GeneralSection />);
    expect(await screen.findByText('80 MB max · rotate off')).toBeInTheDocument();
    expect(screen.getByText('fuzzy 0.6')).toBeInTheDocument();
    expect(apiMock.get).toHaveBeenCalledWith('/settings');
  });

  it('renders every file and search field', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const file = await openGroup(user, 'File Processing');
    expect(within(file).getByRole('spinbutton', { name: 'Max File Size (MB)' })).toHaveValue(80);
    expect(within(file).getByRole('switch', { name: /Auto-rotate Images/ })).not.toBeChecked();
    expect(within(file).getByRole('switch', { name: /Enable Image Preprocessing/ })).toBeChecked();
    expect(within(file).getByRole('switch', { name: /Enable Background OCR/ })).toBeChecked();
    expect(
      within(file).getByText(/Enabling preprocessing can significantly alter/),
    ).toBeInTheDocument();
    const search = await openGroup(user, 'Search Configuration');
    expect(within(search).getByRole('spinbutton', { name: 'Fuzzy Search Threshold' })).toHaveValue(
      0.6,
    );
  });

  it('no longer offers the settings that never had an effect', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const file = await openGroup(user, 'File Processing');
    expect(
      within(file).queryByRole('spinbutton', { name: 'Memory Limit (MB)' }),
    ).not.toBeInTheDocument();
    const search = await openGroup(user, 'Search Configuration');
    expect(
      within(search).queryByRole('button', { name: /Results Per Page/ }),
    ).not.toBeInTheDocument();
    expect(
      within(search).queryByRole('spinbutton', { name: 'Snippet Length' }),
    ).not.toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Edit Storage Management' }),
    ).not.toBeInTheDocument();
    expect(screen.queryByText(/Retention|Auto Cleanup|Compression/)).not.toBeInTheDocument();
  });

  it('shows Save and Cancel only while the form is dirty, and Cancel restores the saved value', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
    const size = within(region).getByRole('spinbutton', { name: 'Max File Size (MB)' });
    await user.clear(size);
    await user.type(size, '120');
    expect(within(region).getByRole('button', { name: 'Save' })).toBeInTheDocument();
    await user.click(within(region).getByRole('button', { name: 'Cancel' }));
    expect(size).toHaveValue(80);
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
    expect(apiMock.put).not.toHaveBeenCalled();
  });

  it('hides Save again when the value is typed back to the saved one', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    const size = within(region).getByRole('spinbutton', { name: 'Max File Size (MB)' });
    await user.type(size, '0');
    expect(within(region).getByRole('button', { name: 'Save' })).toBeInTheDocument();
    await user.type(size, '{Backspace}');
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('saves only the changed fields with API names and numeric values, then toasts', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    const size = within(region).getByRole('spinbutton', { name: 'Max File Size (MB)' });
    await user.clear(size);
    await user.type(size, '200');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', { max_file_size_mb: 200 }),
    );
    expect(await screen.findByText('Settings updated successfully')).toBeInTheDocument();
    expect(await screen.findByText('200 MB max · rotate off')).toBeInTheDocument();
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('saves a decimal', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Search Configuration');
    const fuzzy = within(region).getByRole('spinbutton', { name: 'Fuzzy Search Threshold' });
    await user.clear(fuzzy);
    await user.type(fuzzy, '0.9');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', { fuzzy_search_threshold: 0.9 }),
    );
    expect(await screen.findByText('fuzzy 0.9')).toBeInTheDocument();
  });

  it.each([
    ['Max File Size (MB)', 'File Processing', '501', 'Enter a value from 1 to 500.'],
    ['Max File Size (MB)', 'File Processing', '0', 'Enter a value from 1 to 500.'],
    ['Max File Size (MB)', 'File Processing', '2.5', 'Enter a whole number.'],
    ['Fuzzy Search Threshold', 'Search Configuration', '1.5', 'Enter a value from 0 to 1.'],
  ])('rejects %s = %s inline without saving', async (label, group, value, message) => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, group);
    const input = within(region).getByRole('spinbutton', { name: label });
    await user.clear(input);
    await user.type(input, value);
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(await within(region).findByText(message)).toBeInTheDocument();
    expect(input).toHaveAttribute('aria-invalid', 'true');
    expect(apiMock.put).not.toHaveBeenCalled();
  });

  it('saves an empty max file size as null and summarises the server limit', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    await user.clear(within(region).getByRole('spinbutton', { name: 'Max File Size (MB)' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', { max_file_size_mb: null }),
    );
    expect(await screen.findByText('server limit · rotate off')).toBeInTheDocument();
    expect(within(region).getByRole('spinbutton', { name: 'Max File Size (MB)' })).toHaveValue(
      null,
    );
  });

  it('shows the server limit when the user has no limit of their own', async () => {
    apiMock.get.mockResolvedValue(ok({ ...SERVER_SETTINGS, max_file_size_mb: null }));
    renderSettings(<GeneralSection />);
    expect(await screen.findByText('server limit · rotate off')).toBeInTheDocument();
  });

  it('requires a value unless the field may be empty', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Search Configuration');
    await user.clear(within(region).getByRole('spinbutton', { name: 'Fuzzy Search Threshold' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(await within(region).findByText('Enter a value.')).toBeInTheDocument();
    expect(apiMock.put).not.toHaveBeenCalled();
  });

  it('saves a switch immediately and toasts', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    await user.click(within(region).getByRole('switch', { name: /Auto-rotate Images/ }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', { auto_rotate_images: true }),
    );
    expect(await screen.findByText('Settings updated successfully')).toBeInTheDocument();
    expect(within(region).getByRole('switch', { name: /Auto-rotate Images/ })).toBeChecked();
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('reverts a switch and toasts the error when saving fails', async () => {
    apiMock.put.mockRejectedValue(
      httpError(400, { error: 'Value out of range', code: 'SETTINGS_VALUE_OUT_OF_RANGE' }),
    );
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    await user.click(within(region).getByRole('switch', { name: /Enable Background OCR/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(
      /Please enter a value within the allowed range/,
    );
    expect(within(region).getByRole('switch', { name: /Enable Background OCR/ })).toBeChecked();
  });

  it('keeps the draft and shows the server error inline when a save fails', async () => {
    apiMock.put.mockRejectedValue(
      httpError(400, { error: 'bad', code: 'SETTINGS_CONFLICTING_SETTINGS' }),
    );
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Search Configuration');
    const fuzzy = within(region).getByRole('spinbutton', { name: 'Fuzzy Search Threshold' });
    await user.clear(fuzzy);
    await user.type(fuzzy, '0.8');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(
      await within(region).findByText(
        'Conflicting settings detected. Please review your configuration.',
      ),
    ).toBeInTheDocument();
    expect(fuzzy).toHaveValue(0.8);
    expect(within(region).getByRole('button', { name: 'Save' })).toBeInTheDocument();
  });

  it('falls back to defaults when the server has no settings yet (404)', async () => {
    apiMock.get.mockRejectedValue(httpError(404));
    renderSettings(<GeneralSection />);
    expect(await screen.findByText('server limit · rotate on')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows an error when settings fail to load', async () => {
    apiMock.get.mockRejectedValue(httpError(500));
    renderSettings(<GeneralSection />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to load settings');
  });
});
