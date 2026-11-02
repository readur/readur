import { useTranslation } from 'react-i18next';
import { STATE_GLYPH, stepState, type DebugStep } from './types';
import { StepDetails } from './StepDetails';
import styles from './Debug.module.css';

/** The processing pipeline as a vertical ordered list: shape + status word per step, details below. */
export function PipelineStepper({ steps }: { steps: DebugStep[] }) {
  const { t } = useTranslation();
  return (
    <section aria-labelledby="debug-pipeline" className={styles.block}>
      <h3 id="debug-pipeline" className={styles.blockTitle}>
        {t('debug.pipeline.title')}
      </h3>
      <ol className={styles.stepper}>
        {steps.map((step) => {
          const state = stepState(step.status, step.success);
          return (
            <li key={step.step} className={styles.step} data-state={state}>
              <div className={styles.stepHead}>
                <span className={styles.stepGlyph} aria-hidden="true">
                  {STATE_GLYPH[state]}
                </span>
                <span className={styles.stepName}>{step.name}</span>
                <span className={styles.stepStatus}>{step.status}</span>
              </div>
              <div className={styles.stepBody}>
                <StepDetails step={step} />
              </div>
            </li>
          );
        })}
      </ol>
    </section>
  );
}
