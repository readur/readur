/**
 * The sums behind dragging the phone menu open or shut, with no DOM in sight: where the finger
 * is, which way it is going, how fast it was moving when it lifted, and which side the menu
 * should come to rest on. The menu follows the finger the whole time, so this is asked on every
 * move, not once at the end.
 *
 * The drag may start anywhere on the screen, not only at its edge: iOS and Android keep a swipe
 * from the edge for their own Back gesture, and a page cannot have it.
 */

/** A finger position and the event's timestamp (ms). */
export interface Point {
  x: number;
  y: number;
  t: number;
}

export type MenuSide = 'closed' | 'open';

/**
 * `undecided` until the finger has moved far enough to tell; `ignored` for a scroll (or a push
 * on a closed menu, which cannot close further), after which the touch is left alone.
 */
export type Axis = 'undecided' | 'sideways' | 'ignored';

export interface Gesture {
  readonly from: MenuSide;
  readonly start: Point;
  /** Recent positions, oldest first, kept for reading the release speed. */
  readonly points: readonly Point[];
  readonly axis: Axis;
}

/** How far (px) the finger moves before its direction is judged. */
export const LOCK_DISTANCE = 8;
/** Release speed (px/ms) that counts as a flick. */
export const FLICK = 0.4;
/** How far back (ms) the release speed is measured over. */
const SPEED_WINDOW = 100;
/** A finger that sat still this long (ms) before lifting was not flicking. */
const STILL = 100;

export function begin(from: MenuSide, point: Point): Gesture {
  return { from, start: point, points: [point], axis: 'undecided' };
}

/**
 * The next position. While undecided: more down than across is a scroll; across at least twice
 * as far as down locks sideways; anything in between waits for the next move.
 */
export function move(gesture: Gesture, point: Point): Gesture {
  if (gesture.axis === 'ignored') return gesture;
  // Touches cannot arrive out of order; one that claims to is not the finger.
  const last = gesture.points.at(-1);
  if (last && point.t < last.t) return gesture;
  const points = [...gesture.points.filter((p) => p.t >= point.t - SPEED_WINDOW), point];
  if (gesture.axis === 'sideways') return { ...gesture, points };

  const dx = point.x - gesture.start.x;
  const dy = Math.abs(point.y - gesture.start.y);
  let axis: Axis = 'undecided';
  if (dy >= LOCK_DISTANCE && dy >= Math.abs(dx)) axis = 'ignored';
  else if (gesture.from === 'closed' && dx <= -LOCK_DISTANCE) axis = 'ignored';
  else if (Math.abs(dx) >= LOCK_DISTANCE && dy * 2 < Math.abs(dx)) axis = 'sideways';
  return { ...gesture, points, axis };
}

/** How far the finger has moved sideways since it landed. */
export function offset(gesture: Gesture): number {
  const last = gesture.points.at(-1);
  return last ? last.x - gesture.start.x : 0;
}

/**
 * Speed (px/ms) at release, over the last moments rather than the last two points: the lift
 * repeats the final position, which would make every flick look like a rest.
 */
export function speed(gesture: Gesture, releasedAt: number): number {
  const first = gesture.points[0];
  const last = gesture.points.at(-1);
  if (!first || !last || first === last) return 0;
  if (releasedAt - last.t > STILL) return 0;
  const elapsed = last.t - first.t;
  return elapsed > 0 ? (last.x - first.x) / elapsed : 0;
}

/** How much of the menu (0 to 1, `width` px wide) shows with the finger `dx` from where it landed. */
export function shown(from: MenuSide, dx: number, width: number): number {
  if (width <= 0) return from === 'closed' ? 0 : 1;
  const raw = from === 'closed' ? dx / width : 1 + dx / width;
  return Math.min(1, Math.max(0, raw));
}

/** Where the menu rests on release: a flick decides by its direction, otherwise the nearer side (half counts as open). */
export function settle(showing: number, releaseSpeed: number): MenuSide {
  if (Math.abs(releaseSpeed) >= FLICK) return releaseSpeed > 0 ? 'open' : 'closed';
  return showing >= 0.5 ? 'open' : 'closed';
}
