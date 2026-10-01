import { useState } from 'react';
import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { SlideOver } from '../SlideOver';
import { ToastProvider, useToast } from './Toast';

const PANEL_WIDTH = 440;
const FOOTER_HEIGHT = 64;
let viewportWidth = 1280;

function Page() {
  const toast = useToast();
  const [open, setOpen] = useState(false);
  return (
    <>
      <Button onPress={() => setOpen(true)}>Open details</Button>
      <Button onPress={() => toast.show({ title: 'Retry failed', tone: 'danger', timeout: 60000 })}>Notify</Button>
      <SlideOver
        title="broken.pdf"
        isOpen={open}
        onOpenChange={setOpen}
        footer={<Button onPress={() => toast.show({ title: 'Retry failed', tone: 'danger', timeout: 60000 })}>Retry OCR</Button>}
      >
        <p>Details</p>
      </SlideOver>
    </>
  );
}

const region = () => screen.getByRole('region', { name: 'Notifications' });
const inset = (side: 'right' | 'bottom') => region().style.getPropertyValue(`--toast-inset-${side}`);

beforeEach(() => {
  viewportWidth = 1280;
  vi.spyOn(window, 'innerWidth', 'get').mockImplementation(() => viewportWidth);
  // jsdom has no layout: the panel (the element holding the dialog) and the footer get sizes here.
  vi.spyOn(HTMLElement.prototype, 'offsetWidth', 'get').mockImplementation(function (this: HTMLElement) {
    if (this.querySelector(':scope > [role="dialog"]')) return Math.min(PANEL_WIDTH, viewportWidth);
    return 0;
  });
  vi.spyOn(HTMLElement.prototype, 'offsetHeight', 'get').mockImplementation(function (this: HTMLElement) {
    return this.tagName === 'FOOTER' ? FOOTER_HEIGHT : 0;
  });
});

afterEach(() => vi.restoreAllMocks());

describe('Toasts and an open SlideOver', () => {
  it('sit at the screen corner while no panel is open', async () => {
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <Page />
      </ToastProvider>,
    );
    await user.click(screen.getByRole('button', { name: 'Notify' }));
    await screen.findByRole('alert');
    expect(inset('right')).toBe('0px');
    expect(inset('bottom')).toBe('0px');
  });

  it('move beside a side panel so its footer stays pressable, and come back when it closes', async () => {
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <Page />
      </ToastProvider>,
    );
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    const panel = await screen.findByRole('dialog', { name: 'broken.pdf' });
    await user.click(screen.getByRole('button', { name: 'Retry OCR' }));
    await screen.findByRole('alert');
    expect(inset('right')).toBe(`${PANEL_WIDTH}px`);
    expect(inset('bottom')).toBe('0px');

    await user.keyboard('{Escape}');
    await waitFor(() => expect(panel).not.toBeInTheDocument());
    expect(inset('right')).toBe('0px');
  });

  it('stack above the footer of a full-width panel on a phone', async () => {
    viewportWidth = 390;
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <Page />
      </ToastProvider>,
    );
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    await screen.findByRole('dialog', { name: 'broken.pdf' });
    await user.click(screen.getByRole('button', { name: 'Retry OCR' }));
    await screen.findByRole('alert');
    expect(inset('right')).toBe('0px');
    expect(inset('bottom')).toBe(`${FOOTER_HEIGHT}px`);
  });

  it('follow a resize from phone to desktop width', async () => {
    viewportWidth = 390;
    const user = userEvent.setup();
    render(
      <ToastProvider>
        <Page />
      </ToastProvider>,
    );
    await user.click(screen.getByRole('button', { name: 'Open details' }));
    await screen.findByRole('dialog', { name: 'broken.pdf' });
    await user.click(screen.getByRole('button', { name: 'Retry OCR' }));
    await screen.findByRole('alert');
    viewportWidth = 1280;
    act(() => {
      window.dispatchEvent(new Event('resize'));
    });
    expect(inset('right')).toBe(`${PANEL_WIDTH}px`);
    expect(inset('bottom')).toBe('0px');
  });
});
