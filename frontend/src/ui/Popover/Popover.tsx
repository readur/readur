import { Popover as RACPopover, type PopoverProps as RACPopoverProps } from 'react-aria-components';
import { cx } from '../shared/FieldParts';
import styles from './Popover.module.css';

export type PopoverProps = Omit<RACPopoverProps, 'className'> & { className?: string };

/** Anchored surface. Use inside `DialogTrigger` from react-aria-components (re-exported here). */
export function Popover({ className, offset = 4, ...rest }: PopoverProps) {
  return <RACPopover {...rest} offset={offset} className={cx(styles.popover, className)} />;
}

export { DialogTrigger as PopoverTrigger } from 'react-aria-components';
