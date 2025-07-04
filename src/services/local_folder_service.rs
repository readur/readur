use std::path::Path;
use std::fs;
use anyhow::{anyhow, Result};
use chrono::DateTime;
use tracing::{debug, info, warn};
use walkdir::WalkDir;
use sha2::{Sha256, Digest};
use serde_json;

use crate::models::{FileInfo, LocalFolderSourceConfig};

#[derive(Debug, Clone)]
pub struct LocalFolderService {
    config: LocalFolderSourceConfig,
}

impl LocalFolderService {
    pub fn new(config: LocalFolderSourceConfig) -> Result<Self> {
        // Validate that watch folders exist and are accessible
        for folder in &config.watch_folders {
            let path = Path::new(folder);
            if !path.exists() {
                return Err(anyhow!("Watch folder does not exist: {}", folder));
            }
            if !path.is_dir() {
                return Err(anyhow!("Watch folder is not a directory: {}", folder));
            }
        }

        Ok(Self { config })
    }

    /// Discover files in a specific folder
    pub async fn discover_files_in_folder(&self, folder_path: &str) -> Result<Vec<FileInfo>> {
        let path = Path::new(folder_path);
        if !path.exists() {
            return Err(anyhow!("Folder does not exist: {}", folder_path));
        }

        let mut files: Vec<FileInfo> = Vec::new();
        
        info!("Scanning local folder: {} (recursive: {})", folder_path, self.config.recursive);

        // Use tokio::task::spawn_blocking for file system operations
        let folder_path_clone = folder_path.to_string();
        let config = self.config.clone();
        
        let discovered_files = tokio::task::spawn_blocking(move || -> Result<Vec<FileInfo>> {
            let mut files: Vec<FileInfo> = Vec::new();
            
            let walker = if config.recursive {
                WalkDir::new(&folder_path_clone)
                    .follow_links(config.follow_symlinks)
                    .into_iter()
            } else {
                WalkDir::new(&folder_path_clone)
                    .max_depth(1)
                    .follow_links(config.follow_symlinks)
                    .into_iter()
            };

            for entry_result in walker {
                match entry_result {
                    Ok(entry) => {
                        let path = entry.path();
                        
                        // Skip directories and the root folder itself
                        if path.is_dir() {
                            continue;
                        }

                        // Check file extension
                        let extension = path.extension()
                            .and_then(|ext| ext.to_str())
                            .unwrap_or("")
                            .to_lowercase();

                        if !config.file_extensions.contains(&extension) {
                            debug!("Skipping file with unsupported extension: {}", path.display());
                            continue;
                        }

                        // Get file metadata
                        match fs::metadata(path) {
                            Ok(metadata) => {
                                let modified_time = metadata.modified()
                                    .ok()
                                    .and_then(|time| {
                                        let duration = time.duration_since(std::time::UNIX_EPOCH).ok()?;
                                        DateTime::from_timestamp(duration.as_secs() as i64, 0)
                                    });

                                // Try to get creation time (not available on all systems)
                                let created_time = metadata.created()
                                    .ok()
                                    .and_then(|time| {
                                        let duration = time.duration_since(std::time::UNIX_EPOCH).ok()?;
                                        DateTime::from_timestamp(duration.as_secs() as i64, 0)
                                    });

                                let file_name = path.file_name()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("unknown")
                                    .to_string();

                                // Generate a simple hash-based ETag from file path and modification time
                                let etag = Self::generate_etag(path, &metadata);

                                // Determine MIME type based on extension
                                let mime_type = Self::get_mime_type(&extension);

                                // Extract file permissions and ownership info
                                #[cfg(unix)]
                                let (permissions, owner, group) = {
                                    use std::os::unix::fs::MetadataExt;
                                    (
                                        Some(metadata.mode() & 0o777), // File mode bits (permissions)
                                        Some(metadata.uid().to_string()), // User ID
                                        Some(metadata.gid().to_string()), // Group ID
                                    )
                                };
                                
                                #[cfg(not(unix))]
                                let (permissions, owner, group) = (None, None, None);

                                // Prepare additional metadata
                                let mut additional_metadata = serde_json::Map::new();
                                
                                #[cfg(unix)]
                                {
                                    use std::os::unix::fs::MetadataExt;
                                    additional_metadata.insert("inode".to_string(), serde_json::Value::Number(metadata.ino().into()));
                                    additional_metadata.insert("nlinks".to_string(), serde_json::Value::Number(metadata.nlink().into()));
                                    additional_metadata.insert("device".to_string(), serde_json::Value::Number(metadata.dev().into()));
                                }
                                
                                // Add file attributes
                                additional_metadata.insert("readonly".to_string(), serde_json::Value::Bool(metadata.permissions().readonly()));
                                
                                let file_info = FileInfo {
                                    path: path.to_string_lossy().to_string(),
                                    name: file_name,
                                    size: metadata.len() as i64,
                                    mime_type,
                                    last_modified: modified_time,
                                    etag,
                                    is_directory: false,
                                    created_at: created_time,
                                    permissions,
                                    owner,
                                    group,
                                    metadata: if additional_metadata.is_empty() { None } else { Some(serde_json::Value::Object(additional_metadata)) },
                                };

                                files.push(file_info);
                            }
                            Err(e) => {
                                warn!("Failed to get metadata for {}: {}", path.display(), e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Error walking directory: {}", e);
                    }
                }
            }

            Ok(files)
        }).await??;

        info!("Found {} files in local folder {}", discovered_files.len(), folder_path);
        Ok(discovered_files)
    }

    /// Read file content for processing
    pub async fn read_file(&self, file_path: &str) -> Result<Vec<u8>> {
        let file_path = file_path.to_string();
        
        tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
            let content = fs::read(&file_path)
                .map_err(|e| anyhow!("Failed to read file {}: {}", file_path, e))?;
            Ok(content)
        }).await?
    }

    /// Test if the service can access the configured folders
    pub async fn test_connection(&self) -> Result<String> {
        let mut accessible_folders = 0;
        let mut total_files = 0;

        for folder in &self.config.watch_folders {
            match self.discover_files_in_folder(folder).await {
                Ok(files) => {
                    accessible_folders += 1;
                    total_files += files.len();
                    info!("Local folder {} is accessible with {} files", folder, files.len());
                }
                Err(e) => {
                    return Err(anyhow!("Cannot access folder {}: {}", folder, e));
                }
            }
        }

        Ok(format!(
            "Successfully accessed {} folders with {} total files",
            accessible_folders, total_files
        ))
    }

    /// Generate ETag for file based on path and modification time
    fn generate_etag(path: &Path, metadata: &fs::Metadata) -> String {
        let mut hasher = Sha256::new();
        hasher.update(path.to_string_lossy().as_bytes());
        
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                hasher.update(duration.as_secs().to_be_bytes());
            }
        }
        
