import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { sourceHue } from '../../lib/sourceColor';
import { SourceBadge } from './SourceBadge';

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
});
