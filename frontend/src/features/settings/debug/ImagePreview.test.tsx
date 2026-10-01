import { afterEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { ImagePreview } from './ImagePreview';

describe('ImagePreview', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('renders nothing for non-raster types and never decodes them', () => {
    const create = vi.fn();
    vi.stubGlobal('createImageBitmap', create);
    const hostile = '"><img src=x onerror=alert(1)>.svg';
    const { container } = render(
      <ImagePreview file={new File(['x'], hostile, { type: 'image/svg+xml' })} caption="Preview" />,
    );
    expect(container).toBeEmptyDOMElement();
    expect(create).not.toHaveBeenCalled();
  });

  it('draws a raster file on a canvas named after the file and closes the bitmap', async () => {
    const close = vi.fn();
    const create = vi.fn().mockResolvedValue({ width: 1280, height: 800, close });
    vi.stubGlobal('createImageBitmap', create);
    const drawImage = vi.fn();
    vi.spyOn(HTMLCanvasElement.prototype, 'getContext').mockReturnValue({ drawImage } as never);

    render(<ImagePreview file={new File(['x'], 'scan.png', { type: 'image/png' })} caption="Preview" />);
    const canvas = screen.getByRole('img', { name: 'scan.png' }) as HTMLCanvasElement;
    expect(canvas.tagName).toBe('CANVAS');
    await waitFor(() => expect(drawImage).toHaveBeenCalled());
    expect(create).toHaveBeenCalled();
    expect(canvas.width).toBe(640);
    expect(canvas.height).toBe(400);
    expect(close).toHaveBeenCalled();
  });

  it('shows no preview when decoding fails', async () => {
    vi.stubGlobal('createImageBitmap', vi.fn().mockRejectedValue(new Error('bad')));
    render(<ImagePreview file={new File(['x'], 'bad.png', { type: 'image/png' })} caption="Preview" />);
    await waitFor(() => expect(screen.queryByRole('img')).toBeNull());
  });
});
