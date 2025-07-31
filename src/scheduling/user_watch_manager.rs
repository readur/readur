use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::{
    db::Database,
    models::User,
    services::user_watch_service::UserWatchService,
};

/// Manager that coordinates between the file watcher and user management
/// 
/// This manager handles:
/// - Mapping file paths to users based on directory structure
/// - Discovering existing users and setting up their watch directories
/// - Handling user lifecycle events (creation/deletion)
/// - Providing efficient user lookup by file path
/// - Caching user information for performance
#[derive(Clone)]
pub struct UserWatchManager {
    /// Database for user operations
    db: Database,
    /// Service for managing user watch directories
    user_watch_service: UserWatchService,
    /// Cache of username to user mappings for fast lookup
    /// Uses RwLock for concurrent read access with exclusive write access
    user_cache: Arc<RwLock<HashMap<String, User>>>,
    /// Cache of user directory paths to user IDs for reverse lookup
    path_to_user_cache: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl UserWatchManager {
    /// Create a new UserWatchManager
    /// 
    /// # Arguments
    /// * `db` - Database instance for user operations
    /// * `user_watch_service` - Service for managing user watch directories
    /// 
    /// # Returns
    /// * New UserWatchManager instance
    pub fn new(db: Database, user_watch_service: UserWatchService) -> Self {
        Self {
            db,
            user_watch_service,
            user_cache: Arc::new(RwLock::new(HashMap::new())),
            path_to_user_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize the manager by discovering users and setting up their directories
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn initialize(&self) -> Result<()> {
        info!("Initializing UserWatchManager");
        
        // Initialize the user watch service first
        self.user_watch_service.initialize().await?;
        
        // Discover and cache all users
        self.discover_and_cache_users().await?;
        
        info!("UserWatchManager initialized successfully");
        Ok(())
    }

    /// Discover all users from database and cache them
    async fn discover_and_cache_users(&self) -> Result<()> {
        info!("Discovering and caching users");
        
        // Get all users from database
        let users = self.db.get_all_users().await
            .map_err(|e| anyhow::anyhow!("Failed to get users from database: {}", e))?;
        
        let mut user_cache = self.user_cache.write().await;
        let mut path_cache = self.path_to_user_cache.write().await;
        
        for user in users {
            debug!("Caching user: {} ({})", user.username, user.id);
            
            // Ensure user directory exists
            if let Err(e) = self.user_watch_service.ensure_user_directory(&user).await {
                warn!("Failed to ensure directory for user {}: {}", user.username, e);
                continue;
            }
            
            // Get user directory path for reverse lookup cache
            let user_dir = self.user_watch_service.get_user_directory_by_username(&user.username);
            let dir_key = user_dir.to_string_lossy().to_string();
            
            // Update caches
            user_cache.insert(user.username.clone(), user.clone());
            path_cache.insert(dir_key, user.id);
        }
        
        info!("Cached {} users and their watch directories", user_cache.len());
        Ok(())
    }

    /// Get user by username, checking cache first, then database
    /// 
    /// # Arguments
    /// * `username` - Username to look up
    /// 
    /// # Returns
    /// * Option<User> if found
    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        // Check cache first (read lock)
        {
            let cache = self.user_cache.read().await;
            if let Some(user) = cache.get(username) {
                debug!("Found user {} in cache", username);
                return Ok(Some(user.clone()));
            }
        }

        debug!("User {} not in cache, checking database", username);
        
        // Not in cache, check database (release lock before DB operation)
        let user = self.db.get_user_by_username(username).await?;
        
        if let Some(ref user) = user {
            // Prepare directory before acquiring locks
            let ensure_dir_result = self.user_watch_service.ensure_user_directory(user).await;
            let user_dir = self.user_watch_service.get_user_directory_by_username(username);
            let dir_key = user_dir.to_string_lossy().to_string();
            
            // Update caches with short-lived locks
            {
                let mut cache = self.user_cache.write().await;
                cache.insert(username.to_string(), user.clone());
            }
            
            if ensure_dir_result.is_ok() {
                let mut path_cache = self.path_to_user_cache.write().await;
                path_cache.insert(dir_key, user.id);
            } else {
                warn!("Failed to ensure directory for user {}: {:?}", username, ensure_dir_result);
            }
            
            info!("Cached new user from database: {}", username);
        }
        
        Ok(user)
    }

    /// Get user by file path within user watch directories
    /// 
    /// # Arguments
    /// * `file_path` - Path to a file within a user watch directory
    /// 
    /// # Returns
    /// * Option<User> if the file belongs to a user's watch directory
    pub async fn get_user_by_file_path(&self, file_path: &Path) -> Result<Option<User>> {
        // Extract username from path
        let username = match self.user_watch_service.extract_username_from_path(file_path) {
            Some(username) => username,
            None => {
                debug!("Could not extract username from path: {}", file_path.display());
                return Ok(None);
            }
        };

        debug!("Extracted username '{}' from path: {}", username, file_path.display());
        
        // Look up user by username
        self.get_user_by_username(&username).await
    }

    /// Check if a file path is within user watch directories
    /// 
    /// # Arguments
    /// * `file_path` - Path to check
    /// 
    /// # Returns
    /// * bool indicating whether the path is within user watch directories
    pub fn is_user_watch_path(&self, file_path: &Path) -> bool {
        self.user_watch_service.is_within_user_watch(file_path)
    }

    /// Handle user creation by setting up their watch directory
    /// 
    /// # Arguments
    /// * `user` - Newly created user
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn handle_user_created(&self, user: &User) -> Result<()> {
        info!("Setting up watch directory for new user: {}", user.username);
        
        // Ensure user directory exists
        self.user_watch_service.ensure_user_directory(user).await?;
        
        // Update caches
        let mut user_cache = self.user_cache.write().await;
        let mut path_cache = self.path_to_user_cache.write().await;
        
        let user_dir = self.user_watch_service.get_user_directory_by_username(&user.username);
        let dir_key = user_dir.to_string_lossy().to_string();
        
        user_cache.insert(user.username.clone(), user.clone());
        path_cache.insert(dir_key, user.id);
        
        info!("Successfully set up watch directory for user: {}", user.username);
        Ok(())
    }

    /// Handle user deletion by cleaning up their watch directory
    /// 
    /// # Arguments
    /// * `user` - User being deleted
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn handle_user_deleted(&self, user: &User) -> Result<()> {
        info!("Cleaning up watch directory for deleted user: {}", user.username);
        
        // Remove user directory
        self.user_watch_service.remove_user_directory(user).await?;
        
        // Remove from caches
        let mut user_cache = self.user_cache.write().await;
        let mut path_cache = self.path_to_user_cache.write().await;
        
        user_cache.remove(&user.username);
        
        // Remove from path cache (need to find the entry by user ID)
        let user_dir = self.user_watch_service.get_user_directory_by_username(&user.username);
        let dir_key = user_dir.to_string_lossy().to_string();
        path_cache.remove(&dir_key);
        
        info!("Successfully cleaned up watch directory for user: {}", user.username);
        Ok(())
    }

    /// Handle username change by moving watch directory and updating caches
    /// 
    /// # Arguments
    /// * `old_username` - Previous username
    /// * `updated_user` - User with updated information
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn handle_username_changed(&self, old_username: &str, updated_user: &User) -> Result<()> {
        info!("Handling username change from '{}' to '{}'", old_username, updated_user.username);
        
        let old_dir = self.user_watch_service.get_user_directory_by_username(old_username);
        let new_dir = self.user_watch_service.get_user_directory_by_username(&updated_user.username);
        
        // Move directory if it exists
        if old_dir.exists() {
            info!("Moving user watch directory from '{}' to '{}'", old_dir.display(), new_dir.display());
            tokio::fs::rename(&old_dir, &new_dir).await
                .map_err(|e| anyhow::anyhow!(
                    "Failed to move user watch directory from '{}' to '{}': {}", 
                    old_dir.display(), new_dir.display(), e
                ))?;
        } else {
            // If old directory doesn't exist, create new one
            self.user_watch_service.ensure_user_directory(updated_user).await?;
        }
        
        // Update caches
        let mut user_cache = self.user_cache.write().await;
        let mut path_cache = self.path_to_user_cache.write().await;
        
        // Remove old entries
        user_cache.remove(old_username);
        let old_dir_key = old_dir.to_string_lossy().to_string();
        path_cache.remove(&old_dir_key);
        
        // Add new entries
        user_cache.insert(updated_user.username.clone(), updated_user.clone());
        let new_dir_key = new_dir.to_string_lossy().to_string();
        path_cache.insert(new_dir_key, updated_user.id);
        
        info!("Successfully handled username change to '{}'", updated_user.username);
        Ok(())
    }

    /// Get all users that have watch directories set up
    /// 
    /// # Returns
    /// * Vec<User> of users with watch directories
    pub async fn get_all_watch_users(&self) -> Vec<User> {
        let cache = self.user_cache.read().await;
        cache.values().cloned().collect()
    }

    /// Get statistics about the user watch manager
    /// 
    /// # Returns
    /// * (cached_users, service_stats) tuple
    pub async fn get_statistics(&self) -> Result<(usize, (usize, usize))> {
        let cached_users = {
            let cache = self.user_cache.read().await;
            cache.len()
        };

        let service_stats = self.user_watch_service.get_statistics().await?;
        
        Ok((cached_users, service_stats))
    }

    /// Clear all caches (useful for testing or cache invalidation)
    pub async fn clear_caches(&self) {
        let mut user_cache = self.user_cache.write().await;
        let mut path_cache = self.path_to_user_cache.write().await;
        
        user_cache.clear();
        path_cache.clear();
        
        self.user_watch_service.clear_cache().await;
        
        debug!("All UserWatchManager caches cleared");
    }

    /// Refresh user cache by reloading from database
    /// 
    /// # Returns
    /// * Result indicating success or failure
    pub async fn refresh_user_cache(&self) -> Result<()> {
        info!("Refreshing user cache from database");
        
        // Clear existing cache
        self.clear_caches().await;
        
        // Reload from database
        self.discover_and_cache_users().await?;
        
        info!("User cache refreshed successfully");
        Ok(())
    }

    /// Get the underlying UserWatchService (for direct access if needed)
    pub fn get_user_watch_service(&self) -> &UserWatchService {
        &self.user_watch_service
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

    // Note: These tests would need a mock database implementation
    // For now, they serve as documentation of the intended API
    
    #[tokio::test]
    async fn test_user_watch_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let user_watch_service = UserWatchService::new(temp_dir.path());
        
        // Would need mock database here
        // let db = create_mock_database();
        // let manager = UserWatchManager::new(db, user_watch_service);
        // assert!(manager.initialize().await.is_ok());
    }

    #[tokio::test]
    async fn test_extract_username_from_path() {
        let temp_dir = TempDir::new().unwrap();
        let user_watch_service = UserWatchService::new(temp_dir.path());
        user_watch_service.initialize().await.unwrap();

        let user = create_test_user("testuser");
        let user_dir = user_watch_service.ensure_user_directory(&user).await.unwrap();
        let test_file = user_dir.join("document.pdf");

        let username = user_watch_service.extract_username_from_path(&test_file);
        assert_eq!(username, Some("testuser".to_string()));
    }

    #[tokio::test]
    async fn test_is_user_watch_path() {
        let temp_dir = TempDir::new().unwrap();
        let user_watch_service = UserWatchService::new(temp_dir.path());
        user_watch_service.initialize().await.unwrap();

        let user = create_test_user("testuser");
        let user_dir = user_watch_service.ensure_user_directory(&user).await.unwrap();
        let test_file = user_dir.join("document.pdf");

        assert!(user_watch_service.is_within_user_watch(&test_file));
        
        let outside_file = temp_dir.path().parent().unwrap().join("outside.pdf");
        assert!(!user_watch_service.is_within_user_watch(&outside_file));
    }
}