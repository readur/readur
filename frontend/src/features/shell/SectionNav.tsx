import { Link } from 'react-router-dom';
import styles from './SectionNav.module.css';

export interface SectionNavItem {
  id: string;
  label: string;
  to: string;
}

export interface SectionNavProps {
  label: string;
  items: readonly SectionNavItem[];
  activeId: string;
}

/** Row of in-page section links; the active one carries `aria-current="page"`. */
export function SectionNav({ label, items, activeId }: SectionNavProps) {
  return (
    <nav aria-label={label} className={styles.nav}>
      <ul className={styles.list}>
        {items.map((item) => (
          <li key={item.id}>
            <Link
              to={item.to}
              className={styles.link}
              aria-current={item.id === activeId ? 'page' : undefined}
            >
              {item.label}
            </Link>
          </li>
        ))}
      </ul>
    </nav>
  );
}
