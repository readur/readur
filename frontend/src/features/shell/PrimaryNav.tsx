import { Link, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { cx } from '../../ui/shared/FieldParts';
import { DESTINATIONS, activeDestination } from './destinations';
import styles from './AppShell.module.css';

export interface PrimaryNavProps {
  /** `top` sits in the header; `bottom` is the fixed mobile tab bar. */
  variant: 'top' | 'bottom';
  label: string;
}

/** The four destinations. The current one carries `aria-current="page"`. */
export function PrimaryNav({ variant, label }: PrimaryNavProps) {
  const { t } = useTranslation();
  const { pathname } = useLocation();
  const active = activeDestination(pathname);

  return (
    <nav aria-label={label} className={variant === 'top' ? styles.topNav : styles.bottomBar}>
      <ul className={styles.navList}>
        {DESTINATIONS.map((d) => {
          const isActive = d.id === active;
          return (
            <li key={d.id} className={styles.navItem}>
              <Link
                to={d.path}
                className={cx(styles.navLink, isActive && styles.navLinkActive)}
                aria-current={isActive ? 'page' : undefined}
              >
                <span className={styles.navIcon} aria-hidden="true">
                  {d.icon}
                </span>
                <span className={styles.navLabel}>{t(d.labelKey, d.fallback)}</span>
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
