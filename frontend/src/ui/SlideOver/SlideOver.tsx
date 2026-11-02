import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  type ReactNode,
} from 'react';
import { Dialog as RACDialog, Heading, Modal, ModalOverlay } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton } from '../IconButton';
import { Close } from '../icons';
import { cx } from '../shared/FieldParts';
import { useToastInsetReporter } from '../Toast';
import styles from './SlideOver.module.css';

export interface SlideOverProps {
  title: ReactNode;
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  children?: ReactNode;
  /** Sticky footer, typically `Button`s. */
  footer?: ReactNode;
  /** Panel width on desktop. Default 440px. Always full width below 720px. */
  width?: number | string;
  /**
   * Called for ↑/↓ when focus is not in a text field or a list, so the parent can show the
   * previous or next record without closing the panel.
   */
  onNavigate?: (direction: 'previous' | 'next') => void;
  /** Adds a grab handle on the left edge to resize the panel (desktop only). */
  resizable?: boolean;
  /** With `resizable`, remembers the width in this browser under `readur.slideover.<key>`. */
  storageKey?: string;
  /** With `resizable`, the share of the window it opens at when no width is remembered (default 0.5). */
  defaultShare?: number;
  /**
   * `fill`: the body has no padding and is a column the content divides itself (a fixed preview
   * above a scrolling pane, say). The body still scrolls if the content cannot fit.
   */
  layout?: 'padded' | 'fill';
  className?: string;
}

const DEFAULT_WIDTH = 440;
export const SLIDEOVER_MIN_WIDTH = 360;
/** A resizable panel opens at half the window and can grow to 85% of it. */
const DEFAULT_SHARE = 0.5;
const MAX_SHARE = 0.85;
const KEY_STEP = 24;

const windowWidth = () => (typeof window !== 'undefined' ? window.innerWidth : 1280);
const maxWidth = () => Math.max(SLIDEOVER_MIN_WIDTH, Math.round(windowWidth() * MAX_SHARE));
const clampWidth = (w: number) => Math.round(Math.min(maxWidth(), Math.max(SLIDEOVER_MIN_WIDTH, w)));

const storageName = (key: string) => `readur.slideover.${key}`;

function readWidth(key: string | undefined): number | null {
  if (!key) return null;
  try {
    const raw = Number(window.localStorage.getItem(storageName(key)));
    return Number.isFinite(raw) && raw > 0 ? clampWidth(raw) : null;
  } catch {
    return null;
  }
}

/** Where a resizable panel opens: its remembered width, else its share of the window. */
const openingWidth = (key: string | undefined, share = DEFAULT_SHARE) =>
  readWidth(key) ?? clampWidth(windowWidth() * share);

function writeWidth(key: string | undefined, width: number) {
  if (!key) return;
  try {
    window.localStorage.setItem(storageName(key), String(width));
  } catch {
    /* private mode or blocked storage: the width just isn't remembered */
  }
}

/** The left-edge grab handle: drag it, or focus it and use ←/→. */
function ResizeHandle({ width, onResize, onCommit }: { width: number; onResize: (w: number) => void; onCommit: (w: number) => void }) {
  const { t } = useTranslation();
  const drag = useRef<{ x: number; width: number; latest: number } | null>(null);

  const onPointerDown = (e: ReactPointerEvent<HTMLDivElement>) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture?.(e.pointerId);
    drag.current = { x: e.clientX, width, latest: width };
  };
  const onPointerMove = (e: ReactPointerEvent<HTMLDivElement>) => {
    const d = drag.current;
    if (!d) return;
    // The panel is anchored right, so moving the handle left makes it wider.
    d.latest = clampWidth(d.width + (d.x - e.clientX));
    onResize(d.latest);
  };
  const onPointerUp = () => {
    const d = drag.current;
    drag.current = null;
    if (d) onCommit(d.latest);
  };
  const onKeyDown = (e: ReactKeyboardEvent<HTMLDivElement>) => {
    if (e.key !== 'ArrowLeft' && e.key !== 'ArrowRight') return;
    e.preventDefault();
    const next = clampWidth(width + (e.key === 'ArrowLeft' ? KEY_STEP : -KEY_STEP));
    onResize(next);
    onCommit(next);
  };

  return (
    <div
      role="separator"
      aria-orientation="vertical"
      aria-label={t('ui.slideOver.resize', 'Resize panel')}
      aria-valuenow={width}
      aria-valuemin={SLIDEOVER_MIN_WIDTH}
      aria-valuemax={maxWidth()}
      tabIndex={0}
      className={styles.handle}
      onPointerDown={onPointerDown}
      onPointerMove={onPointerMove}
      onPointerUp={onPointerUp}
      onPointerCancel={onPointerUp}
      onKeyDown={onKeyDown}
    >
      <span className={styles.grip} aria-hidden="true" />
    </div>
  );
}

/** Room a toast needs beside the panel: the 360px region plus its 16px gutters. */
const TOAST_CLEARANCE = 360 + 2 * 16;

