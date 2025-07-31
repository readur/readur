use anyhow::Result;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::models::User;

/// Service for managing per-user watch directories
/// 
/// This service handles:
/// - Creating user-specific watch directories
/// - Managing directory permissions and ownership
/// - Handling cleanup on user deletion
/// - Providing thread-safe access to user directory paths
/// - Graceful error handling for filesystem operations
#[derive(Clone)]
pub struct UserWatchService {
    /// Base directory where user watch folders are created
    base_dir: PathBuf,
    /// Cache of user ID to watch directory path mappings
    /// Uses Arc<RwLock> for concurrent read access with exclusive write access
    user_directories: Arc<RwLock<HashMap<Uuid, PathBuf>>>,
}

impl UserWatchService {
    /// Create a new UserWatchService
    /// 
    /// # Arguments
    /// * `base_dir` - Base directory path where user watch directories will be created
    /// 
    /// # Returns
    /// * New UserWatchService instance
    pub fn new<P: AsRef<Path>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.as_ref().to_path_buf(),
            user_directories: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Validate username for security (prevent path traversal attacks)
    /// 
    /// # Arguments
    /// * `username` - Username to validate
    /// 
    /// # Returns
    /// * Result indicating if username is valid
    fn validate_username(username: &str) -> Result<()> {
        if username.is_empty() || username.len() > 64 {
            return Err(anyhow::anyhow!("Username must be between 1 and 64 characters"));
        }
        
        // Check for path traversal attempts and invalid characters
        if username.contains("..") || 
           username.starts_with('.') || 
           username.contains('/') || 
           username.contains('\\') ||
           username.contains('\0') {
            return Err(anyhow::anyhow!("Username contains invalid characters"));
        }
        
        // Only allow alphanumeric characters, underscore, and dash
        if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(anyhow::anyhow!("Username can only contain alphanumeric characters, underscore, and dash"));
        }
        
        // Additional security checks
        if username == "." || username == ".." {
            return Err(anyhow::anyhow!("Username cannot be '.' or '..'"));
        }
        
