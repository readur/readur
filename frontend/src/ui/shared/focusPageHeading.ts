/**
 * Moves focus to the page's h1 (made focusable for this). Use it when the control that had focus
 * removes itself, so keyboard and screen reader users are not dropped on the document body.
 */
export function focusPageHeading(): void {
  if (typeof document === 'undefined') return;
  const heading = document.querySelector<HTMLElement>('main h1') ?? document.querySelector<HTMLElement>('h1');
  if (!heading) return;
  if (!heading.hasAttribute('tabindex')) heading.setAttribute('tabindex', '-1');
  heading.focus();
}
