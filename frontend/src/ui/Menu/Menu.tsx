import type { ReactNode } from 'react';
import {
  Header,
  Menu as RACMenu,
  MenuSection as RACMenuSection,
  MenuItem as RACMenuItem,
  Popover as RACPopover,
  type MenuItemProps as RACMenuItemProps,
  type MenuProps as RACMenuProps,
} from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Menu.module.css';

export { MenuTrigger } from 'react-aria-components';

export type MenuProps<T extends object> = Omit<RACMenuProps<T>, 'className'> & { className?: string };

/** Menu list in a popover. Compose as `<MenuTrigger><Button/><Menu>…</Menu></MenuTrigger>`. */
export function Menu<T extends object>({ className, ...rest }: MenuProps<T>) {
  return (
    <RACPopover className={styles.popover} offset={4}>
      <RACMenu {...rest} className={cx(styles.menu, className)} />
    </RACPopover>
  );
}

export interface MenuItemProps extends Omit<RACMenuItemProps, 'className'> {
  /** Destructive tone. */
  danger?: boolean;
  className?: string;
}

export function MenuItem({ danger, className, ...rest }: MenuItemProps) {
  return <RACMenuItem {...rest} className={cx(styles.item, danger && styles.danger, className)} />;
}

/** A titled group of menu items (renders a named group with an uppercase heading). */
export function MenuSection({ title, children }: { title: string; children: ReactNode }) {
  return (
    <RACMenuSection className={styles.section}>
      <Header className={styles.sectionTitle}>{title}</Header>
      {children}
    </RACMenuSection>
  );
}
