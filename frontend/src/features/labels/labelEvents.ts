/**
 * Dispatched on `window` whenever labels are created or assigned in bulk, so lists elsewhere on
 * screen (the sidebar's collections) can refresh.
 */
export const LABELS_CHANGED_EVENT = 'readur:labels-changed';

export function notifyLabelsChanged(): void {
  window.dispatchEvent(new CustomEvent(LABELS_CHANGED_EVENT));
}