        Ok(())
    }

    /// Initialize the service by creating the base directory and discovering existing user directories
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing UserWatchService with base directory: {}", self.base_dir.display());
        
        // Create base directory if it doesn't exist
        if !self.base_dir.exists() {
            info!("Creating user watch base directory: {}", self.base_dir.display());
            tokio::fs::create_dir_all(&self.base_dir).await
                .map_err(|e| anyhow::anyhow!(
                    "Failed to create user watch base directory '{}': {}", 
                    self.base_dir.display(), e
                ))?;
        } else if !self.base_dir.is_dir() {
            return Err(anyhow::anyhow!(
                "User watch base path '{}' exists but is not a directory", 
                self.base_dir.display()
            ));
        }

        // Discover existing user directories
        self.discover_existing_directories().await?;
        
        info!("UserWatchService initialized successfully");
        Ok(())
    }

    /// Discover existing user directories in the base directory
    /// This is used during initialization to populate the cache with existing directories
    async fn discover_existing_directories(&self) -> Result<()> {
        debug!("Discovering existing user watch directories");
        
        let mut entries = tokio::fs::read_dir(&self.base_dir).await
            .map_err(|e| anyhow::anyhow!(
                "Failed to read user watch base directory '{}': {}", 
                self.base_dir.display(), e
            ))?;

        let mut discovered_count = 0;
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            anyhow::anyhow!("Error reading directory entry: {}", e)
        })? {
            let path = entry.path();
            if path.is_dir() {
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    debug!("Found existing user watch directory: {}", dir_name);
                    // Note: We don't store these in the cache yet since we don't have user IDs
                    // The cache will be populated when users are looked up during operation
                    discovered_count += 1;
                }
            }
        }

        info!("Discovered {} existing user watch directories", discovered_count);
        Ok(())
    }

    /// Create or ensure a user's watch directory exists
    /// 
    /// # Arguments
    /// * `user` - User for whom to create the watch directory
    /// 
    /// # Returns
    /// * PathBuf to the user's watch directory
    pub async fn ensure_user_directory(&self, user: &User) -> Result<PathBuf> {
        // Validate username for security
        Self::validate_username(&user.username)?;
        // Check cache first (read lock)
        {
            let cache = self.user_directories.read().await;
            if let Some(path) = cache.get(&user.id) {
                if path.exists() {
                    debug!("User watch directory found in cache: {}", path.display());
                    return Ok(path.clone());
                } else {
                    warn!("Cached user watch directory no longer exists: {}", path.display());
                }
            }
        }

        // Not in cache or doesn't exist, create it (write lock)
        let mut cache = self.user_directories.write().await;
        
        // Double-check in case another thread created it while we were waiting for the write lock
        if let Some(path) = cache.get(&user.id) {
            if path.exists() {
                debug!("User watch directory created by another thread: {}", path.display());
                return Ok(path.clone());
            }
        }

        let user_dir = self.base_dir.join(&user.username);
        
        // Use atomic directory creation to avoid race conditions
        match tokio::fs::create_dir_all(&user_dir).await {
            Ok(_) => {
                info!("Created user watch directory for {}: {}", user.username, user_dir.display());
                
                // Set appropriate permissions (readable/writable by owner, readable by group)
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let permissions = std::fs::Permissions::from_mode(0o755);
                    if let Err(e) = std::fs::set_permissions(&user_dir, permissions) {
                        warn!("Failed to set permissions on user watch directory '{}': {}", 
                              user_dir.display(), e);
                        // Don't fail the operation for permission issues
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Directory already exists, check if it's actually a directory
                if !user_dir.is_dir() {
                    return Err(anyhow::anyhow!(
                        "User watch path '{}' exists but is not a directory", 
                        user_dir.display()
                    ));
                }
                debug!("User watch directory already exists for {}: {}", user.username, user_dir.display());
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "Failed to create user watch directory for '{}' at '{}': {}", 
                    user.username, user_dir.display(), e
                ));
            }
        }

        // Update cache
        cache.insert(user.id, user_dir.clone());
        
        Ok(user_dir)
    }

    /// Get the watch directory path for a user (from cache if available)
    /// 
    /// # Arguments
    /// * `user_id` - ID of the user
    /// 
    /// # Returns
    /// * Option<PathBuf> to the user's watch directory if it exists
    pub async fn get_user_directory(&self, user_id: Uuid) -> Option<PathBuf> {
        let cache = self.user_directories.read().await;
        cache.get(&user_id).filter(|path| path.exists()).cloned()
    }

    /// Get the watch directory path for a user by username
    /// This method constructs the path based on the username without checking the cache
    /// 
    /// # Arguments
    /// * `username` - Username of the user
    /// 
    /// # Returns
    /// * PathBuf to where the user's watch directory should be
    pub fn get_user_directory_by_username(&self, username: &str) -> PathBuf {
        self.base_dir.join(username)
    }

    /// Extract username from a file path within the user watch directory structure
    /// 
    /// # Arguments
    /// * `file_path` - Path to a file within a user watch directory
    /// 
    /// # Returns
    /// * Option<String> containing the username if the path is within a user directory
    pub fn extract_username_from_path(&self, file_path: &Path) -> Option<String> {
        // Normalize the file path - use canonical path for security
        let file_canonical = match file_path.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                debug!("Failed to canonicalize file path: {}", file_path.display());
                return None;
            }
        };
        
        let base_canonical = match self.base_dir.canonicalize() {
            Ok(path) => path,
            Err(_) => {
                debug!("Failed to canonicalize base directory: {}", self.base_dir.display());
                return None;
            }
        };

        // Check if the file is within the user watch base directory
        if !file_canonical.starts_with(&base_canonical) {
            debug!("File path {} is not within user watch base directory {}", 
                   file_canonical.display(), base_canonical.display());
            return None;
        }

        // Extract the relative path from base directory
        let relative_path = file_canonical.strip_prefix(&base_canonical).ok()?;
        let components: Vec<_> = relative_path.components().collect();
        
        if components.is_empty() {
            debug!("No path components found after stripping base directory");
            return None;
        }

        // First component should be the username
        let username = components[0].as_os_str().to_str()?;
        
        // Validate the extracted username for security
        if let Err(e) = Self::validate_username(username) {
            warn!("Invalid username '{}' extracted from path {}: {}", 
                  username, file_path.display(), e);
            return None;
        }
        
        debug!("Extracted username '{}' from path {}", username, file_path.display());
        Some(username.to_string())
    }

    /// Remove a user's watch directory and clean up cache
    /// 
    /// # Arguments
    /// * `user` - User whose watch directory should be removed
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn remove_user_directory(&self, user: &User) -> Result<()> {
        info!("Removing user watch directory for {}", user.username);
        
        let user_dir = self.base_dir.join(&user.username);
        
        if user_dir.exists() {
            // Remove directory and all contents
            tokio::fs::remove_dir_all(&user_dir).await
                .map_err(|e| anyhow::anyhow!(
                    "Failed to remove user watch directory for '{}' at '{}': {}", 
                    user.username, user_dir.display(), e
                ))?;
            
            info!("Successfully removed user watch directory for {}", user.username);
        } else {
            debug!("User watch directory for {} did not exist", user.username);
        }

        // Remove from cache
        let mut cache = self.user_directories.write().await;
        cache.remove(&user.id);
        
        Ok(())
    }

    /// Check if a path is within the user watch directory structure
    /// 
    /// # Arguments
    /// * `path` - Path to check
    /// 
    /// # Returns
    /// * bool indicating whether the path is within user watch directories
    pub fn is_within_user_watch(&self, path: &Path) -> bool {
        let file_canonical = path.canonicalize().ok().unwrap_or_else(|| path.to_path_buf());
        let base_canonical = self.base_dir.canonicalize().ok().unwrap_or_else(|| self.base_dir.clone());
        
        file_canonical.starts_with(&base_canonical)
    }

    /// Get statistics about user watch directories
    /// 
    /// # Returns
    /// * (cached_directories, total_directories) tuple
    pub async fn get_statistics(&self) -> Result<(usize, usize)> {
        let cached_count = {
            let cache = self.user_directories.read().await;
            cache.len()
        };

        let mut total_count = 0;
        if self.base_dir.exists() {
            let mut entries = tokio::fs::read_dir(&self.base_dir).await
                .map_err(|e| anyhow::anyhow!(
                    "Failed to read user watch base directory: {}", e
                ))?;

            while let Some(entry) = entries.next_entry().await
                .map_err(|e| anyhow::anyhow!("Error reading directory entry: {}", e))? {
                if entry.path().is_dir() {
                    total_count += 1;
                }
            }
        }

        Ok((cached_count, total_count))
    }

    /// Clear the directory cache (useful for testing or cache invalidation)
    pub async fn clear_cache(&self) {
        let mut cache = self.user_directories.write().await;
        cache.clear();
        debug!("User watch directory cache cleared");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use uuid::Uuid;
    use crate::models::{UserRole, AuthProvider};
    use chrono::Utc;

    fn create_test_user(username: &str) -> User {
        User {
            id: Uuid::new_v4(),
            username: username.to_string(),
            email: format!("{}@example.com", username),
            password_hash: Some("test_hash".to_string()),
            role: UserRole::User,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            oidc_subject: None,
            oidc_issuer: None,
            oidc_email: None,
            auth_provider: AuthProvider::Local,
        }
    }

    #[tokio::test]
    async fn test_user_watch_service_initialization() {
        let temp_dir = TempDir::new().unwrap();
        let service = UserWatchService::new(temp_dir.path());
        
        assert!(service.initialize().await.is_ok());
        assert!(temp_dir.path().exists());
    }

    #[tokio::test]
    async fn test_ensure_user_directory() {
        let temp_dir = TempDir::new().unwrap();
        let service = UserWatchService::new(temp_dir.path());
        service.initialize().await.unwrap();

        let user = create_test_user("testuser");
        let user_dir = service.ensure_user_directory(&user).await.unwrap();
        
        assert!(user_dir.exists());
        assert!(user_dir.is_dir());
        assert_eq!(user_dir.file_name().unwrap(), "testuser");
    }

    #[tokio::test]
    async fn test_extract_username_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let service = UserWatchService::new(temp_dir.path());
        service.initialize().await.unwrap();

        // Create user directory
        let user = create_test_user("testuser");
        let user_dir = service.ensure_user_directory(&user).await.unwrap();
        
        // Create a test file
        let test_file = user_dir.join("test.pdf");
        tokio::fs::write(&test_file, b"test content").await.unwrap();

        let username = service.extract_username_from_path(&test_file);
        assert_eq!(username, Some("testuser".to_string()));
    }

    #[tokio::test]
    async fn test_remove_user_directory() {
        let temp_dir = TempDir::new().unwrap();
        let service = UserWatchService::new(temp_dir.path());
        service.initialize().await.unwrap();

        let user = create_test_user("testuser");
        let user_dir = service.ensure_user_directory(&user).await.unwrap();
        assert!(user_dir.exists());

        service.remove_user_directory(&user).await.unwrap();
        assert!(!user_dir.exists());
    }

    #[tokio::test]
    async fn test_is_within_user_watch() {
        let temp_dir = TempDir::new().unwrap();
        let service = UserWatchService::new(temp_dir.path());
        service.initialize().await.unwrap();

        let user = create_test_user("testuser");
        let user_dir = service.ensure_user_directory(&user).await.unwrap();
        let test_file = user_dir.join("test.pdf");

        assert!(service.is_within_user_watch(&test_file));
        
        let outside_file = temp_dir.path().parent().unwrap().join("outside.pdf");
        assert!(!service.is_within_user_watch(&outside_file));
    }
}