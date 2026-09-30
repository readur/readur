import { screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { UploadSection } from '../upload/UploadSection';
import { ACCEPTED_EXTENSIONS, MAX_FILE_SIZE } from '../upload/uploadConfig';
import { isLit } from '../../board/litStore';
import { api, apiError, ok, serveDefaults } from './intakeMocks';
import { renderIntake, resetIntakeState } from './intakeTestUtils';

const pdf = (name = 'invoice.pdf', size = 2048) => {
  const file = new File(['x'], name, { type: 'application/pdf' });
  Object.defineProperty(file, 'size', { value: size });
  return file;
};

const fileInput = () => screen.getByLabelText('Choose files', { selector: 'input' });

async function addFiles(user: ReturnType<typeof userEvent.setup>, files: File[]) {
  await user.upload(fileInput(), files);
  return screen.findByRole('grid', { name: 'Files to upload' });
}

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
});

describe('Add documents: drop area (ported from UploadZone)', () => {
  it('renders the drop area with its title', async () => {
    renderIntake(<UploadSection />);
    expect(screen.getByRole('group', { name: 'Drop files to add' })).toBeInTheDocument();
    expect(screen.getByText('Drop files to add')).toBeInTheDocument();
  });

  it('lists the accepted file types', () => {
    renderIntake(<UploadSection />);
    const types = screen.getByText(/PDF · PNG/);
    for (const ext of ['PDF', 'PNG', 'JPG', 'TIFF', 'TXT', 'DOCX']) expect(types).toHaveTextContent(ext);
    expect(ACCEPTED_EXTENSIONS).toEqual(['PDF', 'PNG', 'JPG', 'JPEG', 'GIF', 'BMP', 'TIFF', 'TXT', 'RTF', 'DOC', 'DOCX']);
  });

  it('shows the 50 MB size limit', () => {
    renderIntake(<UploadSection />);
    expect(screen.getByText('max 50 MB per file')).toBeInTheDocument();
    expect(MAX_FILE_SIZE).toBe(50 * 1024 * 1024);
  });

  it('shows the Choose files button', () => {
    renderIntake(<UploadSection />);
    expect(screen.getByRole('button', { name: 'Choose files' })).toBeInTheDocument();
  });

  it('keeps Choose files enabled and pressable', async () => {
    const user = userEvent.setup();
    renderIntake(<UploadSection />);
    const button = screen.getByRole('button', { name: 'Choose files' });
    await user.click(button);
    expect(button).toBeEnabled();
  });

  it('has a labelled file input for choosing files', () => {
    renderIntake(<UploadSection />);
    expect(fileInput()).toHaveAttribute('type', 'file');
    expect(fileInput()).toHaveAttribute('multiple');
  });
});

