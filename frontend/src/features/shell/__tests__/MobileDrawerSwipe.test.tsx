import { useState } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { MobileDrawer } from '../MobileDrawer';

const WIDTH = 300;
let clock = 0;

/** A touch event with a controlled timestamp, so speed is what the test says it is. */
function touch(type: string, target: Element, x: number, y: number, dt = 50) {
  clock += dt;
  const event = new Event(type, { bubbles: true, cancelable: true });
  const point = { identifier: 1, clientX: x, clientY: y, target };
  Object.defineProperty(event, 'changedTouches', { value: [point] });
  Object.defineProperty(event, 'touches', { value: type === 'touchend' ? [] : [point] });
  Object.defineProperty(event, 'timeStamp', { value: clock });
  act(() => {
    target.dispatchEvent(event);
  });
}

/** A slow drag (no flick) from (x0, y0) by (dx, dy) in steps. */
function drag(target: Element, x0: number, y0: number, dx: number, dy = 0, steps = 6) {
  touch('touchstart', target, x0, y0);
  for (let i = 1; i <= steps; i += 1) touch('touchmove', target, x0 + (dx * i) / steps, y0 + (dy * i) / steps, 60);
}

function Harness({ enabled = true, startOpen = false, onChange = () => {} }: { enabled?: boolean; startOpen?: boolean; onChange?: (o: boolean) => void }) {
  const [open, setOpen] = useState(startOpen);
  return (
    <>
      <p>Page</p>
      <input aria-label="Name" />
      <MobileDrawer
        isOpen={open}
        swipeEnabled={enabled}
        onOpenChange={(o) => {
          onChange(o);
          setOpen(o);
        }}
      >
        <nav aria-label="Sidebar">Destinations</nav>
      </MobileDrawer>
    </>
  );
}

const panel = () => document.querySelector('[data-drawer-panel]') as HTMLElement | null;

describe('opening the menu with a swipe', () => {
  beforeEach(() => {
    clock = 0;
    vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockReturnValue(WIDTH);
  });
  afterEach(() => vi.restoreAllMocks());

  it('follows the finger while it moves, then opens when let go past halfway', async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    const page = screen.getByText('Page');
    touch('touchstart', page, 40, 400);
    touch('touchmove', page, 60, 402, 60);
    touch('touchmove', page, 160, 404, 60);
    // 120px of a 300px menu: the panel sits 180px off-screen.
    expect(panel()?.style.translate).toBe('-180px 0px');
    touch('touchmove', page, 240, 404, 60);
    touch('touchend', page, 240, 404, 200);
    expect(onChange).toHaveBeenLastCalledWith(true);
    expect(await screen.findByRole('dialog', { name: 'Menu' })).toBeInTheDocument();
  });

  it('eases back shut when let go before halfway', async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    const page = screen.getByText('Page');
    drag(page, 40, 400, 90);
    expect(panel()).not.toBeNull();
    touch('touchend', page, 130, 400, 200);
    await waitFor(() => expect(panel()).toBeNull());
    expect(onChange).not.toHaveBeenCalledWith(true);
  });

  it('opens on a quick flick even when it is short', async () => {
    const onChange = vi.fn();
    render(<Harness onChange={onChange} />);
    const page = screen.getByText('Page');
    touch('touchstart', page, 40, 400);
    touch('touchmove', page, 60, 400, 10);
    touch('touchmove', page, 100, 400, 10);
    touch('touchend', page, 100, 400, 5);
    expect(onChange).toHaveBeenLastCalledWith(true);
  });

  it('leaves a vertical scroll alone', () => {
    render(<Harness />);
    drag(screen.getByText('Page'), 40, 400, 10, 120);
    expect(panel()).toBeNull();
  });

  it('leaves a text field its own sideways drag', () => {
    render(<Harness />);
    drag(screen.getByLabelText('Name'), 40, 400, 200);
    expect(panel()).toBeNull();
  });

  it('does nothing where the sidebar is always on screen', () => {
    render(<Harness enabled={false} />);
    drag(screen.getByText('Page'), 40, 400, 200);
    expect(panel()).toBeNull();
  });

  it('closes an open menu with a leftward drag that follows the finger', async () => {
    const onChange = vi.fn();
    render(<Harness startOpen onChange={onChange} />);
    const nav = await screen.findByRole('navigation', { name: 'Sidebar' });
    touch('touchstart', nav, 250, 400);
    touch('touchmove', nav, 230, 400, 60);
    touch('touchmove', nav, 150, 400, 60);
    expect(panel()?.style.translate).toBe('-100px 0px');
    touch('touchmove', nav, 60, 400, 60);
    touch('touchend', nav, 60, 400, 200);
    await waitFor(() => expect(onChange).toHaveBeenLastCalledWith(false));
  });
});
