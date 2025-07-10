// Stub implementation when S3 feature is not enabled
use anyhow::{anyhow, Result};
use tracing::warn;

use crate::models::{FileIngestionInfo, S3SourceConfig};

#[derive(Debug, Clone)]
pub struct S3Service {
    config: S3SourceConfig,
}

impl S3Service {
    pub async fn new(_config: S3SourceConfig) -> Result<Self> {
        Err(anyhow!("S3 support not compiled in. Enable the 's3' feature to use S3 sources."))
    }

    pub async fn discover_files_in_folder(&self, _folder_path: &str) -> Result<Vec<FileIngestionInfo>> {
        warn!("S3 support not compiled in");
        Ok(Vec::new())
    }

    pub async fn download_file(&self, _object_key: &str) -> Result<Vec<u8>> {
        Err(anyhow!("S3 support not compiled in"))
    }

    pub async fn test_connection(&self) -> Result<String> {
        Err(anyhow!("S3 support not compiled in"))
    }

    pub async fn estimate_sync(&self) -> Result<(usize, i64)> {
        Ok((0, 0))
    }

    pub fn get_config(&self) -> &S3SourceConfig {
        &self.config
    }
}