/** Where ↑/↓ belong to the content, not to record navigation. `data-own-arrows` opts a pane in. */
const EDITABLE =
  'input, textarea, select, [contenteditable="true"], [role="listbox"], [role="menu"], [role="grid"], [role="slider"], [data-own-arrows]';

/**
 * Right-anchored detail panel. Focus moves into the panel, Esc closes it and focus returns to the
 * trigger. The parent may swap its content while it is open; focus stays inside the panel.
 */
export function SlideOver({
  title,
  isOpen,
  onOpenChange,
  children,
  footer,
  width,
  onNavigate,
  resizable = false,
  storageKey,
  defaultShare,
  layout = 'padded',
  className,
}: SlideOverProps) {
  const { t } = useTranslation();
  const dialogRef = useRef<HTMLElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);
  const footerRef = useRef<HTMLElement>(null);
  const navigateRef = useRef(onNavigate);
  navigateRef.current = onNavigate;
  const reportToastInset = useToastInsetReporter();
  const [owner] = useState(() => Symbol('SlideOver'));
  const hasFooter = Boolean(footer);
  const [dragWidth, setDragWidth] = useState<number | null>(() =>
    resizable ? openingWidth(storageKey, defaultShare) : null,
  );
  // Re-read on each opening: the window may have changed size since.
  useEffect(() => {
    if (isOpen && resizable) setDragWidth(openingWidth(storageKey, defaultShare));
  }, [isOpen, resizable, storageKey, defaultShare]);

  // Keep toasts off the panel: beside it when there is room, else above its footer (phones).
  useLayoutEffect(() => {
    const panel = panelRef.current;
    if (!isOpen || !panel) return undefined;
    const measure = () => {
      const width = panel.offsetWidth;
      if (window.innerWidth - width >= TOAST_CLEARANCE) reportToastInset(owner, { right: width, bottom: 0 });
      else reportToastInset(owner, { right: 0, bottom: footerRef.current?.offsetHeight ?? 0 });
    };
    measure();
    const observer = typeof ResizeObserver === 'function' ? new ResizeObserver(measure) : null;
    observer?.observe(panel);
    if (footerRef.current) observer?.observe(footerRef.current);
    window.addEventListener('resize', measure);
    return () => {
      observer?.disconnect();
      window.removeEventListener('resize', measure);
      reportToastInset(owner, null);
    };
  }, [isOpen, hasFooter, owner, reportToastInset]);

  // When swapped content removes the focused element, keep focus inside the panel.
  useLayoutEffect(() => {
    const dialog = dialogRef.current;
    if (!isOpen || !dialog) return;
    const active = document.activeElement;
    if (!active || active === document.body || !active.isConnected) {
      dialog.focus({ preventScroll: true });
    }
  });

  // RAC's Dialog does not forward keyboard handlers, so listen natively. This also catches keys
  // pressed while the panel itself holds focus.
  useEffect(() => {
    const dialog = dialogRef.current;
    if (!isOpen || !dialog) return undefined;
    const onKeyDown = (e: KeyboardEvent) => {
      const navigate = navigateRef.current;
      if (!navigate || (e.key !== 'ArrowUp' && e.key !== 'ArrowDown')) return;
      if (e.altKey || e.ctrlKey || e.metaKey || e.shiftKey || e.defaultPrevented) return;
      const target = e.target as Element | null;
      if (target?.closest?.(EDITABLE)) return;
      e.preventDefault();
      navigate(e.key === 'ArrowUp' ? 'previous' : 'next');
    };
    dialog.addEventListener('keydown', onKeyDown);
    return () => dialog.removeEventListener('keydown', onKeyDown);
  }, [isOpen]);

  const shownWidth = dragWidth ?? width;
  const style =
    shownWidth !== undefined
      ? ({ '--slideover-width': typeof shownWidth === 'number' ? `${shownWidth}px` : shownWidth } as CSSProperties)
      : undefined;
  const numericWidth = dragWidth ?? (typeof width === 'number' ? width : DEFAULT_WIDTH);

  return (
    <ModalOverlay className={styles.overlay} isOpen={isOpen} onOpenChange={onOpenChange} isDismissable>
      <Modal ref={panelRef} className={cx(styles.panel, className)} style={style}>
        <RACDialog ref={dialogRef} className={styles.dialog}>
          {resizable ? (
            <ResizeHandle width={numericWidth} onResize={setDragWidth} onCommit={(w) => writeWidth(storageKey, w)} />
          ) : null}
          <header className={styles.header}>
            <Heading slot="title" className={styles.title}>
              {title}
            </Heading>
            <IconButton
              size="sm"
              label={t('ui.close', 'Close')}
              icon={<Close fontSize="small" />}
              onPress={() => onOpenChange(false)}
            />
          </header>
          <div className={styles.body} data-layout={layout}>
            {children}
          </div>
          {footer ? (
            <footer ref={footerRef} className={styles.footer}>
              {footer}
            </footer>
          ) : null}
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}
