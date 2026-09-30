import type { CSSProperties } from 'react';
import { Label as RACLabel, Radio, RadioGroup } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { LABEL_COLORS, swatchStyle } from './labelData';
import styles from './Labels.module.css';

export interface ColorChoicesProps {
  /** A hex colour; a value outside the presets selects none of them. */
  value: string;
  onChange: (color: string) => void;
  label: string;
  isDisabled?: boolean;
}

/** The preset label colours as a row of named swatches (one radio group). */
export function ColorChoices({ value, onChange, label, isDisabled }: ColorChoicesProps) {
  const { t } = useTranslation();
  return (
    <RadioGroup
      className={styles.choiceGroup}
      value={LABEL_COLORS.some((c) => c.value === value) ? value : null}
      onChange={onChange}
      isDisabled={isDisabled}
    >
      <RACLabel className={styles.fieldLabel}>{label}</RACLabel>
      <div className={styles.choices}>
        {LABEL_COLORS.map((c) => (
          <Radio key={c.value} value={c.value} className={styles.colorChoice} aria-label={t(c.nameKey, c.fallback)}>
            <span className={styles.colorDot} style={swatchStyle(c.value) as CSSProperties} aria-hidden="true" />
          </Radio>
        ))}
      </div>
    </RadioGroup>
  );
}
