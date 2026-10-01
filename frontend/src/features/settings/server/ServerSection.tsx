import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { api } from '../../../services/api';
import type { ServerConfiguration } from '../../../types/generated';
import { Button, Pass, PassCell, Skeleton, useToast } from '../../../ui';
import { CloudSync } from '../../../ui/icons';
import { SettingGroup } from '../fold/SettingGroup';
import { Notice } from '../shared/Notice';
import { Tag } from '../shared/Facts';
import shared from '../shared/shared.module.css';

const P = 'settings.serverConfiguration';

/** Read-only server configuration from `GET /settings/config`, one fold per area. */
export default function ServerSection() {
  const { t } = useTranslation();
  const toast = useToast();
  const [config, setConfig] = useState<ServerConfiguration | null>(null);
  const [loading, setLoading] = useState(true);

  const load = useCallback(async () => {
    setLoading(true);
    try {
      const response = await api.get<ServerConfiguration>('/settings/config');
      setConfig(response.data);
    } catch (error) {
      const status = (error as { response?: { status?: number } })?.response?.status;
      if (status === 403) toast.show({ title: t('settings.messages.serverConfigLoadFailed'), tone: 'danger' });
      else if (status !== 404) toast.show({ title: t('settings.messages.serverConfigLoadFailedGeneric'), tone: 'danger' });
    } finally {
      setLoading(false);
    }
  }, [t, toast]);

  useEffect(() => {
    void load();
  }, [load]);

  if (loading && !config) return <Skeleton lines={4} height={48} label={t('common.status.loading', 'Loading...')} />;
  if (!config) return <Notice tone="danger">{t(`${P}.loadFailed`)}</Notice>;

  const c = config;
  return (
    <div className={shared.stack}>
      <SettingGroup
        id="server-upload"
        title={t(`${P}.fileUpload.title`)}
        summary={`${c.max_file_size_mb} MB · ${c.upload_path}`}
      >
        <Pass>
          <PassCell label={t(`${P}.fileUpload.maxFileSize`)} mono>{`${c.max_file_size_mb} MB`}</PassCell>
          <PassCell label={t(`${P}.fileUpload.uploadPath`)} mono>{c.upload_path}</PassCell>
          <PassCell label={t(`${P}.fileUpload.allowedFileTypes`)}>
            <span className={shared.row}>
              {c.allowed_file_types.map((type) => (
                <Tag key={type}>{type}</Tag>
              ))}
            </span>
          </PassCell>
          {c.watch_folder ? (
            <PassCell label={t(`${P}.fileUpload.watchFolder`)} mono>{c.watch_folder}</PassCell>
          ) : null}
        </Pass>
      </SettingGroup>

      <SettingGroup
        id="server-ocr"
        title={t(`${P}.ocrProcessing.title`)}
        summary={`${c.concurrent_ocr_jobs} · ${c.ocr_timeout_seconds}s · ${c.memory_limit_mb} MB · ${c.ocr_language}`}
      >
        <Pass>
          <PassCell label={t(`${P}.ocrProcessing.concurrentOcrJobs`)} mono>{c.concurrent_ocr_jobs}</PassCell>
          <PassCell label={t(`${P}.ocrProcessing.ocrTimeout`)} mono>{`${c.ocr_timeout_seconds}s`}</PassCell>
          <PassCell label={t(`${P}.ocrProcessing.memoryLimit`)} mono>{`${c.memory_limit_mb} MB`}</PassCell>
          <PassCell label={t(`${P}.ocrProcessing.ocrLanguage`)} mono>{c.ocr_language}</PassCell>
          <PassCell label={t(`${P}.ocrProcessing.cpuPriority`)}>{c.cpu_priority}</PassCell>
          <PassCell label={t(`${P}.ocrProcessing.backgroundOcr`)}>
            <Tag tone={c.enable_background_ocr ? 'ok' : 'default'}>
              {c.enable_background_ocr ? t(`${P}.ocrProcessing.enabled`) : t(`${P}.ocrProcessing.disabled`)}
            </Tag>
          </PassCell>
        </Pass>
      </SettingGroup>

      <SettingGroup
        id="server-info"
        title={t(`${P}.serverInformation.title`)}
        summary={`${c.server_host}:${c.server_port} · v${c.version}`}
      >
        <Pass>
          <PassCell label={t(`${P}.serverInformation.serverHost`)} mono>{c.server_host}</PassCell>
          <PassCell label={t(`${P}.serverInformation.serverPort`)} mono>{c.server_port}</PassCell>
          <PassCell label={t(`${P}.serverInformation.jwtSecret`)}>
            <Tag tone={c.jwt_secret_set ? 'ok' : 'danger'}>
              {c.jwt_secret_set ? t(`${P}.serverInformation.configured`) : t(`${P}.serverInformation.notSet`)}
            </Tag>
          </PassCell>
          <PassCell label={t(`${P}.serverInformation.version`)} mono>{c.version}</PassCell>
          {c.build_info ? (
            <PassCell label={t(`${P}.serverInformation.buildInformation`)} mono span={2}>{c.build_info}</PassCell>
          ) : null}
        </Pass>
      </SettingGroup>

      {c.watch_interval_seconds ? (
        <SettingGroup
          id="server-watch"
          title={t(`${P}.watchFolderConfiguration.title`)}
          summary={`${c.watch_interval_seconds}s`}
        >
          <Pass>
            <PassCell label={t(`${P}.watchFolderConfiguration.watchInterval`)} mono>{`${c.watch_interval_seconds}s`}</PassCell>
            {c.file_stability_check_ms ? (
              <PassCell label={t(`${P}.watchFolderConfiguration.fileStabilityCheck`)} mono>
                {`${c.file_stability_check_ms}ms`}
              </PassCell>
            ) : null}
            {c.max_file_age_hours ? (
              <PassCell label={t(`${P}.watchFolderConfiguration.maxFileAge`)} mono>{`${c.max_file_age_hours}h`}</PassCell>
            ) : null}
          </Pass>
        </SettingGroup>
      ) : null}

      <div>
        <Button icon={<CloudSync fontSize="inherit" />} onPress={() => void load()} isPending={loading}>
          {t(`${P}.refreshConfiguration`)}
        </Button>
      </div>
    </div>
  );
}
