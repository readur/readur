import { useRef, type KeyboardEvent, type PointerEvent } from 'react';
import { cx } from '../shared/FieldParts';
import styles from './SplitHandle.module.css';

export interface SplitHandleProps {
  /** Names the separator, e.g. "Resize preview". */
  label: string;
  /** Size in px of the part above the handle. */
  value: number;
  min: number;
  max: number;
  /** While dragging or stepping. */
  onChange: (value: number) => void;
  /** Once a drag ends or a key step lands: the time to remember the size. */
  onCommit?: (value: number) => void;
  /** Px per arrow-key step. Default 24. */
  step?: number;
  className?: string;
}

/**
 * An 8px divider with a three-dot grip between two stacked parts; dragging it (or ↑/↓, Home/End
 * when focused) resizes the part above. Its arrow keys are its own, also inside a drawer.
 */
export function SplitHandle({ label, value, min, max, onChange, onCommit, step = 24, className }: SplitHandleProps) {
  const drag = useRef<{ y: number; start: number; latest: number } | null>(null);
  const clamp = (v: number) => Math.round(Math.min(max, Math.max(min, v)));

  const onPointerDown = (e: PointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    drag.current = { y: e.clientY, start: value, latest: value };
  };
  const onPointerMove = (e: PointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    d.latest = clamp(d.start + (e.clientY - d.y));
    onChange(d.latest);
  };
  const onPointerUp = () => {
    const d = drag.current;
    drag.current = null;
    if (d) onCommit?.(d.latest);
  };
  const onKeyDown = (e: KeyboardEvent<HTMLDivElement>) => {
    const next =
      e.key === 'ArrowDown' ? value + step
      : e.key === 'ArrowUp' ? value - step
      : e.key === 'Home' ? min
      : e.key === 'End' ? max
      : null;
    if (next === null) return;
    e.preventDefault();
    const clamped = clamp(next);
    onChange(clamped);
    onCommit?.(clamped);
  };

  return (
    <div
      role="separator"
      aria-orientation="horizontal"
      aria-label={label}
      aria-valuenow={Math.round(value)}
      aria-valuemin={min}
      aria-valuemax={max}
      tabIndex={0}
      data-own-arrows=""
      className={cx(styles.handle, className)}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onKeyDown={onKeyDown}
    >
      <span className={styles.grip} aria-hidden="true">
        <span className={styles.dot} data-dot="" />
        <span className={styles.dot} data-dot="" />
        <span className={styles.dot} data-dot="" />
      </span>
    </div>
  );
}
