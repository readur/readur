//! The per-backend entry points of a source sync: each validates its source's
//! configuration, builds the backend client and hands listing and download
//! to the shared pipeline.

use anyhow::{anyhow, Result};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::{
    models::{LocalFolderSourceConfig, S3SourceConfig, Source, WebDAVSourceConfig},
    services::local_folder_service::{authorize_local_folder_paths, LocalFolderService},
    services::s3_service::S3Service,
    services::webdav::{SyncProgress, WebDAVConfig, WebDAVService},
    utils::outbound::{validate_outbound_url, validate_outbound_url_for_config},
};

use super::{owner_is_active_admin, FolderDiscovery, PipelineRun, SourceSyncService, SyncOutcome};

impl SourceSyncService {
    pub(super) async fn sync_webdav_source(
        &self,
        source: &Source,
        enable_background_ocr: bool,
        cancellation_token: CancellationToken,
        progress: &Arc<SyncProgress>,
    ) -> Result<SyncOutcome> {
        let config: WebDAVSourceConfig = serde_json::from_value(source.config.clone())
            .map_err(|e| anyhow!("Invalid WebDAV config: {}", e))?;

        info!("WebDAV source sync config: server_url={}, username={}, watch_folders={:?}, file_extensions={:?}, server_type={:?}",
            config.server_url, config.username, config.watch_folders, config.file_extensions, config.server_type);

        // Re-check the destination at connect time; DNS may have changed
        // since the source was saved. A host that does not resolve right now
        // is left to the WebDAV client: its resolver applies the same address
        // checks, and its retry and error handling report the failure through
        // the sync progress like any other connection problem.
        validate_outbound_url_for_config(&WebDAVConfig::normalize_server_url(&config.server_url))
            .await
            .map_err(|e| anyhow!("{}", e))?;

        // Requests to list files in a Nextcloud folder might take > 2 minutes
        // Set timeout to 3 minutes to accommodate large folder structures
        let webdav_config = WebDAVConfig {
            server_url: config.server_url,
            username: config.username,
            password: config.password,
            watch_folders: config.watch_folders,
            file_extensions: config.file_extensions,
            timeout_seconds: 180, // 3 minutes for discover_files_in_folder operations
            server_type: config.server_type,
        };

        let webdav_service = WebDAVService::new(webdav_config)
            .map_err(|e| anyhow!("Failed to create WebDAV service: {}", e))?
            .with_cancellation_token(cancellation_token.clone());
        // The service holds the watch folders decoded and normalised.
        let webdav_config = webdav_service.get_config().clone();

        info!(
            "WebDAV service created successfully, starting sync with {} folders",
            webdav_config.watch_folders.len()
        );
        let discovery_token = cancellation_token.clone();

        self.run_pipeline(
            PipelineRun {
                user_id: source.user_id,
                source_id: source.id,
                watch_folders: &webdav_config.watch_folders,
                file_extensions: &webdav_config.file_extensions,
                enable_background_ocr,
                cancellation_token,
                progress,
            },
            |folder_path| {
                let service = webdav_service.clone();
                let state_clone = self.state.clone();
                let user_id = source.user_id;
                let progress = progress.clone();
                let token = discovery_token.clone();
                async move {
                    info!("Using smart sync for scheduled sync: {}", folder_path);
                    progress.set_current_directory(&folder_path);

                    let smart_sync_service = crate::services::webdav::SmartSyncService::new(state_clone)
                        .with_cancellation_token(token);

                    match smart_sync_service.evaluate_and_sync_detailed(user_id, Some(source.id), &service, &folder_path, Some(&progress)).await {
                        Ok(outcome) => {
                            match outcome.strategy {
                                Some(strategy) => info!("Smart sync completed for {}: {} files found using {:?}, {} directories failed",
                                                        folder_path, outcome.files.len(), strategy, outcome.failed_directories.len()),
                                None => info!("Smart sync: no changes detected for {}", folder_path),
                            }
                            Ok(FolderDiscovery {
                                files: outcome.files,
                                failed_directories: outcome.failed_directories,
                                directory_commit: Some(outcome.directory_commit),
                            })
                        },
                        Err(e) => {
                            error!("Smart sync failed for scheduled sync {}: {}", folder_path, e);
                            Err(e)
                        }
                    }
                }
            },
            |file_path| {
                let service = webdav_service.clone();
                let progress = progress.clone(); // Clone progress for the async closure
                async move {
                    debug!("WebDAV download_file called for: {}", file_path);
                    progress.set_current_file(Some(&file_path));
                    let result = service.download_file(&file_path).await;
                    match &result {
                        Ok(data) => debug!("WebDAV downloaded {} bytes for file: {}", data.len(), file_path),
                        Err(e) => error!("WebDAV download failed for file {}: {}", file_path, e),
                    }
                    result
                }
            }
        ).await
    }

