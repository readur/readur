import {
  Button,
  Input,
  SearchField as RACSearchField,
  type SearchFieldProps as RACSearchFieldProps,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Close, Search } from '../icons';
import { cx, FieldHelp, FieldLabel, type FieldChromeProps } from '../shared/FieldParts';
import fieldStyles from '../shared/field.module.css';
import styles from './SearchField.module.css';

export interface SearchFieldProps extends Omit<RACSearchFieldProps, 'children' | 'className'>, FieldChromeProps {
  placeholder?: string;
  className?: string;
}

/** Search input with a clear button. Esc clears the value. */
export function SearchField({ label, description, errorMessage, placeholder, className, ...rest }: SearchFieldProps) {
  const { t } = useTranslation();
  return (
    <RACSearchField {...rest} className={cx(fieldStyles.field, styles.field, className)}>
      <FieldLabel label={label} />
      <div className={styles.wrap}>
        <span className={styles.searchIcon} aria-hidden="true">
          <Search fontSize="small" />
        </span>
        <Input className={cx(fieldStyles.control, styles.input)} placeholder={placeholder} />
        <Button className={styles.clear} aria-label={t('ui.clear', 'Clear')}>
          <Close fontSize="small" />
        </Button>
      </div>
      <FieldHelp description={description} errorMessage={errorMessage} />
    </RACSearchField>
  );
}
