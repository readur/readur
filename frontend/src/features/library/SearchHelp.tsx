import { useId, useState } from 'react';
import { Dialog as RACDialog, Heading, Label, Radio, RadioGroup } from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import type { SearchMode } from '../../types/generated';
import { IconButton, Popover, PopoverTrigger } from '../../ui';
import { Help } from '../../ui/icons';
import styles from './Library.module.css';

const MODES: { mode: SearchMode; key: string; fallback: string; hintKey: string; hint: string }[] = [
  { mode: 'simple', key: 'library.help.simple', fallback: 'All words', hintKey: 'library.help.simpleHint', hint: 'Every word appears somewhere' },
  { mode: 'phrase', key: 'library.help.phrase', fallback: 'Exact phrase', hintKey: 'library.help.phraseHint', hint: 'The words appear together, in order' },
  { mode: 'boolean', key: 'library.help.boolean', fallback: 'Operators', hintKey: 'library.help.booleanHint', hint: 'Use & (and), | (or), ! (not)' },
  { mode: 'fuzzy', key: 'library.help.fuzzy', fallback: 'Similar spelling', hintKey: 'library.help.fuzzyHint', hint: 'Also finds near misses and typos' },
];

const EXAMPLES: { q: string; mode: SearchMode; key: string; fallback: string }[] = [
  { q: 'invoice', mode: 'simple', key: 'library.help.exampleWord', fallback: 'Documents that mention a word' },
  { q: 'project proposal', mode: 'phrase', key: 'library.help.examplePhrase', fallback: 'An exact phrase' },
  { q: 'invoice & paid', mode: 'boolean', key: 'library.help.exampleAnd', fallback: 'Both words' },
  { q: 'budget | forecast', mode: 'boolean', key: 'library.help.exampleOr', fallback: 'Either word' },
  { q: 'contract & !draft', mode: 'boolean', key: 'library.help.exampleNot', fallback: 'Leave a word out' },
  { q: 'reciept', mode: 'fuzzy', key: 'library.help.exampleFuzzy', fallback: 'Tolerates typos' },
];

interface SearchHelpProps {
  mode: SearchMode | null;
  onModeChange: (mode: SearchMode | null) => void;
  /** Run an example: sets the query and the matching mode. */
  onExample: (q: string, mode: SearchMode | null) => void;
}

/** How matching works, a mode switch, and example searches to try. */
export function SearchHelp({ mode, onModeChange, onExample }: SearchHelpProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const examplesId = useId();
  return (
    <PopoverTrigger isOpen={open} onOpenChange={setOpen}>
      <IconButton variant="ghost" label={t('library.help.open', 'Search help')} icon={<Help fontSize="small" />} />
      <Popover placement="bottom end">
        <RACDialog className={styles.help} aria-label={t('library.help.title', 'Search help')}>
          <Heading slot="title" className={styles.helpHeading}>
            {t('library.help.title', 'Search help')}
          </Heading>
          <p className={styles.helpText}>
            {t('library.help.intro', 'Search looks through file names and the text read from each document.')}
          </p>
          <RadioGroup
            className={styles.helpModes}
            value={mode ?? 'simple'}
            onChange={(v) => onModeChange(v === 'simple' ? null : (v as SearchMode))}
          >
            <Label className={styles.helpLabel}>{t('library.help.match', 'Match')}</Label>
            {MODES.map((m) => (
              <Radio key={m.mode} value={m.mode} className={styles.radio}>
                <span>{t(m.key, m.fallback)}</span>
                <span className={styles.radioHint}>{t(m.hintKey, m.hint)}</span>
              </Radio>
            ))}
          </RadioGroup>
          <span className={styles.helpLabel} id={examplesId}>
            {t('library.help.examples', 'Try')}
          </span>
          <ul className={styles.examples} aria-labelledby={examplesId}>
            {EXAMPLES.map((ex) => (
              <li key={ex.q}>
                <button
                  type="button"
                  className={styles.example}
                  onClick={() => {
                    onExample(ex.q, ex.mode === 'simple' ? null : ex.mode);
                    setOpen(false);
                  }}
                >
                  <code className={styles.exampleQuery}>{ex.q}</code>{' '}
                  <span className={styles.exampleText}>{t(ex.key, ex.fallback)}</span>
                </button>
              </li>
            ))}
          </ul>
        </RACDialog>
      </Popover>
    </PopoverTrigger>
  );
}
