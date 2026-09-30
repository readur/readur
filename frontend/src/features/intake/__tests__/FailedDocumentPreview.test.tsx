import { screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../../services/api', async () => (await import('./intakeMocks')).apiModule);

import { FailedDocumentPreview } from '../attention/FailedDocumentPreview';
import { apiError, ocrService, ok, serveDefaults } from './intakeMocks';
import { renderIntake, resetIntakeState } from './intakeTestUtils';

const props = { failedDocumentId: 'f1', filename: 'test-document.pdf', mimeType: 'application/pdf' };
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
