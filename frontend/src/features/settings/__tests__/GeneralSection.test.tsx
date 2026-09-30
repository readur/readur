import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import GeneralSection from '../general/GeneralSection';
import { apiMock, httpError, ok, renderSettings } from './settingsTestUtils';

vi.mock('../../../services/api', async (importOriginal) => (await import('./apiMock')).mockApiModule(importOriginal));

const SERVER_SETTINGS = {
  max_file_size_mb: 80,
  memory_limit_mb: 1024,
  auto_rotate_images: false,
  enable_image_preprocessing: true,
  enable_background_ocr: true,
  search_results_per_page: 50,
  search_snippet_length: 300,
  fuzzy_search_threshold: 0.6,
  retention_days: 30,
  enable_auto_cleanup: true,
  enable_compression: false,
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
    expect(await screen.findByText('80 MB max · 1024 MB memory · rotate off')).toBeInTheDocument();
    expect(screen.getByText('50 per page · 300 chars · fuzzy 0.6')).toBeInTheDocument();
    expect(screen.getByText('keep 30 days · cleanup on · compression off')).toBeInTheDocument();
    expect(apiMock.get).toHaveBeenCalledWith('/settings');
  });

  it('renders every file, search and storage field from the previous page', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const file = await openGroup(user, 'File Processing');
    expect(within(file).getByRole('spinbutton', { name: 'Max File Size (MB)' })).toHaveValue(80);
    expect(within(file).getByRole('spinbutton', { name: 'Memory Limit (MB)' })).toHaveValue(1024);
    expect(within(file).getByRole('switch', { name: /Auto-rotate Images/ })).not.toBeChecked();
    expect(within(file).getByRole('switch', { name: /Enable Image Preprocessing/ })).toBeChecked();
    expect(within(file).getByRole('switch', { name: /Enable Background OCR/ })).toBeChecked();
    expect(within(file).getByText(/Enabling preprocessing can significantly alter/)).toBeInTheDocument();
    const search = await openGroup(user, 'Search Configuration');
    expect(within(search).getByRole('button', { name: /Results Per Page/ })).toHaveTextContent('50');
    expect(within(search).getByRole('spinbutton', { name: 'Snippet Length' })).toHaveValue(300);
    expect(within(search).getByRole('spinbutton', { name: 'Fuzzy Search Threshold' })).toHaveValue(0.6);
    const storage = await openGroup(user, 'Storage Management');
    expect(within(storage).getByRole('spinbutton', { name: 'Retention Days' })).toHaveValue(30);
    expect(within(storage).getByRole('switch', { name: /Enable Auto Cleanup/ })).toBeChecked();
    expect(within(storage).getByRole('switch', { name: /Enable Compression/ })).not.toBeChecked();
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
    const memory = within(region).getByRole('spinbutton', { name: 'Memory Limit (MB)' });
    await user.clear(memory);
    await user.type(memory, '2048');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/settings', { memory_limit_mb: 2048 }));
    expect(await screen.findByText('Settings updated successfully')).toBeInTheDocument();
    expect(await screen.findByText('80 MB max · 2048 MB memory · rotate off')).toBeInTheDocument();
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('saves a select change and a decimal together', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Search Configuration');
    await user.click(within(region).getByRole('button', { name: /Results Per Page/ }));
    await user.click(await screen.findByRole('option', { name: '100' }));
    const fuzzy = within(region).getByRole('spinbutton', { name: 'Fuzzy Search Threshold' });
    await user.clear(fuzzy);
    await user.type(fuzzy, '0.9');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(apiMock.put).toHaveBeenCalledWith('/settings', { search_results_per_page: 100, fuzzy_search_threshold: 0.9 }),
    );
  });

  it.each([
    ['Max File Size (MB)', 'File Processing', '501', 'Enter a value from 1 to 500.'],
    ['Memory Limit (MB)', 'File Processing', '64', 'Enter a value from 128 to 4096.'],
    ['Max File Size (MB)', 'File Processing', '2.5', 'Enter a whole number.'],
    ['Snippet Length', 'Search Configuration', '20', 'Enter a value from 50 to 500.'],
    ['Fuzzy Search Threshold', 'Search Configuration', '1.5', 'Enter a value from 0 to 1.'],
    ['Retention Days', 'Storage Management', '0', 'Enter 1 or more.'],
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

  it('requires a value unless the field may be empty', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    await user.clear(within(region).getByRole('spinbutton', { name: 'Max File Size (MB)' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(await within(region).findByText('Enter a value.')).toBeInTheDocument();
    expect(apiMock.put).not.toHaveBeenCalled();
  });

  it('saves an empty retention period as null (keep forever)', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Storage Management');
    await user.clear(within(region).getByRole('spinbutton', { name: 'Retention Days' }));
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/settings', { retention_days: null }));
    expect(await screen.findByText('keep forever · cleanup on · compression off')).toBeInTheDocument();
  });

  it('saves a switch immediately and toasts', async () => {
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'File Processing');
    await user.click(within(region).getByRole('switch', { name: /Auto-rotate Images/ }));
    await waitFor(() => expect(apiMock.put).toHaveBeenCalledWith('/settings', { auto_rotate_images: true }));
    expect(await screen.findByText('Settings updated successfully')).toBeInTheDocument();
    expect(within(region).getByRole('switch', { name: /Auto-rotate Images/ })).toBeChecked();
    expect(within(region).queryByRole('button', { name: 'Save' })).not.toBeInTheDocument();
  });

  it('reverts a switch and toasts the error when saving fails', async () => {
    apiMock.put.mockRejectedValue(httpError(400, { error: 'Value out of range', code: 'SETTINGS_VALUE_OUT_OF_RANGE' }));
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Storage Management');
    await user.click(within(region).getByRole('switch', { name: /Enable Compression/ }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/Please enter a value within the allowed range/);
    expect(within(region).getByRole('switch', { name: /Enable Compression/ })).not.toBeChecked();
  });

  it('keeps the draft and shows the server error inline when a save fails', async () => {
    apiMock.put.mockRejectedValue(httpError(400, { error: 'bad', code: 'SETTINGS_CONFLICTING_SETTINGS' }));
    const user = userEvent.setup();
    renderSettings(<GeneralSection />);
    const region = await openGroup(user, 'Search Configuration');
    const snippet = within(region).getByRole('spinbutton', { name: 'Snippet Length' });
    await user.clear(snippet);
    await user.type(snippet, '250');
    await user.click(within(region).getByRole('button', { name: 'Save' }));
    expect(
      await within(region).findByText('Conflicting settings detected. Please review your configuration.'),
    ).toBeInTheDocument();
    expect(snippet).toHaveValue(250);
    expect(within(region).getByRole('button', { name: 'Save' })).toBeInTheDocument();
  });

  it('falls back to defaults when the server has no settings yet (404)', async () => {
    apiMock.get.mockRejectedValue(httpError(404));
    renderSettings(<GeneralSection />);
    expect(await screen.findByText('50 MB max · 512 MB memory · rotate on')).toBeInTheDocument();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
  });

  it('shows an error when settings fail to load', async () => {
    apiMock.get.mockRejectedValue(httpError(500));
    renderSettings(<GeneralSection />);
    expect(await screen.findByRole('alert')).toHaveTextContent('Failed to load settings');
  });
});
