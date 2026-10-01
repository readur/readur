import {
  Tab as RACTab,
  TabList as RACTabList,
  TabPanel as RACTabPanel,
  Tabs as RACTabs,
  type TabListProps,
  type TabPanelProps,
  type TabProps,
  type TabsProps,
} from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Tabs.module.css';

export function Tabs({ className, ...rest }: Omit<TabsProps, 'className'> & { className?: string }) {
  return <RACTabs {...rest} className={cx(styles.tabs, className)} />;
}

export function TabList<T extends object>({
  className,
  ...rest
}: Omit<TabListProps<T>, 'className'> & { className?: string }) {
  return <RACTabList {...rest} className={cx(styles.list, className)} />;
}

export function Tab({ className, ...rest }: Omit<TabProps, 'className'> & { className?: string }) {
  return <RACTab {...rest} className={cx(styles.tab, className)} />;
}

export function TabPanel({ className, ...rest }: Omit<TabPanelProps, 'className'> & { className?: string }) {
  return <RACTabPanel {...rest} className={cx(styles.panel, className)} />;
}
