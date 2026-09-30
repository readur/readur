import type { ReactNode } from 'react';
import {
  Button,
  ComboBox as RACComboBox,
  Input,
  ListBox,
  ListBoxItem,
  Popover,
  type ComboBoxProps as RACComboBoxProps,
  type ListBoxItemProps,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { ExpandMore } from '../icons';
import { cx, FieldHelp, FieldLabel, type FieldChromeProps } from '../shared/FieldParts';
import fieldStyles from '../shared/field.module.css';
import lb from '../shared/listbox.module.css';

export interface ComboBoxProps<T extends object>
  extends Omit<RACComboBoxProps<T>, 'children' | 'className'>,
    FieldChromeProps {
  placeholder?: string;
  /** `ComboBoxItem` elements, or a render function when `items`/`defaultItems` is given. */
  children: ReactNode | ((item: T) => ReactNode);
  className?: string;
}

export function ComboBox<T extends object>({
  label,
  description,
  errorMessage,
  placeholder,
  children,
  className,
  ...rest
}: ComboBoxProps<T>) {
  const { t } = useTranslation();
  return (
    <RACComboBox {...rest} className={cx(fieldStyles.field, className)}>
      <FieldLabel label={label} />
      <div className={lb.group}>
        <Input className={fieldStyles.control} placeholder={placeholder} />
        <Button className={lb.groupButton} aria-label={t('ui.showOptions', 'Show options')}>
          <ExpandMore fontSize="small" />
        </Button>
      </div>
      <FieldHelp description={description} errorMessage={errorMessage} />
      <Popover className={lb.popover}>
        <ListBox className={lb.listbox}>{children}</ListBox>
      </Popover>
    </RACComboBox>
  );
}

export function ComboBoxItem(props: Omit<ListBoxItemProps, 'className'>) {
  return <ListBoxItem {...props} className={lb.option} />;
}