        hasher.update(metadata.len().to_be_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)[..16].to_string() // Use first 16 chars as ETag
    }

    /// Get MIME type based on file extension
    fn get_mime_type(extension: &str) -> String {
        match extension {
            "pdf" => "application/pdf",
            "txt" => "text/plain",
            "png" => "image/png",
            "jpg" | "jpeg" => "image/jpeg",
            "tiff" | "tif" => "image/tiff",
            "bmp" => "image/bmp",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "xls" => "application/vnd.ms-excel",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "ppt" => "application/vnd.ms-powerpoint",
            "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
            _ => "application/octet-stream",
        }.to_string()
    }

    pub fn get_config(&self) -> &LocalFolderSourceConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs::File;
    use std::io::Write;

    #[tokio::test]
    async fn test_local_folder_discovery() {
        // Create a temporary directory with test files
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_str().unwrap();

        // Create test files
        let mut pdf_file = File::create(temp_dir.path().join("test.pdf")).unwrap();
        pdf_file.write_all(b"fake pdf content").unwrap();

        let mut txt_file = File::create(temp_dir.path().join("test.txt")).unwrap();
        txt_file.write_all(b"test content").unwrap();

        // Create unsupported file
        let mut bin_file = File::create(temp_dir.path().join("test.bin")).unwrap();
        bin_file.write_all(b"binary content").unwrap();

        // Create config
        let config = LocalFolderSourceConfig {
            watch_folders: vec![temp_path.to_string()],
            file_extensions: vec!["pdf".to_string(), "txt".to_string()],
            auto_sync: true,
            sync_interval_minutes: 60,
            recursive: false,
            follow_symlinks: false,
        };

        let service = LocalFolderService::new(config).unwrap();
        let files = service.discover_files_in_folder(temp_path).await.unwrap();

        // Should find 2 files (pdf and txt), but not bin
        assert_eq!(files.len(), 2);
        
        let pdf_file = files.iter().find(|f| f.name == "test.pdf").unwrap();
        assert_eq!(pdf_file.mime_type, "application/pdf");
        assert_eq!(pdf_file.size, 16);

        let txt_file = files.iter().find(|f| f.name == "test.txt").unwrap();
        assert_eq!(txt_file.mime_type, "text/plain");
        assert_eq!(txt_file.size, 12);
    }

    #[tokio::test]
    async fn test_file_reading() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        let test_content = b"Hello, World!";
        
        let mut file = File::create(&file_path).unwrap();
        file.write_all(test_content).unwrap();

        let config = LocalFolderSourceConfig {
            watch_folders: vec![temp_dir.path().to_str().unwrap().to_string()],
            file_extensions: vec!["txt".to_string()],
            auto_sync: false,
            sync_interval_minutes: 60,
            recursive: false,
            follow_symlinks: false,
        };

        let service = LocalFolderService::new(config).unwrap();
        let content = service.read_file(file_path.to_str().unwrap()).await.unwrap();
        
        assert_eq!(content, test_content);
    }
}