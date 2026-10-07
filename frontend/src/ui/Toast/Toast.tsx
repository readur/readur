import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
} from 'react';
import {
  Button,
  Text,
  UNSTABLE_Toast as RACToast,
  UNSTABLE_ToastContent as ToastContent,
  UNSTABLE_ToastQueue as ToastQueue,
  UNSTABLE_ToastRegion as ToastRegion,
} from 'react-aria-components';
import { useTranslation } from 'react-i18next';
import { Close, Error as ErrorIcon, Info, CheckCircle } from '../icons';
import { cx } from '../shared/FieldParts';
import { ProgressBar } from '../Progress';
import { Spinner } from '../Spinner';
import styles from './Toast.module.css';

export type ToastTone = 'info' | 'success' | 'danger';

export interface ToastOptions {
  title: string;
  description?: string;
  tone?: ToastTone;
  /** Auto-dismiss delay in ms. Default 5000. */
  timeout?: number;
}

export interface ProgressToastOptions {
  title: string;
  description?: string;
  /** 0–100; omit for an indeterminate spinner. */
  value?: number;
}

export interface ToastApi {
  show: (options: ToastOptions) => void;
  /**
   * Shows or updates the long-running toast `id` (a sync, an import). It has no timeout: the
   * owner calls `dismiss(id)` when the work ends. Once the person closes it, updates stop
   * bringing it back until it is dismissed.
   */
  progress: (id: string, options: ProgressToastOptions) => void;
  dismiss: (id: string) => void;
}

interface ToastContentValue {
  title: string;
  description?: string;
  tone: ToastTone;
  /** Set for a progress toast; its live text and value are read from the provider by id. */
  progressId?: string;
}

const DEFAULT_TIMEOUT = 5000;

const TONE_DEFAULT = { info: 'Info:', success: 'Success:', danger: 'Error:' } as const;

const TONE_ICON = { info: Info, success: CheckCircle, danger: ErrorIcon } as const;

const ToastContext = createContext<ToastApi | null>(null);

/**
 * Space an open overlay (a SlideOver) claims at the screen's right or bottom edge. The toast
 * region moves out of it so toasts never cover the overlay's controls.
 */
export interface ToastInset {
  right: number;
  bottom: number;
}

type InsetReporter = (owner: symbol, inset: ToastInset | null) => void;

const ToastInsetContext = createContext<InsetReporter | null>(null);

/** Lets an overlay claim screen space the toasts must avoid; `null` releases it. No-op outside a provider. */
export function useToastInsetReporter(): InsetReporter {
  return useContext(ToastInsetContext) ?? NOOP_REPORTER;
}

const NOOP_REPORTER: InsetReporter = () => undefined;

