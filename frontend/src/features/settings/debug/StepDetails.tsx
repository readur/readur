/* eslint-disable @typescript-eslint/no-explicit-any */
import { useTranslation } from 'react-i18next';
import { Facts, Tag, YesNo } from '../shared/Facts';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { FileAnalysis } from './FileAnalysis';
import { HistoryTable } from './HistoryTable';
import { mb, when, type DebugStep } from './types';
import styles from './Debug.module.css';

/** Per-step diagnostics for the four pipeline stages. Unknown steps show only their error. */
export function StepDetails({ step }: { step: DebugStep }) {
  const { t } = useTranslation();
  const d = step.details ?? {};
  const yes = t('debug.steps.fileInformation.yes');
  const no = t('debug.steps.fileInformation.no');
  const S = 'debug.steps';

  return (
    <div className={shared.stack}>
      {step.error ? <Notice tone="danger">{step.error}</Notice> : null}

      {step.step === 1 ? (
        <>
          <div className={shared.grid2}>
            <Facts
              title={t(`${S}.fileInformation.title`)}
              items={[
                { label: t(`${S}.fileInformation.filename`), value: d.filename, mono: true },
                { label: t(`${S}.fileInformation.original`), value: d.original_filename, mono: true },
                { label: t(`${S}.fileInformation.size`), value: mb(d.file_size), mono: true },
                { label: t(`${S}.fileInformation.mimeType`), value: d.mime_type, mono: true },
                { label: t(`${S}.fileInformation.fileExists`), value: <YesNo value={d.file_exists} yes={yes} no={no} /> },
              ]}
            />
            <Facts
              title={t(`${S}.fileMetadata.title`)}
              items={[
                ...(d.file_metadata
                  ? [
                      { label: t(`${S}.fileMetadata.actualSize`), value: mb(d.file_metadata.size), mono: true },
                      { label: t(`${S}.fileMetadata.isFile`), value: d.file_metadata.is_file ? yes : no },
                      {
                        label: t(`${S}.fileMetadata.modified`),
                        value: d.file_metadata.modified
                          ? new Date(d.file_metadata.modified.secs_since_epoch * 1000).toLocaleString()
                          : t(`${S}.fileMetadata.unknown`),
                        mono: true,
                      },
                    ]
                  : [{ label: t(`${S}.fileMetadata.title`), value: t(`${S}.fileMetadata.notAvailable`) }]),
                { label: t(`${S}.fileMetadata.created`), value: when(d.created_at), mono: true },
              ]}
            />
          </div>
          {d.file_analysis ? <FileAnalysis analysis={d.file_analysis} /> : null}
        </>
      ) : null}

      {step.step === 2 ? (
        <>
          <Facts
            title={t(`${S}.queueStatus.title`)}
            items={[
              { label: t(`${S}.queueStatus.userOcrEnabled`), value: <YesNo value={d.user_ocr_enabled} yes={yes} no={no} /> },
              { label: t(`${S}.queueStatus.queueEntries`), value: d.queue_entries_count, mono: true },
            ]}
          />
          {Array.isArray(d.queue_history) && d.queue_history.length > 0 ? (
            <HistoryTable title={t(`${S}.queueStatus.queueHistory`)} rows={d.queue_history} />
          ) : null}
        </>
      ) : null}

      {step.step === 3 ? (
        <div className={shared.grid2}>
          <Facts
            title={t(`${S}.ocrResults.title`)}
            items={[
              { label: t(`${S}.ocrResults.textLength`), value: `${d.ocr_text_length ?? 0} ${t(`${S}.ocrResults.characters`)}`, mono: true },
              { label: t(`${S}.ocrResults.confidence`), value: d.ocr_confidence ? `${d.ocr_confidence.toFixed(1)}%` : 'N/A', mono: true },
              { label: t(`${S}.ocrResults.wordCount`), value: d.ocr_word_count || 0, mono: true },
              {
                label: t(`${S}.ocrResults.processingTime`),
                value: d.ocr_processing_time_ms ? `${d.ocr_processing_time_ms}ms` : 'N/A',
                mono: true,
              },
              { label: t(`${S}.ocrResults.completedAt`), value: when(d.ocr_completed_at, t(`${S}.ocrResults.notCompleted`)), mono: true },
            ]}
          />
          <Facts
            title={t(`${S}.ocrResults.processingDetails`)}
            items={[
              { label: t(`${S}.ocrResults.hasProcessedImage`), value: <YesNo value={d.has_processed_image} yes={yes} no={no} /> },
              ...(d.processed_image_info
                ? [
                    {
                      label: t(`${S}.ocrResults.imageSize`),
                      value: `${d.processed_image_info.image_width}x${d.processed_image_info.image_height}`,
                      mono: true,
                    },
                    {
                      label: t(`${S}.ocrResults.fileSize`),
                      value: `${((d.processed_image_info.file_size ?? 0) / 1024).toFixed(1)} KB`,
                      mono: true,
                    },
                    {
                      label: t(`${S}.ocrResults.processingSteps`),
                      value: d.processed_image_info.processing_steps?.join(', ') || t(`${S}.ocrResults.none`),
                    },
                    ...(d.processed_image_info.processing_parameters
                      ? [
                          {
                            label: t(`${S}.ocrResults.processingParameters`),
                            value: JSON.stringify(d.processed_image_info.processing_parameters),
                            mono: true,
                          },
                        ]
                      : []),
                  ]
                : []),
            ]}
          />
        </div>
      ) : null}

      {step.step === 4 ? <QualityDetails d={d} /> : null}
    </div>
  );
}

