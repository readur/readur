import { cx } from '../shared/FieldParts';
import styles from './Avatar.module.css';

export interface AvatarProps {
  name: string;
  size?: 'sm' | 'md';
  className?: string;
}

const initials = (name: string) => {
  const parts = name.trim().split(/\s+/).filter(Boolean);
  const raw = parts.length > 1 ? parts[0][0] + parts[parts.length - 1][0] : name.trim().slice(0, 2);
  return raw.toUpperCase();
};

/** A person's initials in an accent disc; the full name is the accessible name. */
export function Avatar({ name, size = 'md', className }: AvatarProps) {
  return (
    <span role="img" aria-label={name} className={cx(styles.avatar, styles[size], className)}>
      <span aria-hidden="true">{initials(name)}</span>
    </span>
  );
}