/** Mount once near the app root. Renders the toast region (bottom-right, clear of an open SlideOver). */
export function ToastProvider({ children }: { children: ReactNode }) {
  const { t } = useTranslation();
  const [queue] = useState(() => new ToastQueue<ToastContentValue>({ maxVisibleToasts: 5 }));
  const [insets, setInsets] = useState<ReadonlyMap<symbol, ToastInset>>(() => new Map());
  const reportInset = useCallback<InsetReporter>((owner, inset) => {
    setInsets((prev) => {
      const current = prev.get(owner);
      if (inset ? current?.right === inset.right && current?.bottom === inset.bottom : !current) return prev;
      const next = new Map(prev);
      if (inset) next.set(owner, inset);
      else next.delete(owner);
      return next;
    });
  }, []);
  let right = 0;
  let bottom = 0;
  for (const inset of insets.values()) {
    right = Math.max(right, inset.right);
    bottom = Math.max(bottom, inset.bottom);
  }
  const regionStyle = {
    '--toast-inset-right': `${right}px`,
    '--toast-inset-bottom': `${bottom}px`,
  } as CSSProperties;
  // Hovering a toast holds every timer (and the drain bars) until the pointer leaves the stack.
  // Listen natively on the region: it lets pointer events through to the toasts, so React Aria's
  // own region hover cannot be relied on. The region only exists while toasts do, hence the
  // callback ref.
  const [region, setRegion] = useState<HTMLDivElement | null>(null);
  const [paused, setPaused] = useState(false);
  useEffect(() => {
    if (!region) return undefined;
    const onOver = (e: PointerEvent) => {
      if (e.pointerType === 'touch') return;
      setPaused(true);
    };
    const onOut = (e: PointerEvent) => {
      const next = e.relatedTarget as Node | null;
      if (next && region.contains(next)) return;
      setPaused(false);
    };
    region.addEventListener('pointerover', onOver);
    region.addEventListener('pointerout', onOut);
    return () => {
      region.removeEventListener('pointerover', onOver);
      region.removeEventListener('pointerout', onOut);
      setPaused(false);
    };
  }, [region]);
  // Only act on a change: React Aria's Timer.resume() starts a second timeout if one is running.
  const wasPaused = useRef(false);
  useEffect(() => {
    if (paused !== wasPaused.current) {
      if (paused) queue.pauseAll();
      else queue.resumeAll();
      wasPaused.current = paused;
    }
    if (paused) region?.setAttribute('data-paused', 'true');
    else region?.removeAttribute('data-paused');
  }, [paused, queue, region]);
  // Progress toasts: the queue holds one entry per id; updates only change this map, so the
  // toast updates in place instead of leaving and re-entering.
  const [progressById, setProgressById] = useState<ReadonlyMap<string, ProgressToastOptions>>(() => new Map());
  const progressKeys = useRef(new Map<string, string>());
  const closedByPerson = useRef(new Set<string>());
  const api = useMemo<ToastApi>(
    () => ({
      show: ({ title, description, tone = 'info', timeout = DEFAULT_TIMEOUT }) => {
        queue.add({ title, description, tone }, { timeout });
      },
      progress: (id, options) => {
        if (closedByPerson.current.has(id)) return;
        setProgressById((prev) => new Map(prev).set(id, options));
        if (progressKeys.current.has(id)) return;
        const key = queue.add(
          { title: options.title, tone: 'info', progressId: id },
          {
            onClose: () => {
              // Closed by the person (dismiss() deletes the key first).
              if (progressKeys.current.delete(id)) closedByPerson.current.add(id);
            },
          },
        );
        progressKeys.current.set(id, key);
      },
      dismiss: (id) => {
        closedByPerson.current.delete(id);
        const key = progressKeys.current.get(id);
        progressKeys.current.delete(id);
        if (key) queue.close(key);
        setProgressById((prev) => {
          if (!prev.has(id)) return prev;
          const next = new Map(prev);
          next.delete(id);
          return next;
        });
      },
    }),
    [queue],
  );

  return (
    <ToastContext.Provider value={api}>
      <ToastInsetContext.Provider value={reportInset}>{children}</ToastInsetContext.Provider>
      <ToastRegion
        ref={setRegion}
        queue={queue}
        className={styles.region}
        style={regionStyle}
        aria-label={t('ui.notifications', 'Notifications')}
      >
        {({ toast }) => {
          const live = toast.content.progressId ? progressById.get(toast.content.progressId) : undefined;
          const title = live?.title ?? toast.content.title;
          const description = live ? live.description : toast.content.description;
          return (
          <RACToast
            toast={toast}
            className={cx(styles.toast, styles[toast.content.tone])}
            style={{ '--toast-timeout': `${toast.timeout ?? 0}ms` } as CSSProperties}
          >
            <span className={styles.toneIcon} aria-hidden="true">
              {(() => {
                const Icon = TONE_ICON[toast.content.tone];
                return <Icon fontSize="small" />;
              })()}
            </span>
            <ToastContent className={styles.content} role={toast.content.tone === 'danger' ? 'alert' : 'status'}>
              <Text slot="title" className={styles.title}>
                <span className="visually-hidden">{t(`ui.toast.${toast.content.tone}`, TONE_DEFAULT[toast.content.tone])} </span>
                {title}
              </Text>
              {description ? (
                <Text slot="description" className={styles.description}>
                  {description}
                </Text>
              ) : null}
              {toast.content.progressId ? (
                live?.value !== undefined ? (
                  <ProgressBar value={live.value} label={title} className={styles.progress} />
                ) : (
                  <Spinner size={14} />
                )
              ) : null}
            </ToastContent>
            <Button slot="close" className={styles.close} aria-label={t('ui.close', 'Close')}>
              <Close fontSize="small" />
            </Button>
            {toast.timeout ? <span className={styles.timer} aria-hidden="true" /> : null}
          </RACToast>
          );
        }}
      </ToastRegion>
    </ToastContext.Provider>
  );
}

/** Returns `{ show }`. Outside a provider it is a no-op so isolated component tests keep working. */
export function useToast(): ToastApi {
  return useContext(ToastContext) ?? NOOP;
}

const NOOP: ToastApi = { show: () => undefined, progress: () => undefined, dismiss: () => undefined };
