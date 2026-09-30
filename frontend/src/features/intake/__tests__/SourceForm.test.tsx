import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { SourceForm } from '../connections/form/SourceForm';
import type { SourceResponse } from '../../../services/api';
import { apiError, ok, serveDefaults, sourcesService } from './intakeMocks';
import { renderIntake, resetIntakeState, source } from './intakeTestUtils';

const onSaved = vi.fn();
const onOpenChange = vi.fn();

function renderForm(existing?: Record<string, unknown>) {
  renderIntake(
    <SourceForm
      isOpen
      onOpenChange={onOpenChange}
      source={existing ? (existing as unknown as SourceResponse) : null}
      onSaved={onSaved}
    />,
  );
  return screen.getByRole('dialog');
}

const field = (name: RegExp | string) => screen.getByRole('textbox', { name });

async function chooseType(user: ReturnType<typeof userEvent.setup>, name: RegExp) {
  await user.click(screen.getByRole('radio', { name }));
}

async function fillWebdav(user: ReturnType<typeof userEvent.setup>) {
  await user.type(field(/^name/i), 'Office cloud');
  await user.type(field(/server url/i), 'https://cloud.example.com');
  await user.type(field(/username/i), 'ada');
}

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
});

describe('SourceForm: type and per-type fields', () => {
  it('opens as "Add connection" with the three connection types, WebDAV first', () => {
    const dialog = renderForm();
    expect(within(dialog).getByRole('heading', { name: 'Add connection' })).toBeInTheDocument();
    const types = within(dialog).getByRole('radiogroup', { name: 'Connection type' });
    expect(within(types).getAllByRole('radio').map((r) => r.getAttribute('value'))).toEqual(['webdav', 'local_folder', 's3']);
    expect(within(types).getByRole('radio', { name: /webdav/i })).toBeChecked();
  });

  it('shows the WebDAV fields: server type, URL, username and password', () => {
    renderForm();
    expect(screen.getByRole('radiogroup', { name: 'Server type' })).toBeInTheDocument();
    expect(screen.getByRole('radio', { name: /^nextcloud/i })).toBeInTheDocument();
    expect(field(/server url/i)).toBeInTheDocument();
    expect(field(/username/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^password/i)).toHaveAttribute('type', 'password');
  });

  it('switches to local-folder fields and its default folder', async () => {
    const user = userEvent.setup();
    renderForm();
    await chooseType(user, /local folder/i);
    expect(screen.getByRole('switch', { name: /scan subfolders/i })).toBeChecked();
    expect(screen.getByRole('switch', { name: /follow symbolic links/i })).not.toBeChecked();
    expect(screen.queryByRole('textbox', { name: /server url/i })).not.toBeInTheDocument();
    expect(within(screen.getByRole('list', { name: 'Directories to monitor' })).getByText('/home/user/Documents')).toBeInTheDocument();
  });

  it('switches to S3 fields: bucket, region, keys, endpoint, addressing style and prefix', async () => {
    const user = userEvent.setup();
    renderForm();
    await chooseType(user, /s3-compatible/i);
    expect(field(/bucket name/i)).toBeInTheDocument();
    expect(field(/region/i)).toHaveValue('us-east-1');
    expect(field(/access key id/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/secret access key/i)).toHaveAttribute('type', 'password');
    expect(field(/endpoint url/i)).toBeInTheDocument();
    expect(field(/object key prefix/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /addressing style/i })).toHaveTextContent('Auto-detect');
    expect(within(screen.getByRole('list', { name: 'Object prefixes to monitor' })).getByText('documents/')).toBeInTheDocument();
  });

  it('hides the type choice when editing and loads every saved value', () => {
    renderForm(source('s1'));
    expect(screen.getByRole('heading', { name: 'Edit connection' })).toBeInTheDocument();
    expect(screen.queryByRole('radiogroup', { name: 'Connection type' })).not.toBeInTheDocument();
    expect(field(/^name/i)).toHaveValue('Source s1');
    expect(field(/server url/i)).toHaveValue('https://cloud.example.com');
    expect(screen.getByRole('radio', { name: /^nextcloud/i })).toBeChecked();
    expect(screen.getByRole('switch', { name: /automatic sync/i })).toBeChecked();
    expect(screen.getByRole('textbox', { name: /sync interval/i })).toHaveValue('60');
  });
});

