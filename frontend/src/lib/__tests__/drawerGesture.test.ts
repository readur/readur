import { describe, expect, it } from 'vitest';
import { FLICK, LOCK_DISTANCE, begin, move, offset, shown, settle, speed } from '../drawerGesture';

const at = (x: number, y: number, t: number) => ({ x, y, t });

describe('drawer gesture', () => {
  it('waits until the finger has moved far enough to tell its direction', () => {
    const g = move(begin('closed', at(100, 300, 0)), at(100 + LOCK_DISTANCE - 1, 300, 10));
    expect(g.axis).toBe('undecided');
  });

  it('locks sideways when the finger goes clearly across', () => {
    const g = move(begin('closed', at(100, 300, 0)), at(130, 305, 16));
    expect(g.axis).toBe('sideways');
    expect(offset(g)).toBe(30);
  });

  it('lets a mostly vertical move be a scroll', () => {
    const g = move(begin('closed', at(100, 300, 0)), at(106, 330, 16));
    expect(g.axis).toBe('ignored');
    // Once ignored, it stays ignored.
    expect(move(g, at(200, 330, 32)).axis).toBe('ignored');
  });

  it('ignores a leftward push on a closed menu, which has nowhere to go', () => {
    expect(move(begin('closed', at(200, 300, 0)), at(170, 300, 16)).axis).toBe('ignored');
  });

  it('closes an open menu with a leftward drag', () => {
    expect(move(begin('open', at(200, 300, 0)), at(170, 302, 16)).axis).toBe('sideways');
  });

  it('measures how much of the menu shows, from nothing or from everything, within 0 and 1', () => {
    expect(shown('closed', 150, 300)).toBe(0.5);
    expect(shown('closed', 900, 300)).toBe(1);
    expect(shown('closed', -40, 300)).toBe(0);
    expect(shown('open', -75, 300)).toBe(0.75);
    expect(shown('open', 50, 300)).toBe(1);
    expect(shown('closed', 50, 0)).toBe(0);
  });

  it('reads the release speed over the last moments, and calls a resting finger still', () => {
    let g = begin('closed', at(0, 300, 0));
    g = move(g, at(20, 300, 20));
    g = move(g, at(60, 300, 40));
    g = move(g, at(120, 300, 60));
    expect(speed(g, 70)).toBeCloseTo(2, 1);
    // Held still for a while before lifting: not a flick.
    expect(speed(g, 400)).toBe(0);
  });

  it('settles on the nearer side, unless thrown', () => {
    expect(settle(0.6, 0)).toBe('open');
    expect(settle(0.4, 0)).toBe('closed');
    expect(settle(0.5, 0)).toBe('open');
    expect(settle(0.2, FLICK)).toBe('open');
    expect(settle(0.9, -FLICK)).toBe('closed');
  });
});
