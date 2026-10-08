import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import * as apiModule from '../../../services/api';
import { DocumentThumbnail } from '../DocumentThumbnail';
import { MetadataDisplay } from '../details/MetadataDisplay';
import { DocumentViewer } from '../reading/DocumentViewer';
import { splitMatches } from '../reading/highlight';
import type { ApiMock } from './mockApi';
import { stubObjectUrls } from './testUtils';

vi.mock('../../../services/api', async () => (await import('./mockApi')).createApiMock());

const m = apiModule as unknown as ApiMock;

beforeEach(() => {
  stubObjectUrls();
});

describe('DocumentViewer', () => {
  it('shows a PDF in a titled frame', async () => {
    m.documentService.view.mockResolvedValue({ data: new Blob(['%PDF']) });
    render(<DocumentViewer documentId="d1" filename="a.pdf" mimeType="application/pdf" />);
    expect(screen.getByRole('status', { name: 'Loading preview' })).toBeInTheDocument();
    expect(await screen.findByTitle('a.pdf')).toHaveAttribute('src', 'blob:fake#navpanes=0&pagemode=none&view=FitH');
    expect(m.documentService.view).toHaveBeenCalledWith('d1');
  });

  it('with openImageInNewTab, an image links to itself in a new tab; a PDF does not', async () => {
    m.documentService.view.mockResolvedValue({ data: new Blob(['png']) });
    const { unmount } = render(<DocumentViewer documentId="d1" filename="scan.png" mimeType="image/png" openImageInNewTab />);
    const link = await screen.findByRole('link', { name: 'Open scan.png in a new tab' });
    expect(link).toHaveAttribute('href', 'blob:fake');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', expect.stringContaining('noopener'));
    expect(link).toContainElement(screen.getByRole('img', { name: 'scan.png' }));
    unmount();
    m.documentService.view.mockResolvedValue({ data: new Blob(['%PDF']) });
    render(<DocumentViewer documentId="d1" filename="a.pdf" mimeType="application/pdf" openImageInNewTab />);
    await screen.findByTitle('a.pdf');
    expect(screen.queryByRole('link')).not.toBeInTheDocument();
  });

  it('shows an image with the filename as its name', async () => {
    m.documentService.view.mockResolvedValue({ data: new Blob(['png']) });
    render(<DocumentViewer documentId="d1" filename="scan.png" mimeType="image/png" />);
    expect(await screen.findByRole('img', { name: 'scan.png' })).toBeInTheDocument();
  });

  it('shows plain text files inline', async () => {
    m.documentService.view.mockResolvedValue({ data: 'hello world' });
    render(<DocumentViewer documentId="d1" filename="notes.txt" mimeType="text/plain" />);
    expect(await screen.findByText('hello world')).toBeInTheDocument();
  });

  it('explains when a type cannot be previewed', async () => {
    m.documentService.view.mockResolvedValue({ data: new Blob(['x']) });
    render(<DocumentViewer documentId="d1" filename="a.docx" mimeType="application/msword" />);
    expect(await screen.findByText('No preview for this file type')).toBeInTheDocument();
    // Named by its short type code, as in every TYPE cell, not by the raw MIME type.
    expect(screen.getByText(/^DOC files can’t be shown/)).toBeInTheDocument();
    expect(screen.queryByText(/application\/msword/)).not.toBeInTheDocument();
  });

  it('reports a failed load', async () => {
    m.documentService.view.mockRejectedValue(new Error('nope'));
    render(<DocumentViewer documentId="d1" filename="a.pdf" mimeType="application/pdf" />);
    expect(await screen.findByRole('alert')).toHaveTextContent("Couldn't load the preview.");
  });
});

