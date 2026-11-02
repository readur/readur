import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Button } from '../Button';
import { Menu, MenuItem, MenuSection, MenuTrigger } from './Menu';

const setup = (onAction = vi.fn()) =>
  render(
    <MenuTrigger>
      <Button>Actions</Button>
      <Menu aria-label="Document actions" onAction={onAction}>
        <MenuItem id="edit">Edit</MenuItem>
        <MenuItem id="delete" danger>
          Delete
        </MenuItem>
      </Menu>
    </MenuTrigger>,
  );

describe('Menu', () => {
  it('opens from its trigger and exposes named menu and items', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('button', { name: 'Actions' }));
    expect(await screen.findByRole('menu', { name: 'Actions' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Edit' })).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: 'Delete' })).toBeInTheDocument();
  });

  it('navigates with arrow keys and activates with Enter', async () => {
    const onAction = vi.fn();
    const user = userEvent.setup();
    setup(onAction);
    await user.tab();
    await user.keyboard('{Enter}');
    await screen.findByRole('menu');
    await user.keyboard('{ArrowDown}{Enter}');
    expect(onAction).toHaveBeenCalled();
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('closes with Escape', async () => {
    const user = userEvent.setup();
    setup();
    await user.click(screen.getByRole('button', { name: 'Actions' }));
    await screen.findByRole('menu');
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('groups items under a named section', async () => {
    render(
      <MenuTrigger>
        <Button>Open</Button>
        <Menu aria-label="Actions">
          <MenuSection title="Document">
            <MenuItem>Download</MenuItem>
          </MenuSection>
        </Menu>
      </MenuTrigger>,
    );
    await userEvent.click(screen.getByRole('button', { name: 'Open' }));
    expect(await screen.findByRole('group', { name: 'Document' })).toBeInTheDocument();
  });
});