describe('Add documents: upload board', () => {
  it('lists chosen files as pending with name, size, progress and status', async () => {
    const user = userEvent.setup();
    renderIntake(<UploadSection />);
    const grid = await addFiles(user, [pdf('a.pdf'), pdf('b.pdf', 1024 * 1024)]);
    const heads = within(grid).getAllByRole('columnheader').map((h) => h.textContent);
    expect(heads.slice(0, 4)).toEqual(['Name', 'Size', 'Progress', 'Status']);
    const row = within(grid).getByRole('row', { name: /b\.pdf/ });
    expect(row).toHaveTextContent('1 MB');
    expect(row).toHaveTextContent(/pending/i);
    expect(within(row).getByRole('progressbar', { name: 'Upload progress for b.pdf' })).toHaveAttribute('aria-valuenow', '0');
    expect(screen.getByRole('button', { name: 'Upload all (2)' })).toBeEnabled();
  });

  it('shows the empty state before any file is chosen', () => {
    renderIntake(<UploadSection />);
    expect(screen.getByRole('heading', { name: 'No files yet' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Upload all (0)' })).toBeDisabled();
  });

  it('rejects files over the size limit with a message', async () => {
    const user = userEvent.setup({ applyAccept: false });
    renderIntake(<UploadSection />);
    await user.upload(fileInput(), [pdf('huge.pdf', MAX_FILE_SIZE + 1)]);
    expect(await screen.findByRole('alert')).toHaveTextContent(/huge\.pdf/);
    expect(screen.queryByRole('row', { name: /huge\.pdf/ })).not.toBeInTheDocument();
  });

  it('uploads with labels and OCR languages, then marks the new document', async () => {
    api.post.mockImplementation((_url: string, _form: FormData, config: { onUploadProgress?: (e: { loaded: number; total: number }) => void }) => {
      config.onUploadProgress?.({ loaded: 50, total: 100 });
      return ok({ id: 'doc-42' });
    });
    const user = userEvent.setup();
    renderIntake(<UploadSection />);
    const grid = await addFiles(user, [pdf('a.pdf')]);
    await user.click(screen.getByRole('button', { name: 'Upload all (1)' }));
    const row = within(grid).getByRole('row', { name: /a\.pdf/ });
    await waitFor(() => expect(row).toHaveAttribute('data-changed', 'true'));
    expect(within(row).getByText('New')).toBeInTheDocument();
    expect(within(row).getByRole('progressbar')).toHaveAttribute('aria-valuenow', '100');
    expect(isLit('document', 'doc-42')).toBe(true);
    const [url, form] = api.post.mock.calls[0];
    expect(url).toBe('/documents');
    expect((form as FormData).get('ocr_languages[0]')).toBe('eng');
    expect((form as FormData).get('file')).toBeInstanceOf(File);
  });

  it('shows a per-row error with Retry, and retrying succeeds', async () => {
    api.post.mockImplementationOnce(() => Promise.reject(apiError(413, 'DOCUMENT_TOO_LARGE', 'too big')));
    const user = userEvent.setup();
    renderIntake(<UploadSection />);
    const grid = await addFiles(user, [pdf('a.pdf')]);
    await user.click(screen.getByRole('button', { name: 'Upload all (1)' }));
    const row = within(grid).getByRole('row', { name: /a\.pdf/ });
    await waitFor(() => expect(row).toHaveTextContent('The file is too large.'));
    expect(row).toHaveTextContent(/failed/i);
    api.post.mockImplementation(() => ok({ id: 'doc-7' }));
    await user.click(within(row).getByRole('button', { name: 'Retry a.pdf' }));
    await waitFor(() => expect(isLit('document', 'doc-7')).toBe(true));
  });

  it('removes a pending file and clears finished ones', async () => {
    const user = userEvent.setup();
    renderIntake(<UploadSection />);
    const grid = await addFiles(user, [pdf('a.pdf'), pdf('b.pdf')]);
    await user.click(within(grid).getByRole('button', { name: 'Remove a.pdf' }));
    expect(within(grid).queryByRole('row', { name: /a\.pdf/ })).not.toBeInTheDocument();
    await user.click(screen.getByRole('button', { name: 'Upload all (1)' }));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Clear finished' })).toBeEnabled());
    await user.click(screen.getByRole('button', { name: 'Clear finished' }));
    expect(screen.getByRole('heading', { name: 'No files yet' })).toBeInTheDocument();
  });

  it('opens a finished document from its row', async () => {
    api.post.mockImplementation(() => ok({ id: 'doc-9' }));
    const user = userEvent.setup();
    renderIntake(<UploadSection />);
    const grid = await addFiles(user, [pdf('a.pdf')]);
    await user.click(screen.getByRole('button', { name: 'Upload all (1)' }));
    await waitFor(() => expect(within(grid).getByRole('row', { name: /a\.pdf/ })).toHaveAttribute('data-changed', 'true'));
    await user.click(within(grid).getByRole('rowheader', { name: /a\.pdf/ }));
    expect(screen.getByRole('status', { name: 'location', hidden: true })).toHaveTextContent('/documents/doc-9');
  });

  it('offers the OCR language and label pickers', async () => {
    renderIntake(<UploadSection />);
    expect(screen.getByRole('heading', { name: 'Apply to these uploads' })).toBeInTheDocument();
    expect(screen.getByText('OCR languages')).toBeInTheDocument();
    expect(screen.getByText('Labels')).toBeInTheDocument();
  });
});
