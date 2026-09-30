import { useMemo, useRef, type KeyboardEvent, type RefObject, type SyntheticEvent } from 'react';

/** Marks the checkbox cell; presses there are left to React Aria's selection handling. */
export const SELECT_CELL_ATTR = 'data-select-cell';
const ROW_KEY_ATTR = 'data-flip-key';
const INTERACTIVE =
  'button, a[href], input, select, textarea, label, [role="button"], [role="checkbox"], [role="link"], [role="menuitem"], [role="switch"]';

function rowKeyFor(target: EventTarget | null, container: HTMLElement | null): string | null {
  if (!(target instanceof Element) || !container?.contains(target)) return null;
  const row = target.closest(`[role="row"][${ROW_KEY_ATTR}]`);
  if (!row || !container.contains(row)) return null;
  if (target.closest(`[${SELECT_CELL_ATTR}]`)) return null;
  // Controls rendered inside a cell (links, menu buttons) keep their own behaviour.
  const control = target.closest(INTERACTIVE);
  if (control && control !== row && row.contains(control)) return null;
  return row.getAttribute(ROW_KEY_ATTR);
}

export interface RowPressHandlers {
  onPointerDownCapture: (e: SyntheticEvent) => void;
  onMouseDownCapture: (e: SyntheticEvent) => void;
  onClickCapture: (e: SyntheticEvent) => void;
  onKeyDownCapture: (e: KeyboardEvent) => void;
}

/**
 * Makes a row press (click, tap or Enter) always run the row action, even while rows are
 * selected. React Aria's "toggle" selection turns row presses into selection toggles once a
 * selection exists; we intercept presses outside the checkbox cell in the capture phase so
 * selection only changes through the checkbox cells, Space and the select-all checkbox.
 * Space still reaches React Aria untouched.
 */
export function useRowPress(
  containerRef: RefObject<HTMLElement | null>,
  onRowAction: ((id: string) => void) | undefined,
): RowPressHandlers {
  const actionRef = useRef(onRowAction);
  actionRef.current = onRowAction;

  return useMemo(() => {
    const block = (e: SyntheticEvent) => {
      if (rowKeyFor(e.target, containerRef.current) !== null) {
        // Keep React Aria from starting a press (which would toggle selection). The browser still
        // moves focus to the row as the default action of the press.
        e.stopPropagation();
      }
    };
    return {
      onPointerDownCapture: block,
      onMouseDownCapture: block,
      onClickCapture: (e) => {
        const key = rowKeyFor(e.target, containerRef.current);
        if (key === null) return;
        e.stopPropagation();
        actionRef.current?.(key);
      },
      onKeyDownCapture: (e) => {
        if (e.key !== 'Enter' || e.altKey || e.ctrlKey || e.metaKey || e.shiftKey) return;
        const key = rowKeyFor(e.target, containerRef.current);
        if (key === null) return;
        e.preventDefault();
        e.stopPropagation();
        actionRef.current?.(key);
      },
    };
  }, [containerRef]);
}
