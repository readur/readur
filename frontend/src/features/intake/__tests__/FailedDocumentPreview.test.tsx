import { screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { FailedDocumentPreview } from '../attention/FailedDocumentPreview';
import { apiError, ocrService, ok, serveDefaults } from './intakeMocks';
import { renderIntake, resetIntakeState } from './intakeTestUtils';

const props = { id: 'f1', filename: 'test-document.pdf', mimeType: 'application/pdf', load: (id: string) => ocrService.viewFailedDocument(id) };
const createObjectURL = vi.fn(() => 'blob:preview');
const revokeObjectURL = vi.fn();

beforeEach(() => {
  resetIntakeState();
  serveDefaults();
  Object.assign(window.URL, { createObjectURL, revokeObjectURL });
  createObjectURL.mockReturnValue('blob:preview');
  ocrService.viewFailedDocument.mockImplementation(() => ok(new Blob(['x'])));
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe('Failed document preview (ported from FailedDocumentViewer)', () => {
  it('loads through the loader it is given (document view for real documents)', async () => {
    const load = vi.fn(() => ok(new Blob(['x'])));
    renderIntake(<FailedDocumentPreview {...props} id="doc-1" load={load} />);
    expect(await screen.findByTitle('test-document.pdf')).toBeInTheDocument();
    expect(load).toHaveBeenCalledWith('doc-1');
  });

  it('loads the stored file of the failed document', async () => {
    renderIntake(<FailedDocumentPreview {...props} />);
    expect(await screen.findByTitle('test-document.pdf')).toHaveAttribute('src', 'blob:preview');
    expect(ocrService.viewFailedDocument).toHaveBeenCalledWith('f1');
  });

  it('shows images inline with the file name as alt text', async () => {
    renderIntake(<FailedDocumentPreview {...props} mimeType="image/jpeg" filename="scan.jpg" />);
    expect(await screen.findByRole('img', { name: 'scan.jpg' })).toHaveAttribute('src', 'blob:preview');
  });

  it('shows text files in a frame', async () => {
    renderIntake(<FailedDocumentPreview {...props} mimeType="text/plain" filename="notes.txt" />);
    expect(await screen.findByTitle('notes.txt')).toBeInTheDocument();
  });

  it('shows markup files as plain text in a fully sandboxed frame', async () => {
    let blobType: string | undefined;
    createObjectURL.mockImplementation(((blob: Blob) => {
      blobType = blob.type;
      return 'blob:preview';
    }) as never);
    renderIntake(<FailedDocumentPreview {...props} mimeType="text/html" filename="page.html" />);
    const frame = await screen.findByTitle('page.html');
    expect(frame).toHaveAttribute('sandbox', '');
    expect(blobType).toBe('text/plain');
  });

  it('leaves PDFs unsandboxed so the built-in viewer works', async () => {
    renderIntake(<FailedDocumentPreview {...props} />);
    expect(await screen.findByTitle('test-document.pdf')).not.toHaveAttribute('sandbox');
  });

  it('names file types it cannot preview', async () => {
    renderIntake(<FailedDocumentPreview {...props} mimeType="application/zip" filename="bundle.zip" />);
    expect(await screen.findByText('No preview for this file type (application/zip)')).toBeInTheDocument();
  });

  it('explains a missing file', async () => {
    ocrService.viewFailedDocument.mockImplementation(() => Promise.reject(apiError(404)));
    renderIntake(<FailedDocumentPreview {...props} />);
    expect(await screen.findByText('The file was not found or has been deleted.')).toBeInTheDocument();
  });

  it('explains any other load failure', async () => {
    ocrService.viewFailedDocument.mockImplementation(() => Promise.reject(apiError(500)));
    renderIntake(<FailedDocumentPreview {...props} />);
    expect(await screen.findByText('The file could not be loaded for preview.')).toBeInTheDocument();
  });
});
