import type { ReactNode } from 'react';
import {
  Button,
  ListBox,
  ListBoxItem,
  Popover,
  Select as RACSelect,
  SelectValue,
  type ListBoxItemProps,
  type SelectProps as RACSelectProps,
} from 'react-aria-components';
import { ExpandMore } from '../icons';
import { cx, FieldHelp, FieldLabel, type FieldChromeProps } from '../shared/FieldParts';
import fieldStyles from '../shared/field.module.css';
import lb from '../shared/listbox.module.css';

export interface SelectProps<T extends object>
  extends Omit<RACSelectProps<T>, 'children' | 'className'>,
    FieldChromeProps {
  items?: Iterable<T>;
  /** `SelectItem` elements, or a render function when `items` is given. */
  children: ReactNode | ((item: T) => ReactNode);
  className?: string;
}

export function Select<T extends object>({
  label,
  description,
  errorMessage,
  items,
  children,
  className,
  ...rest
}: SelectProps<T>) {
  return (
    <RACSelect {...rest} className={cx(fieldStyles.field, className)}>
      <FieldLabel label={label} />
      <Button className={cx(fieldStyles.control, lb.trigger)}>
        <SelectValue />
        <span className={lb.chevron} aria-hidden="true">
          <ExpandMore fontSize="small" />
        </span>
      </Button>
      <FieldHelp description={description} errorMessage={errorMessage} />
      <Popover className={lb.popover}>
        <ListBox className={lb.listbox} items={items}>
          {children}
        </ListBox>
      </Popover>
    </RACSelect>
  );
}

export function SelectItem(props: Omit<ListBoxItemProps, 'className'>) {
  return <ListBoxItem {...props} className={lb.option} />;
}
