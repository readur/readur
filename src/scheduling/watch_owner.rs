//! Which account owns the files ingested from the global watch folder.

use anyhow::Result;
use tracing::error;
use uuid::Uuid;

use crate::db::Database;
use crate::models::UserRole;

/// Owner of files from the global watch folder, and of files left in a
/// per-user watch directory whose user no longer exists: the configured
/// admin account ([`crate::seed::admin_username`]), the same account the
/// seed creates. It is looked up by that exact name and must hold the admin
/// role, so a renamed admin never leaves the files to whichever account
/// happens to be called "admin".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchFolderOwner {
    username: String,
}

impl WatchFolderOwner {
    /// The admin named by `ADMIN_USERNAME` (default `admin`).
    pub fn from_env() -> Self {
        Self::named(crate::seed::admin_username())
    }

    /// A specific account, by username.
    pub fn named(username: impl Into<String>) -> Self {
        Self {
            username: username.into(),
        }
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    /// The owner's user id. `None`, logged as an error, when the account
    /// does not exist or is not an admin: the caller skips the file instead
    /// of guessing another owner. Database errors are returned.
    pub async fn resolve(&self, db: &Database) -> Result<Option<Uuid>> {
        match db.get_user_by_username(&self.username).await? {
            Some(user) if user.role == UserRole::Admin => Ok(Some(user.id)),
            Some(_) => {
                error!(
                    "Watch folder owner '{}' (ADMIN_USERNAME) is not an admin; skipping watch folder files until the account is an admin or ADMIN_USERNAME names one",
                    self.username
                );
                Ok(None)
            }
            None => {
                error!(
                    "Watch folder owner '{}' (ADMIN_USERNAME) does not exist; skipping watch folder files until the account is created or ADMIN_USERNAME names an existing admin",
                    self.username
                );
                Ok(None)
            }
        }
    }
}
