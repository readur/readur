import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactNode } from 'react';
import { Tooltip, TooltipTrigger } from '../Tooltip';
import { cx } from '../shared/FieldParts';
import styles from './Pass.module.css';

export interface TruncatedTextProps {
  children: ReactNode;
  className?: string;
  /** Wrap onto at most this many lines before the ellipsis (default: a single line). */
  lines?: number;
}

function textOf(node: ReactNode): string | null {
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  return null;
}

/**
 * Single-line text with an ellipsis. The full text stays in the DOM (so assistive tech reads it
 * whole); when it is visually cut off the span becomes focusable and hovering or focusing it
 * shows the full value in a Tooltip.
 */
export function TruncatedText({ children, className, lines = 1 }: TruncatedTextProps) {
  const ref = useRef<HTMLSpanElement>(null);
  const [truncated, setTruncated] = useState(false);
  const [open, setOpen] = useState(false);
  const full = textOf(children);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || full === null) return undefined;
    const check = () =>
      setTruncated(lines > 1 ? el.scrollHeight > el.clientHeight + 1 : el.scrollWidth > el.clientWidth + 1);
    check();
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(check);
    observer.observe(el);
    return () => observer.disconnect();
  }, [full, lines]);

  const canTip = truncated && full !== null;
  const show = () => canTip && setOpen(true);
  const hide = () => setOpen(false);

  const text = (
    <span
      ref={ref}
      className={cx(lines > 1 ? styles.clamp : styles.truncate, className)}
      style={lines > 1 ? ({ '--clamp-lines': lines } as CSSProperties) : undefined}
      title={canTip ? (full ?? undefined) : undefined}
      tabIndex={canTip ? 0 : undefined}
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
      onKeyDown={(e) => {
        if (e.key === 'Escape' && open) {
          e.stopPropagation();
          hide();
        }
      }}
    >
      {children}
    </span>
  );

  // The trigger state is driven by the span's own hover/focus handlers; the span is not an
  // interactive control, so it is anchored through `triggerRef` rather than RAC's Focusable.
  return (
    <TooltipTrigger isOpen={canTip && open} onOpenChange={setOpen}>
      {text}
      <Tooltip triggerRef={ref}>{full}</Tooltip>
    </TooltipTrigger>
  );
}
