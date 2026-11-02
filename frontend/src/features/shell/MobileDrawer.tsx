import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from 'react';
import { flushSync } from 'react-dom';
import { Dialog as RACDialog, Modal, ModalOverlay } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { IconButton } from '../../ui';
import { Close } from '../../ui/icons';
import { begin, move, offset, settle, shown, speed, type Gesture, type MenuSide } from '../../lib/drawerGesture';
import styles from './AppShell.module.css';

export interface MobileDrawerProps {
  isOpen: boolean;
  onOpenChange: (isOpen: boolean) => void;
  children: ReactNode;
  /** A sideways drag anywhere opens the menu (and drags it shut). Only where the menu is a drawer. */
  swipeEnabled?: boolean;
}

/**
 * How long (ms) to wait for the panel to finish sliding before carrying on anyway: a panel let
 * go where it already was has no transition to end, and reduced motion makes it instant.
 */
const SLIDE_FALLBACK = 300;

/** `dragging`: following a finger. `settling`: sliding shut after a drag let go. */
type Swipe = 'none' | 'dragging' | 'settling';

/**
 * The sidebar as a left-hand drawer on phones and small tablets. Esc or the scrim closes it.
 *
 * A finger dragged sideways anywhere on the page pulls it in, and it follows the finger the whole
 * way, the scrim darkening with it; let go and it settles on the nearer side, or the side it was
 * flicked towards. Dragging the open menu left closes it the same way. Positions are written to
 * the panel's inline style on every move (not through state: a render per touchmove is too slow
 * for a 60 Hz gesture); the sums are in lib/drawerGesture.
 */