    pub(super) async fn sync_local_folder_source(
        &self,
        source: &Source,
        enable_background_ocr: bool,
        cancellation_token: CancellationToken,
        progress: &Arc<SyncProgress>,
    ) -> Result<SyncOutcome> {
        let config: LocalFolderSourceConfig = serde_json::from_value(source.config.clone())
            .map_err(|e| anyhow!("Invalid LocalFolder config: {}", e))?;

        // Re-check against the current configuration: the allowlist or the
        // owner's role may have changed since the source was saved.
        let owner = self.state.db.get_user_by_id(source.user_id).await?;
        let owner_is_admin = owner_is_active_admin(owner.as_ref());
        authorize_local_folder_paths(
            &config.watch_folders,
            owner_is_admin,
            &self.state.config.security.local_source_allowed_paths,
        )
        .map_err(|e| anyhow!("Local folder source is not permitted: {}", e))?;

        let local_service = LocalFolderService::open(config.clone())
            .await
            .map_err(|e| anyhow!("Failed to create LocalFolder service: {}", e))?;

        self.run_pipeline(
            PipelineRun {
                user_id: source.user_id,
                source_id: source.id,
                watch_folders: &config.watch_folders,
                file_extensions: &config.file_extensions,
                enable_background_ocr,
                cancellation_token,
                progress,
            },
            |folder_path| {
                let service = local_service.clone();
                async move {
                    service
                        .discover_files_in_folder(&folder_path)
                        .await
                        .map(FolderDiscovery::from)
                }
            },
            |file_path| {
                let service = local_service.clone();
                async move { service.read_file(&file_path).await }
            },
        )
        .await
    }

    pub(super) async fn sync_s3_source(
        &self,
        source: &Source,
        enable_background_ocr: bool,
        cancellation_token: CancellationToken,
        progress: &Arc<SyncProgress>,
    ) -> Result<SyncOutcome> {
        let config: S3SourceConfig = serde_json::from_value(source.config.clone())
            .map_err(|e| anyhow!("Invalid S3 config: {}", e))?;

        for url in S3Service::outbound_urls(&config) {
            validate_outbound_url(&url)
                .await
                .map_err(|e| anyhow!("{}", e))?;
        }

        let s3_service = S3Service::new(config.clone())
            .await
            .map_err(|e| anyhow!("Failed to create S3 service: {}", e))?
            .with_cancellation_token(cancellation_token.clone());

        self.run_pipeline(
            PipelineRun {
                user_id: source.user_id,
                source_id: source.id,
                watch_folders: &config.watch_folders,
                file_extensions: &config.file_extensions,
                enable_background_ocr,
                cancellation_token,
                progress,
            },
            |folder_path| {
                let service = s3_service.clone();
                async move {
                    service
                        .discover_files_in_folder(&folder_path)
                        .await
                        .map(FolderDiscovery::from)
                }
            },
            |file_path| {
                let service = s3_service.clone();
                async move { service.download_file(&file_path).await }
            },
        )
        .await
    }
}
