import { useTranslation } from 'react-i18next';
import { IconButton } from '../../ui';
import { Brightness4, Brightness7 } from '../../ui/icons';
import { useThemeMode } from '../../theme/useThemeMode';

/** Flips light/dark. The label names the mode it switches to. */
export function ThemeToggle() {
  const { t } = useTranslation();
  const { mode, toggle } = useThemeMode();
  const toDark = mode === 'light';
  return (
    <IconButton
      label={toDark ? t('shell.theme.toDark', 'Switch to dark mode') : t('shell.theme.toLight', 'Switch to light mode')}
      icon={toDark ? <Brightness4 fontSize="inherit" /> : <Brightness7 fontSize="inherit" />}
      onPress={toggle}
    />
  );
}