export function MobileDrawer({ isOpen, onOpenChange, children, swipeEnabled = false }: MobileDrawerProps) {
  const { t } = useTranslation();
  const [swipe, setSwipe] = useState<Swipe>('none');
  const overlay = useRef<HTMLDivElement>(null);
  const panel = useRef<HTMLDivElement>(null);
  const gesture = useRef<Gesture | null>(null);
  const finger = useRef<number | null>(null);
  const width = useRef(0);
  const openChange = useRef(onOpenChange);
  openChange.current = onOpenChange;

  const mounted = isOpen || swipe !== 'none';

  // Sliding shut after a drag: wait for the slide, then let the menu go without its own exit animation.
  useLayoutEffect(() => {
    if (swipe !== 'settling') return undefined;
    slideTo(overlay.current, panel.current, 0, width.current);
    const node = panel.current;
    const done = () => {
      overlay.current?.setAttribute('data-swiped-shut', '');
      setSwipe('none');
      if (isOpen) openChange.current(false);
    };
    const onEnd = (event: TransitionEvent) => {
      if (event.target === node) done();
    };
    node?.addEventListener('transitionend', onEnd);
    const timer = window.setTimeout(done, SLIDE_FALLBACK);
    return () => {
      node?.removeEventListener('transitionend', onEnd);
      window.clearTimeout(timer);
    };
  }, [swipe, isOpen]);

  // Every touch passes through here; nothing is prevented, the listeners only watch.
  useEffect(() => {
    if (!swipeEnabled) return undefined;
    const from: MenuSide = isOpen ? 'open' : 'closed';
    const point = (touch: Touch, event: TouchEvent) => ({ x: touch.clientX, y: touch.clientY, t: event.timeStamp });
    const own = (list: TouchList) => Array.from(list).find((touch) => touch.identifier === finger.current);

    const onStart = (event: TouchEvent) => {
      if (gesture.current || swipe === 'settling') return;
      const touch = event.changedTouches[0];
      if (!touch) return;
      if (from === 'closed' && claimed(event.target)) return;
      finger.current = touch.identifier;
      gesture.current = begin(from, point(touch, event));
    };

    const onMove = (event: TouchEvent) => {
      const current = gesture.current;
      const touch = own(event.changedTouches);
      if (!current || !touch) return;
      const next = move(current, point(touch, event));
      if (next.axis === 'ignored') {
        gesture.current = null;
        return;
      }
      gesture.current = next;
      if (next.axis !== 'sideways') return;
      if (current.axis !== 'sideways') {
        // The move that locked: the panel must exist and be laid out before it can follow.
        if (from === 'closed') flushSync(() => setSwipe('dragging'));
        overlay.current?.setAttribute('data-dragging', '');
        const measured = panel.current?.offsetWidth ?? 0;
        if (!(measured > 0)) {
          gesture.current = null;
          if (from === 'closed') setSwipe('none');
          return;
        }
        width.current = measured;
      }
      const dx = offset(next);
      const shift = Math.max(-width.current, Math.min(0, from === 'closed' ? dx - width.current : dx));
      panel.current?.style.setProperty('translate', `${shift}px 0px`);
      overlay.current?.style.setProperty('--drawer-shown', String(shown(from, dx, width.current)));
    };

    const finish = (side: MenuSide) => {
      if (side === 'open') {
        release(overlay.current, panel.current);
        if (from === 'closed') {
          openChange.current(true);
          setSwipe('none');
        }
      } else {
        setSwipe('settling');
      }
    };

    const onEnd = (event: TouchEvent) => {
      const current = gesture.current;
      if (!current || !own(event.changedTouches)) return;
      gesture.current = null;
      if (current.axis !== 'sideways') return;
      finish(settle(shown(from, offset(current), width.current), speed(current, event.timeStamp)));
    };

    // The browser took the touch for itself: whatever was dragged goes back where it was.
    const onCancel = (event: TouchEvent) => {
      const current = gesture.current;
      if (!current || !own(event.changedTouches)) return;
      gesture.current = null;
      if (current.axis === 'sideways') finish(from);
    };

    document.addEventListener('touchstart', onStart, { passive: true });
    document.addEventListener('touchmove', onMove, { passive: true });
    document.addEventListener('touchend', onEnd, { passive: true });
    document.addEventListener('touchcancel', onCancel, { passive: true });
    return () => {
      document.removeEventListener('touchstart', onStart);
      document.removeEventListener('touchmove', onMove);
      document.removeEventListener('touchend', onEnd);
      document.removeEventListener('touchcancel', onCancel);
    };
  }, [swipeEnabled, isOpen, swipe]);

  // A widened window has the sidebar back on screen: drop any drag in progress.
  useEffect(() => {
    if (!swipeEnabled && swipe !== 'none') {
      gesture.current = null;
      setSwipe('none');
    }
  }, [swipeEnabled, swipe]);

  return (
    <ModalOverlay
      ref={overlay}
      className={styles.drawerOverlay}
      isOpen={mounted}
      onOpenChange={(open) => {
        if (!open) onOpenChange(false);
      }}
      isDismissable
    >
      <Modal ref={panel} className={styles.drawer} data-drawer-panel="">
        <RACDialog className={styles.drawerDialog} aria-label={t('shell.menu.label', 'Menu')}>
          {children}
          {/* After the content so it paints above it without a z-index. */}
          <div className={styles.drawerClose}>
            <IconButton
              size="sm"
              label={t('shell.menu.close', 'Close menu')}
              icon={<Close fontSize="small" />}
              onPress={() => onOpenChange(false)}
            />
          </div>
        </RACDialog>
      </Modal>
    </ModalOverlay>
  );
}

/** Hands the panel back to the stylesheet, which eases it to its open place. */
function release(overlay: HTMLElement | null, panel: HTMLElement | null) {
  overlay?.removeAttribute('data-dragging');
  overlay?.style.removeProperty('--drawer-shown');
  panel?.style.removeProperty('translate');
}

/** Eases the panel from wherever it is to `showing` (0 shut, 1 open) of its `width`. */
function slideTo(overlay: HTMLElement | null, panel: HTMLElement | null, showing: number, width: number) {
  overlay?.removeAttribute('data-dragging');
  overlay?.style.setProperty('--drawer-shown', String(showing));
  panel?.style.setProperty('translate', `${-(1 - showing) * width}px 0px`);
}

/**
 * Whether a touch began on something that already uses a sideways drag: a text field selects
 * with it, a slider or resize handle is dragged by it, and a box that scrolls sideways scrolls.
 * Dialogs and menus over the page keep their touches too.
 */
function claimed(target: EventTarget | null): boolean {
  const el = target instanceof Element ? target : null;
  if (!el) return false;
  if (el.closest('input, textarea, select, [contenteditable="true"], [role="slider"], [role="separator"], [role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"], [data-no-swipe]')) {
    return true;
  }
  for (let box: Element | null = el; box; box = box.parentElement) {
    const overflow = getComputedStyle(box).overflowX;
    if ((overflow === 'auto' || overflow === 'scroll') && box.scrollWidth > box.clientWidth) return true;
  }
  return false;
}