describe('SourceForm: validation', () => {
  it('blocks saving with every required field marked and a summary announced', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(await screen.findByRole('alert')).toHaveTextContent(/fix the highlighted fields/i);
    expect(field(/^name/i)).toHaveAttribute('aria-invalid', 'true');
    expect(field(/server url/i)).toHaveAttribute('aria-invalid', 'true');
    expect(field(/username/i)).toHaveAttribute('aria-invalid', 'true');
    expect(sourcesService.create).not.toHaveBeenCalled();
  });

  it('rejects a server URL with another scheme (the backend refuses it too)', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/^name/i), 'x');
    await user.type(field(/server url/i), 'ftp://example.com');
    await user.type(field(/username/i), 'u');
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(await screen.findByText('Enter a server address such as cloud.example.com or https://cloud.example.com')).toBeInTheDocument();
    expect(sourcesService.create).not.toHaveBeenCalled();
  });

  it('accepts a bare host, which the backend completes with https://', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/^name/i), 'Home NAS');
    await user.type(field(/server url/i), 'nas.example.com');
    await user.type(field(/username/i), 'u');
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    await waitFor(() => expect(sourcesService.create).toHaveBeenCalled());
    expect(sourcesService.create.mock.calls[0][0].config.server_url).toBe('nas.example.com');
  });

  it('still saves an existing connection stored without a scheme', async () => {
    const user = userEvent.setup();
    renderForm(source('s1', { config: { ...source('s1').config, server_url: 'cloud.example.com' } }));
    await user.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(sourcesService.update).toHaveBeenCalled());
  });

  it('requires bucket and both keys for S3', async () => {
    const user = userEvent.setup();
    renderForm();
    await chooseType(user, /s3-compatible/i);
    await user.type(field(/^name/i), 'Bucket');
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(field(/bucket name/i)).toHaveAttribute('aria-invalid', 'true');
    expect(field(/access key id/i)).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByLabelText(/secret access key/i)).toHaveAttribute('aria-invalid', 'true');
  });

  it('requires at least one folder', async () => {
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    await user.click(screen.getByRole('button', { name: 'Remove /Documents' }));
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(await screen.findByText('Add at least one folder')).toBeInTheDocument();
    expect(sourcesService.create).not.toHaveBeenCalled();
  });

  it('shows the interval only with automatic sync and keeps it within 15–1440 minutes', async () => {
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    expect(screen.queryByRole('textbox', { name: /sync interval/i })).not.toBeInTheDocument();
    await user.click(screen.getByRole('switch', { name: /automatic sync/i }));
    const interval = screen.getByRole('textbox', { name: /sync interval/i });
    expect(interval).toHaveValue('60');
    await user.clear(interval);
    await user.type(interval, '10');
    await user.tab();
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(await screen.findByText('Enter whole minutes between 15 and 1440')).toBeInTheDocument();
    expect(sourcesService.create).not.toHaveBeenCalled();
  });
});

describe('SourceForm: folders and file types (ported from WebDAVTab)', () => {
  it('adds a folder and shows it in the list', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/folders to monitor/i), '/Photos');
    await user.click(screen.getByRole('button', { name: 'Add to Folders to monitor' }));
    const list = screen.getByRole('list', { name: 'Folders to monitor' });
    expect(within(list).getByText('/Documents')).toBeInTheDocument();
    expect(within(list).getByText('/Photos')).toBeInTheDocument();
    expect(field(/folders to monitor/i)).toHaveValue('');
  });

  it('adds a folder with Enter', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/folders to monitor/i), '/Scans{Enter}');
    expect(within(screen.getByRole('list', { name: 'Folders to monitor' })).getByText('/Scans')).toBeInTheDocument();
  });

  it('removes a folder', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.click(screen.getByRole('button', { name: 'Remove /Documents' }));
    expect(screen.queryByRole('list', { name: 'Folders to monitor' })).not.toBeInTheDocument();
  });

  it('prevents adding the same folder twice', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/folders to monitor/i), '/Documents');
    await user.click(screen.getByRole('button', { name: 'Add to Folders to monitor' }));
    expect(screen.getByText('“/Documents” is already in the list')).toBeInTheDocument();
    expect(within(screen.getByRole('list', { name: 'Folders to monitor' })).getAllByRole('listitem')).toHaveLength(1);
  });

  it('keeps Add disabled for an empty value', () => {
    renderForm();
    expect(screen.getByRole('button', { name: 'Add to Folders to monitor' })).toBeDisabled();
  });

  it('adds a relative WebDAV folder path but recommends an absolute one', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/folders to monitor/i), 'relative/path');
    await user.click(screen.getByRole('button', { name: 'Add to Folders to monitor' }));
    expect(within(screen.getByRole('list', { name: 'Folders to monitor' })).getByText('relative/path')).toBeInTheDocument();
    expect(screen.getByText('“relative/path” is a relative path. Absolute paths starting with “/” are recommended.')).toBeInTheDocument();
  });

  it('uses new-password autocomplete for the WebDAV password', () => {
    renderForm();
    expect(screen.getByLabelText(/^password/i)).toHaveAttribute('autocomplete', 'new-password');
  });

  it('adds file extensions without a leading dot', async () => {
    const user = userEvent.setup();
    renderForm();
    await user.type(field(/file extensions/i), '.docx');
    await user.click(screen.getByRole('button', { name: 'Add to File extensions' }));
    expect(within(screen.getByRole('list', { name: 'File extensions' })).getByText('docx')).toBeInTheDocument();
  });

  it('updates the example sync URL as the server details change', async () => {
    const user = userEvent.setup();
    renderForm();
    expect(screen.queryByRole('figure', { name: 'Example sync URL' })).not.toBeInTheDocument();
    await user.click(screen.getByRole('radio', { name: /^nextcloud/i }));
    await user.type(field(/server url/i), 'cloud.example.com');
    await user.type(field(/username/i), 'ada');
    const preview = screen.getByRole('figure', { name: 'Example sync URL' });
    expect(within(preview).getByRole('status')).toHaveTextContent(
      'https://cloud.example.com/remote.php/dav/files/ada/Documents/document1.pdf',
    );
    expect(within(preview).getByText('WebDAV path')).toBeInTheDocument();
  });
});