describe('DocumentThumbnail', () => {
  it('shows the server thumbnail when there is one', async () => {
    m.documentService.getThumbnail.mockResolvedValue({ data: new Blob(['png']) });
    const { container } = render(<DocumentThumbnail documentId="d1" mimeType="application/pdf" size="small" />);
    await waitFor(() => expect(container.querySelector('img')).toHaveAttribute('src', 'blob:fake'));
    expect(m.documentService.getThumbnail).toHaveBeenCalledWith('d1');
  });

  it('falls back to a hairline stub with the mono type code', async () => {
    m.documentService.getThumbnail.mockRejectedValue(new Error('none'));
    const { container } = render(<DocumentThumbnail documentId="d1" mimeType="application/pdf" emptyText="No preview yet" />);
    expect(await screen.findByText('No preview yet')).toBeInTheDocument();
    expect(screen.getByText('PDF')).toHaveAttribute('aria-hidden', 'true');
    expect(container.querySelector('[data-state="none"]')).toBeInTheDocument();
  });

  it('does not show "no preview" while the thumbnail is still loading', () => {
    m.documentService.getThumbnail.mockReturnValue(new Promise(() => undefined));
    const { container } = render(<DocumentThumbnail documentId="d1" mimeType="application/pdf" emptyText="No preview yet" />);
    expect(screen.getByText('PDF')).toBeInTheDocument();
    expect(screen.queryByText('No preview yet')).not.toBeInTheDocument();
    expect(container.querySelector('[data-state="loading"]')).toBeInTheDocument();
  });

  it('does not ask the server for types it can only answer with a placeholder', async () => {
    const { container } = render(
      <DocumentThumbnail
        documentId="d1"
        mimeType="application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        emptyText="No preview yet"
      />,
    );
    expect(await screen.findByText('No preview yet')).toBeInTheDocument();
    expect(screen.getByText('DOCX')).toBeInTheDocument();
    expect(container.querySelector('img')).toBeNull();
    expect(m.documentService.getThumbnail).not.toHaveBeenCalled();
  });

  it('renders nothing without a thumbnail when the fallback is off', async () => {
    m.documentService.getThumbnail.mockRejectedValue(new Error('none'));
    const { container } = render(<DocumentThumbnail documentId="d1" mimeType="image/png" fallbackIcon={false} />);
    await waitFor(() => expect(m.documentService.getThumbnail).toHaveBeenCalled());
    expect(container).toBeEmptyDOMElement();
  });
});

describe('MetadataDisplay', () => {
  it('labels each field and formats values', () => {
    render(
      <MetadataDisplay
        title="Source metadata"
        metadata={{ camera_model: 'X100', permissions: 420, is_shared: true, keywords: ['a', 'b'] }}
      />,
    );
    expect(screen.getByRole('heading', { name: 'Source metadata' })).toBeInTheDocument();
    expect(screen.getByText('Camera model')).toBeInTheDocument();
    expect(screen.getByText('644 (octal)')).toBeInTheDocument();
    expect(screen.getByText('Yes')).toBeInTheDocument();
    expect(screen.getByText('a')).toBeInTheDocument();
  });

  it('collapses behind a summary with a field count when compact', async () => {
    const user = userEvent.setup();
    render(<MetadataDisplay compact metadata={{ owner: 'root', group: 'wheel' }} />);
    const summary = screen.getByText('2 fields').closest('summary') as HTMLElement;
    expect(summary.closest('details')).not.toHaveAttribute('open');
    await user.click(summary);
    expect(summary.closest('details')).toHaveAttribute('open');
  });

  it('renders nothing for empty metadata', () => {
    const { container } = render(<MetadataDisplay metadata={{}} />);
    expect(container).toBeEmptyDOMElement();
  });
});

describe('splitMatches', () => {
  it('splits case-insensitively and numbers the matches', () => {
    expect(splitMatches('Tax and TAX', 'tax')).toEqual([
      { text: 'Tax', match: 0 },
      { text: ' and ', match: -1 },
      { text: 'TAX', match: 1 },
    ]);
  });

  it('returns the whole text when the query is blank', () => {
    expect(splitMatches('abc', '  ')).toEqual([{ text: 'abc', match: -1 }]);
  });
});
