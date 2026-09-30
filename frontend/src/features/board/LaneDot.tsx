import styles from './Home.module.css';

/** The colour dot of a source (decorative: the name always sits beside it). */
export function LaneDot({ hue }: { hue: number }) {
  return <span className={styles.dot} data-hue={hue} aria-hidden="true" />;
}
