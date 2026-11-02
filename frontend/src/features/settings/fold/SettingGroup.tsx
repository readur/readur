import { useEffect, useId, useRef, useState, type ReactNode } from 'react';
import { useLocation } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { Button } from '../../../ui';
import { usePrefersReducedMotion } from '../../../ui/motion';
import styles from './SettingGroup.module.css';

export interface SettingGroupProps {
  /** Stable id; `#id` in the URL opens the group on arrival. */
  id: string;
  title: string;
  /** Current values in one line, e.g. "ENG, DEU · 4 jobs · 300s". */
  summary?: ReactNode;
  children: ReactNode;
  /** Open on first render. */
  defaultExpanded?: boolean;
}

/** Fallback for browsers/tests where `transitionend` never fires. */
const COLLAPSE_FALLBACK_MS = 260;

/**
 * A setting group that starts as a one-line summary and unfolds in place into its form.
 * The toggle button carries `aria-expanded`/`aria-controls`; the region animates its height
 * (instantly under reduced motion) and is `hidden` while collapsed.
 */
export function SettingGroup({ id, title, summary, children, defaultExpanded = false }: SettingGroupProps) {
  const { t } = useTranslation();
  const { hash } = useLocation();
  const reduced = usePrefersReducedMotion();
  const uid = useId();
  const regionId = `${id}-region-${uid}`;
  const headingId = `${id}-heading-${uid}`;
  const targeted = hash === `#${id}`;
  const [expanded, setExpanded] = useState(defaultExpanded || targeted);
  // `hidden` lags behind `expanded` on collapse so the height can animate first.
  const [hidden, setHidden] = useState(!(defaultExpanded || targeted));
  const [open, setOpen] = useState(defaultExpanded || targeted);
  const rootRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (!targeted) return;
    setExpanded(true);
    const el = rootRef.current;
    if (el && typeof el.scrollIntoView === 'function') el.scrollIntoView({ block: 'start' });
  }, [targeted]);

  useEffect(() => {
    if (expanded) {
      setHidden(false);
      if (reduced) {
        setOpen(true);
        return undefined;
      }
      // Let the collapsed layout paint before opening so the height transitions.
      const frame = requestAnimationFrame(() => setOpen(true));
      return () => cancelAnimationFrame(frame);
    }
    setOpen(false);
    if (reduced) {
      setHidden(true);
      return undefined;
    }
    const timer = window.setTimeout(() => setHidden(true), COLLAPSE_FALLBACK_MS);
    return () => window.clearTimeout(timer);
  }, [expanded, reduced]);

  return (
    <div ref={rootRef} id={id} className={styles.group} data-expanded={expanded || undefined}>
      <div className={styles.summaryRow}>
        <h3 id={headingId} className={styles.name}>
          {title}
        </h3>
        <p className={styles.summary}>{summary}</p>
        <Button
          size="sm"
          variant={expanded ? 'ghost' : 'secondary'}
          aria-expanded={expanded}
          aria-controls={regionId}
          onPress={() => setExpanded((v) => !v)}
          className={styles.toggle}
        >
          {`${expanded ? t('settings.fold.close', 'Close') : t('settings.fold.edit', 'Edit')} `}
          <span className="visually-hidden">{title}</span>
        </Button>
      </div>
      <div
        id={regionId}
        role="region"
        aria-labelledby={headingId}
        className={styles.region}
        data-open={open || undefined}
        hidden={hidden}
        onTransitionEnd={(e) => {
          if (e.target === e.currentTarget && !expanded) setHidden(true);
        }}
      >
        <div className={styles.inner}>
          <div className={styles.body}>{children}</div>
        </div>
      </div>
    </div>
  );
}
