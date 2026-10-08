import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { sourceHue } from '../../lib/sourceColor';
import { SourceBadge, SourceTile } from './SourceBadge';

describe('SourceBadge', () => {
  it('shows a hue tile with the type icon and the name', () => {
    const { container } = render(<SourceBadge sourceId="abc" kind="webdav" type="webdav" name="Nextcloud" showType />);
    expect(screen.getByText('Nextcloud')).toBeInTheDocument();
    expect(screen.getByText('WebDAV')).toBeInTheDocument();
    expect(container.querySelector('[data-slot]')).toHaveAttribute('data-slot', String(sourceHue('abc', 'webdav').index));
    expect(container.querySelector('[data-tile] svg')).not.toBeNull();
    expect(container.querySelector('[data-tile]')).toHaveAttribute('aria-hidden', 'true');
  });

  it('treats a missing source as an upload', () => {
    const { container } = render(<SourceBadge sourceId={null} name="Upload" />);
    expect(container.querySelector('[data-slot]')).toHaveAttribute('data-slot', '1');
  });

  it('hides the type caption unless asked', () => {
    render(<SourceBadge sourceId="abc" kind="s3" type="s3" name="Backups" />);
    expect(screen.queryByText('S3')).not.toBeInTheDocument();
  });

  it('does not style the whole badge as the icon tile', () => {
    const { container } = render(<SourceBadge sourceId="abc" kind="s3" name="Backups" />);
    const root = container.firstElementChild as HTMLElement;
    const tile = container.querySelector('[data-tile]') as HTMLElement;
    expect(root.className.split(' ')).not.toContain(tile.className);
  });

  it('understands the backend type names for uploads and the watch folder', () => {
    const { container, rerender } = render(<SourceBadge sourceId={null} kind="upload" type="direct_upload" name="Upload" showType />);
    expect(container.querySelector('[data-tile] svg.lucide-upload')).not.toBeNull();
    expect(screen.getByText('Upload', { selector: '[class*="type"]' })).toBeInTheDocument();
    rerender(<SourceBadge sourceId={null} kind="watch" type="watch_folder" name="Watch folder" />);
    expect(container.querySelector('[data-tile] svg.lucide-folder-search')).not.toBeNull();
  });
});

describe('SourceTile', () => {
  it('is the decorative icon tile alone, in the source hue', () => {
    const { container } = render(<SourceTile sourceId={null} kind="watch" type="watch_folder" />);
    const tile = container.querySelector('[data-tile]');
    expect(tile).toHaveAttribute('aria-hidden', 'true');
    expect(tile?.querySelector('svg')).not.toBeNull();
    expect(container.firstElementChild).toHaveAttribute('data-slot');
    expect(container).toHaveTextContent('');
  });
});