describe('SourceForm: test connection', () => {
  it('is disabled until the server URL and username are filled', async () => {
    const user = userEvent.setup();
    renderForm();
    const test = screen.getByRole('button', { name: 'Test connection' });
    expect(test).toBeDisabled();
    await user.type(field(/server url/i), 'https://cloud.example.com');
    await user.type(field(/username/i), 'ada');
    expect(test).toBeEnabled();
  });

  it('sends the form to the test endpoint and shows the server message', async () => {
    sourcesService.testConnection.mockImplementation(() => ok({ success: true, message: 'Connected to Nextcloud 28' }));
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    await user.click(screen.getByRole('button', { name: 'Test connection' }));
    expect(await screen.findByText('Connected to Nextcloud 28')).toBeInTheDocument();
    expect(sourcesService.testConnection).toHaveBeenCalledWith({
      source_type: 'webdav',
      config: expect.objectContaining({ server_url: 'https://cloud.example.com', username: 'ada', watch_folders: ['/Documents'] }),
    });
  });

  it('explains an authentication failure', async () => {
    sourcesService.testConnection.mockImplementation(() => Promise.reject(apiError(401, 'SOURCE_AUTH_FAILED', 'bad creds')));
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    await user.click(screen.getByRole('button', { name: 'Test connection' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Sign-in failed. Check the username and password.');
  });

  it('shows an unsuccessful test result from the server', async () => {
    sourcesService.testConnection.mockImplementation(() => ok({ success: false, message: 'Folder /Documents not found' }));
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    await user.click(screen.getByRole('button', { name: 'Test connection' }));
    expect(await screen.findByRole('alert')).toHaveTextContent('Folder /Documents not found');
  });
});

describe('SourceForm: saving', () => {
  it('creates a connection with the type and full config, then closes', async () => {
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    await waitFor(() => expect(onSaved).toHaveBeenCalled());
    expect(sourcesService.create).toHaveBeenCalledWith({
      name: 'Office cloud',
      source_type: 'webdav',
      enabled: true,
      config: expect.objectContaining({ server_url: 'https://cloud.example.com', username: 'ada', server_type: 'generic', auto_sync: false }),
    });
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it('updates an existing connection without changing its type', async () => {
    const user = userEvent.setup();
    renderForm(source('s1'));
    await user.click(screen.getByRole('switch', { name: /connection enabled/i }));
    await user.click(screen.getByRole('button', { name: 'Save changes' }));
    await waitFor(() => expect(sourcesService.update).toHaveBeenCalled());
    const [id, body] = sourcesService.update.mock.calls[0];
    expect(id).toBe('s1');
    expect(body).toMatchObject({ name: 'Source s1', enabled: false });
    expect(body).not.toHaveProperty('source_type');
  });

  it('keeps the dialog open and explains a duplicate name', async () => {
    sourcesService.create.mockImplementation(() => Promise.reject(apiError(409, 'SOURCE_DUPLICATE_NAME', 'dup')));
    const user = userEvent.setup();
    renderForm();
    await fillWebdav(user);
    await user.click(screen.getByRole('button', { name: 'Add connection' }));
    expect(await screen.findByText('A connection with this name already exists.')).toBeInTheDocument();
    expect(onSaved).not.toHaveBeenCalled();
    expect(onOpenChange).not.toHaveBeenCalledWith(false);
  });
});

describe('SourceForm: crawl estimate (ported from WebDAVTab)', () => {
  it('is offered only when editing a WebDAV connection', async () => {
    renderForm();
    expect(screen.queryByRole('button', { name: 'Estimate crawl' })).not.toBeInTheDocument();
  });

  it('asks the server for the saved connection and shows the totals and folders', async () => {
    sourcesService.estimate.mockImplementation(() =>
      ok({
        folders: [{ path: '/Documents', total_files: 1500, supported_files: 1200, estimated_time_hours: 0.67, total_size_mb: 2500.5 }],
        total_files: 1500,
        total_supported_files: 1200,
        total_estimated_time_hours: 0.67,
        total_size_mb: 2500.5,
      }),
    );
    const user = userEvent.setup();
    renderForm(source('s1'));
    await user.click(screen.getByRole('button', { name: 'Estimate crawl' }));
    expect(sourcesService.estimate).toHaveBeenCalledWith('s1');
    const totals = await screen.findByRole('group', { name: 'Estimate results' });
    expect(within(totals).getByText('0.7h')).toBeInTheDocument();
    expect(within(totals).getByText('2.4 GB')).toBeInTheDocument();
    expect(screen.getByRole('grid', { name: 'Estimate by folder' })).toHaveTextContent('/Documents');
  });
});
