import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { MemoryRouter, useLocation, useNavigate } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import { useDrawerParam } from '../useDrawerParam';

function Harness({ paramKey = 'document' }: { paramKey?: string }) {
  const drawer = useDrawerParam(paramKey);
  return (
    <div>
      <output aria-label={`${paramKey} id`}>{drawer.id ?? 'none'}</output>
      <output aria-label={`${paramKey} href`}>{drawer.href('z9')}</output>
      <button type="button" onClick={() => drawer.open('a1')}>open {paramKey} a1</button>
      <button type="button" onClick={() => drawer.open('b2')}>open {paramKey} b2</button>
      <button type="button" onClick={() => drawer.step('c3')}>step {paramKey} c3</button>
      <button type="button" onClick={() => drawer.close()}>close {paramKey}</button>
      <button
        type="button"
        onClick={() => {
          drawer.close();
          drawer.close();
        }}
      >
        close {paramKey} twice
      </button>
    </div>
  );
}

function Probe() {
  const location = useLocation();
  const navigate = useNavigate();
  return (
    <>
      <output aria-label="url">{location.pathname + location.search + location.hash}</output>
      <button type="button" onClick={() => navigate(-1)}>back</button>
      <button type="button" onClick={() => navigate(1)}>forward</button>
    </>
  );
}

function setup(entries: string[], extra?: React.ReactNode) {
  const user = userEvent.setup();
  render(
    <MemoryRouter initialEntries={entries} initialIndex={entries.length - 1}>
      <Harness />
      {extra}
      <Probe />
    </MemoryRouter>,
  );
  const url = () => screen.getByRole('status', { name: 'url' }).textContent;
  return { user, url };
}

describe('useDrawerParam', () => {
  it('reads the open id from the URL', () => {
    setup(['/documents?document=x7']);
    expect(screen.getByRole('status', { name: 'document id' })).toHaveTextContent('x7');
  });

  it('opens by adding the param and keeps the other params and the hash', async () => {
    const { user, url } = setup(['/documents?labels=work&page=2#top']);
    await user.click(screen.getByRole('button', { name: 'open document a1' }));
    expect(url()).toBe('/documents?labels=work&page=2&document=a1#top');
    expect(screen.getByRole('status', { name: 'document id' })).toHaveTextContent('a1');
  });

  it('pushes a history entry on open, so Back closes the drawer', async () => {
    const { user, url } = setup(['/documents?labels=work']);
    await user.click(screen.getByRole('button', { name: 'open document a1' }));
    await user.click(screen.getByRole('button', { name: 'back' }));
    expect(url()).toBe('/documents?labels=work');
  });

  it('closes a drawer it opened by going back, leaving no extra history entry', async () => {
    const { user, url } = setup(['/home', '/documents?labels=work']);
    await user.click(screen.getByRole('button', { name: 'open document a1' }));
    await user.click(screen.getByRole('button', { name: 'close document' }));
    expect(url()).toBe('/documents?labels=work');
    await user.click(screen.getByRole('button', { name: 'back' }));
    expect(url()).toBe('/home');
  });

  it('goes back only once when closed twice before the URL changes (a delete that also drops the id)', async () => {
    const { user, url } = setup(['/home', '/sources']);
    await user.click(screen.getByRole('button', { name: 'open document a1' }));
    await user.click(screen.getByRole('button', { name: 'close document twice' }));
    expect(url()).toBe('/sources');
  });

  it('closes a deep-linked drawer by replacing the URL without its param', async () => {
    const { user, url } = setup(['/home', '/documents?labels=work&document=x7']);
    await user.click(screen.getByRole('button', { name: 'close document' }));
    expect(url()).toBe('/documents?labels=work');
    await user.click(screen.getByRole('button', { name: 'back' }));
    expect(url()).toBe('/home');
  });

  it('switches records and steps without adding history entries', async () => {
    const { user, url } = setup(['/home', '/documents']);
    await user.click(screen.getByRole('button', { name: 'open document a1' }));
    await user.click(screen.getByRole('button', { name: 'open document b2' }));
    await user.click(screen.getByRole('button', { name: 'step document c3' }));
    expect(url()).toBe('/documents?document=c3');
    // Still the drawer this hook opened: closing goes back to the page, not to a1 or b2.
    await user.click(screen.getByRole('button', { name: 'close document' }));
    expect(url()).toBe('/documents');
  });

  it('builds an href that keeps the current params', () => {
    setup(['/search?q=lease&document=x7']);
    expect(screen.getByRole('status', { name: 'document href' })).toHaveTextContent('/search?q=lease&document=z9');
  });

  it('keeps two drawers apart', async () => {
    const { user, url } = setup(['/sources'], <Harness paramKey="source" />);
    await user.click(screen.getByRole('button', { name: 'open source a1' }));
    await user.click(screen.getByRole('button', { name: 'open document b2' }));
    expect(url()).toBe('/sources?source=a1&document=b2');
    await user.click(screen.getByRole('button', { name: 'close document' }));
    expect(url()).toBe('/sources?source=a1');
    expect(screen.getByRole('status', { name: 'source id' })).toHaveTextContent('a1');
  });
});
