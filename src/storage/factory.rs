//! Factory for creating storage backends based on configuration

use anyhow::Result;
use std::sync::Arc;

use super::{StorageBackend, StorageConfig};
use super::local::LocalStorageBackend;

#[cfg(feature = "s3")]
use crate::services::s3_service::S3Service;

/// Create a storage backend based on the provided configuration
pub async fn create_storage_backend(config: StorageConfig) -> Result<Arc<dyn StorageBackend>> {
    match config {
        StorageConfig::Local { upload_path } => {
            let backend = LocalStorageBackend::new(upload_path);
            backend.initialize().await?;
            Ok(Arc::new(backend))
        }
        #[cfg(feature = "s3")]
        StorageConfig::S3 { s3_config, .. } => {
            let backend = S3Service::new(s3_config).await?;
            backend.initialize().await?;
            Ok(Arc::new(backend))
        }
    }
}

/// Create storage configuration from environment variables
pub fn storage_config_from_env(config: &crate::config::Config) -> Result<StorageConfig> {
    if config.s3_enabled {
        #[cfg(feature = "s3")]
        {
            if let Some(s3_config) = &config.s3_config {
                Ok(StorageConfig::S3 {
                    s3_config: s3_config.clone(),
                    fallback_path: Some(config.upload_path.clone()),
                })
            } else {
                // S3 enabled but no config, fall back to local
                Ok(StorageConfig::Local {
                    upload_path: config.upload_path.clone(),
                })
            }
        }
        #[cfg(not(feature = "s3"))]
        {
            // S3 requested but not compiled in
            tracing::warn!("S3 storage requested but S3 feature not compiled in, using local storage");
            Ok(StorageConfig::Local {
                upload_path: config.upload_path.clone(),
            })
        }
    } else {
        Ok(StorageConfig::Local {
            upload_path: config.upload_path.clone(),
        })
    }
}