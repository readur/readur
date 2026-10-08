import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { useState } from 'react';
import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { SplitHandle } from './SplitHandle';

function Harness({ onCommit = () => {} }: { onCommit?: (v: number) => void }) {
  const [value, setValue] = useState(300);
  return <SplitHandle label="Resize preview" value={value} min={120} max={500} onChange={setValue} onCommit={onCommit} />;
}

describe('SplitHandle', () => {
  it('is a labelled horizontal separator carrying its value and bounds, with a three-dot grip', () => {
    render(<Harness />);
    const handle = screen.getByRole('separator', { name: 'Resize preview' });
    expect(handle).toHaveAttribute('aria-orientation', 'horizontal');
    expect(handle).toHaveAttribute('aria-valuenow', '300');
    expect(handle).toHaveAttribute('aria-valuemin', '120');
    expect(handle).toHaveAttribute('aria-valuemax', '500');
    expect(handle.querySelectorAll('[data-dot]')).toHaveLength(3);
    // Inside a drawer, ↑/↓ belong to the handle, not to record navigation.
    expect(handle).toHaveAttribute('data-own-arrows');
  });

  it('moves with ↑/↓ in steps, jumps with Home/End, and stays within bounds', async () => {
    const onCommit = vi.fn();
    const user = userEvent.setup();
    render(<Harness onCommit={onCommit} />);
    const handle = screen.getByRole('separator', { name: 'Resize preview' });
    act(() => handle.focus());
    await user.keyboard('{ArrowDown}');
    expect(handle).toHaveAttribute('aria-valuenow', '324');
    expect(onCommit).toHaveBeenLastCalledWith(324);
    await user.keyboard('{ArrowUp}{ArrowUp}');
    expect(handle).toHaveAttribute('aria-valuenow', '276');
    await user.keyboard('{End}');
    expect(handle).toHaveAttribute('aria-valuenow', '500');
    await user.keyboard('{ArrowDown}');
    expect(handle).toHaveAttribute('aria-valuenow', '500');
    await user.keyboard('{Home}');
    expect(handle).toHaveAttribute('aria-valuenow', '120');
  });

  it('follows a pointer drag and commits once on release', () => {
    const onCommit = vi.fn();
    render(<Harness onCommit={onCommit} />);
    const handle = screen.getByRole('separator', { name: 'Resize preview' });
    handle.setPointerCapture = vi.fn();
    act(() => {
      handle.dispatchEvent(new PointerEvent('pointerdown', { bubbles: true, clientY: 400, pointerId: 1, button: 0 }));
      handle.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientY: 460, pointerId: 1 }));
      handle.dispatchEvent(new PointerEvent('pointermove', { bubbles: true, clientY: 1000, pointerId: 1 }));
      handle.dispatchEvent(new PointerEvent('pointerup', { bubbles: true, clientY: 1000, pointerId: 1 }));
    });
    expect(handle).toHaveAttribute('aria-valuenow', '500');
    expect(onCommit).toHaveBeenCalledTimes(1);
    expect(onCommit).toHaveBeenCalledWith(500);
  });
});

describe('SplitHandle grab area', () => {
  const css = readFileSync(resolve(__dirname, 'SplitHandle.module.css'), 'utf8');
  it('stays an 8px bar to look at', () => {
    expect(css).toMatch(/\.handle \{[^}]*height: 8px/);
  });
  it('catches a pointer well above and below the bar, and more for a finger', () => {
    expect(css).toMatch(/\.handle \{[^}]*position: relative/);
    expect(css).toMatch(/\.handle::before \{[^}]*content: "";[^}]*position: absolute;[^}]*inset: -8px 0/);
    expect(css).toMatch(/@media \(pointer: coarse\) \{\s*\.handle::before \{\s*inset: -26px 0 -10px;/);
  });
});
