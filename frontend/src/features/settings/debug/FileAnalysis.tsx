/* eslint-disable @typescript-eslint/no-explicit-any */
import { useTranslation } from 'react-i18next';
import { Facts, YesNo } from '../shared/Facts';
import { Notice } from '../shared/Notice';
import shared from '../shared/shared.module.css';
import { mb } from './types';
import styles from './Debug.module.css';

/** Detailed file analysis for the upload step: readability, PDF internals or a text preview. */
export function FileAnalysis({ analysis: a }: { analysis: any }) {
  const { t } = useTranslation();
  const F = 'debug.steps.fileAnalysis';
  const yes = t('debug.steps.fileInformation.yes');
  const no = t('debug.steps.fileInformation.no');
  const pdf = a.pdf_info;

  return (
    <div className={shared.stack}>
      <p className={styles.blockTitle}>{t(`${F}.title`)}</p>
      <div className={shared.grid2}>
        <div className={shared.stack}>
          <Facts
            title={t(`${F}.basicAnalysis`)}
            items={[
              { label: t(`${F}.fileType`), value: a.file_type, mono: true },
              { label: t(`${F}.size`), value: mb(a.file_size_bytes), mono: true },
              { label: t(`${F}.readable`), value: <YesNo value={a.is_readable} yes={yes} no={no} /> },
            ]}
          />
          {a.error_details ? (
            <Notice tone="danger">
              <strong>{t(`${F}.fileError`)}</strong> {a.error_details}
            </Notice>
          ) : null}
        </div>
        {pdf ? (
          <div className={shared.stack}>
            <Facts
              title={t(`${F}.pdfAnalysis`)}
              items={[
                { label: t(`${F}.validPdf`), value: <YesNo value={pdf.is_valid_pdf} yes={yes} no={no} /> },
                { label: t(`${F}.pdfVersion`), value: pdf.pdf_version || t('debug.steps.fileMetadata.unknown'), mono: true },
                { label: t(`${F}.pages`), value: pdf.page_count || t('debug.steps.fileMetadata.unknown'), mono: true },
                { label: t(`${F}.hasText`), value: <YesNo value={pdf.has_text_content} yes={yes} no={no} /> },
                { label: t(`${F}.hasImages`), value: <YesNo value={pdf.has_images} yes={yes} no={no} /> },
                { label: t(`${F}.encrypted`), value: <YesNo value={pdf.is_encrypted} yes={yes} no={no} /> },
                { label: t(`${F}.fontCount`), value: pdf.font_count, mono: true },
                { label: t(`${F}.textLength`), value: `${pdf.estimated_text_length} ${t(`${F}.chars`)}`, mono: true },
              ]}
            />
            {pdf.text_extraction_error ? (
              <Notice tone="danger">
                <strong>{t(`${F}.pdfTextExtractionError`)}</strong> {pdf.text_extraction_error}
              </Notice>
            ) : null}
          </div>
        ) : a.text_preview ? (
          <div className={shared.factsBox}>
            <p className={shared.factsTitle}>{t(`${F}.textPreview`)}</p>
            <pre className={styles.preview}>{a.text_preview}</pre>
          </div>
        ) : (
          <div className={shared.factsBox}>
            <p className={shared.factsTitle}>{t(`${F}.fileContent`)}</p>
            <p className={shared.meta}>{t(`${F}.noPreview`)}</p>
          </div>
        )}
      </div>
    </div>
  );
}