function QualityDetails({ d }: { d: any }) {
  const { t } = useTranslation();
  const Q = 'debug.steps.qualityValidation';
  const th = d.quality_thresholds ?? {};
  const actual = d.actual_values ?? {};
  return (
    <>
      <div className={shared.grid2}>
        <Facts
          title={t(`${Q}.title`)}
          items={[
            { label: t(`${Q}.minConfidence`), value: `${th.min_confidence}%`, mono: true },
            { label: t(`${Q}.brightness`), value: th.brightness_threshold, mono: true },
            { label: t(`${Q}.contrast`), value: th.contrast_threshold, mono: true },
            { label: t(`${Q}.noise`), value: th.noise_threshold, mono: true },
            { label: t(`${Q}.sharpness`), value: th.sharpness_threshold, mono: true },
          ]}
        />
        <Facts
          title={t(`${Q}.actualValues`)}
          items={[
            { label: t(`${Q}.confidence`), value: actual.confidence ? `${actual.confidence.toFixed(1)}%` : 'N/A', mono: true },
            { label: t(`${Q}.wordCount`), value: actual.word_count || 0, mono: true },
            {
              label: t(`${Q}.processedImageAvailable`),
              value: (
                <YesNo
                  value={actual.processed_image_available}
                  yes={t('debug.steps.fileInformation.yes')}
                  no={t('debug.steps.fileInformation.no')}
                />
              ),
            },
            ...(actual.processing_parameters
              ? [{ label: t('debug.steps.ocrResults.processingParameters'), value: JSON.stringify(actual.processing_parameters), mono: true }]
              : []),
          ]}
        />
      </div>
      <div>
        <p className={styles.blockTitle}>{t(`${Q}.qualityChecks`)}</p>
        <ul className={styles.checks}>
          {Object.entries(d.quality_checks || {}).map(([check, passed]) => (
            <li key={check}>
              <Tag tone={passed === true ? 'ok' : passed === false ? 'danger' : 'default'}>
                <span aria-hidden="true">{passed === true ? '■ ' : passed === false ? '▲ ' : '◆ '}</span>
                {check.replace('_check', '').replace('_', ' ')}
                <span className="visually-hidden">
                  {' '}
                  {passed === true
                    ? t('settings.debugTools.checkPassed', 'OK')
                    : passed === false
                      ? t('settings.debugTools.checkFailed', 'failed')
                      : ''}
                </span>
              </Tag>
            </li>
          ))}
        </ul>
      </div>
    </>
  );
}
