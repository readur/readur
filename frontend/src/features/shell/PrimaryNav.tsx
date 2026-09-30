import { Link, useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { cx } from '../../ui/shared/FieldParts';
import { DESTINATIONS, activeDestination } from './destinations';
import styles from './Sidebar.module.css';
import shellStyles from './AppShell.module.css';

export interface PrimaryNavProps {
  /** `side` is the sidebar list; `bottom` is the fixed phone tab bar (no Settings). */
  variant: 'side' | 'bottom';
  label: string;
}

/** The destinations. The current one carries `aria-current="page"`. */
export function PrimaryNav({ variant, label }: PrimaryNavProps) {
  const { t } = useTranslation();
  const { pathname } = useLocation();
  const active = activeDestination(pathname);
  const side = variant === 'side';
  const items = side ? DESTINATIONS : DESTINATIONS.filter((d) => d.inTabBar);

  return (
    <nav aria-label={label} className={side ? styles.primaryNav : shellStyles.bottomBar}>
      <ul className={side ? styles.navList : shellStyles.tabList}>
        {items.map((d) => {
          const isActive = d.id === active;
          return (
            <li key={d.id} className={side ? undefined : shellStyles.tabItem}>
              <Link
                to={d.path}
                className={cx(
                  side ? styles.navLink : shellStyles.tabLink,
                  isActive && (side ? styles.navLinkActive : shellStyles.tabLinkActive),
                )}
                aria-current={isActive ? 'page' : undefined}
              >
                <span className={side ? styles.navIcon : shellStyles.tabIcon} aria-hidden="true">
                  {d.icon}
                </span>
                <span className={side ? styles.navLabel : shellStyles.tabLabel}>{t(d.labelKey, d.fallback)}</span>
              </Link>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
