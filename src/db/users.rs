use anyhow::Result;
use chrono::Utc;
use sqlx::{postgres::PgRow, Row};
use uuid::Uuid;

use crate::models::{AuthProvider, CreateUser, User, UserRole};
use super::Database;

/// bcrypt work factor used for all stored password hashes.
pub const BCRYPT_COST: u32 = 12;

/// Column list matching [`user_from_row`]. Keep the two in sync.
const USER_COLUMNS: &str = "id, username, email, password_hash, role, created_at, updated_at, \
     oidc_subject, oidc_issuer, oidc_email, auth_provider, token_version, is_active";

fn user_from_row(row: &PgRow) -> User {
    User {
        id: row.get("id"),
        username: row.get("username"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        role: row.get::<String, _>("role").try_into().unwrap_or(UserRole::User),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        oidc_subject: row.get("oidc_subject"),
        oidc_issuer: row.get("oidc_issuer"),
        oidc_email: row.get("oidc_email"),
        auth_provider: row
            .get::<String, _>("auth_provider")
            .try_into()
            .unwrap_or(AuthProvider::Local),
        token_version: row.get("token_version"),
        is_active: row.get("is_active"),
    }
}

impl Database {
    pub async fn create_user(&self, user: CreateUser) -> Result<User> {
        self.create_user_with_status(user, true).await
    }

    /// Create a local account. `is_active = false` creates it disabled
    /// (e.g. self-registration awaiting admin approval).
    pub async fn create_user_with_status(&self, user: CreateUser, is_active: bool) -> Result<User> {
        let password_hash = bcrypt::hash(&user.password, BCRYPT_COST)?;
        let now = Utc::now();

        let row = sqlx::query(&format!(
            "INSERT INTO users (username, email, password_hash, role, created_at, updated_at, auth_provider, is_active)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING {USER_COLUMNS}"
        ))
        .bind(&user.username)
        .bind(&user.email)
        .bind(&password_hash)
        .bind(user.role.as_ref().unwrap_or(&UserRole::User).to_string())
        .bind(now)
        .bind(now)
        .bind(AuthProvider::Local.to_string())
        .bind(is_active)
        .fetch_one(&self.pool)
        .await?;

        Ok(user_from_row(&row))
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(&format!("SELECT {USER_COLUMNS} FROM users WHERE username = $1"))
            .bind(username)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(user_from_row))
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1"))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(user_from_row))
    }

    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(&format!("SELECT {USER_COLUMNS} FROM users ORDER BY created_at DESC"))
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.iter().map(user_from_row).collect())
    }

    /// Update account fields. Changing the password or deactivating the
    /// account revokes all outstanding session tokens.
    pub async fn update_user(
        &self,
        id: Uuid,
        username: Option<String>,
        email: Option<String>,
        password: Option<String>,
        is_active: Option<bool>,
    ) -> Result<User> {
        let user = self.get_user_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("User not found"))?;

        let username = username.unwrap_or(user.username);
        let email = email.unwrap_or(user.email);
        let password_changed = password.is_some();
        let password_hash = match password {
            Some(pwd) => Some(bcrypt::hash(&pwd, BCRYPT_COST)?),
            None => user.password_hash,
        };
        let new_is_active = is_active.unwrap_or(user.is_active);
        let revoke_sessions = password_changed || (user.is_active && !new_is_active);

        let row = sqlx::query(&format!(
            "UPDATE users SET username = $1, email = $2, password_hash = $3, is_active = $4,
                 token_version = token_version + CASE WHEN $5 THEN 1 ELSE 0 END,
                 password_changed_at = CASE WHEN $6 THEN NOW() ELSE password_changed_at END,
                 updated_at = NOW()
             WHERE id = $7
             RETURNING {USER_COLUMNS}"
        ))
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(new_is_active)
        .bind(revoke_sessions)
        .bind(password_changed)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(user_from_row(&row))
    }

    /// Replace a user's password and revoke all outstanding session tokens.
    pub async fn set_user_password(&self, id: Uuid, new_password: &str) -> Result<User> {
        let password_hash = bcrypt::hash(new_password, BCRYPT_COST)?;
        let row = sqlx::query(&format!(
            "UPDATE users SET password_hash = $2, password_changed_at = NOW(),
                 token_version = token_version + 1, updated_at = NOW()
             WHERE id = $1
             RETURNING {USER_COLUMNS}"
        ))
        .bind(id)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(user_from_row(&row))
    }

    /// Invalidate every session token issued to the user so far.
    pub async fn revoke_user_sessions(&self, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE users SET token_version = token_version + 1, updated_at = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_user(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_user_by_oidc_subject(&self, subject: &str, issuer: &str) -> Result<Option<User>> {
        let row = sqlx::query(&format!(
            "SELECT {USER_COLUMNS} FROM users WHERE oidc_subject = $1 AND oidc_issuer = $2"
        ))
        .bind(subject)
        .bind(issuer)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.as_ref().map(user_from_row))
    }

    /// Rewrite the stored issuer of an account bound to an OIDC identity.
    pub async fn update_user_oidc_issuer(&self, user_id: Uuid, oidc_issuer: &str) -> Result<User> {
        let row = sqlx::query(&format!(
            "UPDATE users SET oidc_issuer = $2, updated_at = NOW()
             WHERE id = $1
             RETURNING {USER_COLUMNS}"
        ))
        .bind(user_id)
        .bind(oidc_issuer)
        .fetch_one(&self.pool)
        .await?;
        Ok(user_from_row(&row))
    }

    pub async fn create_oidc_user(
        &self,
        user: CreateUser,
        oidc_subject: &str,
        oidc_issuer: &str,
        oidc_email: &str,
    ) -> Result<User> {
        let now = Utc::now();

        let row = sqlx::query(&format!(
            "INSERT INTO users (username, email, role, created_at, updated_at,
                                oidc_subject, oidc_issuer, oidc_email, auth_provider)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING {USER_COLUMNS}"
        ))
        .bind(&user.username)
        .bind(&user.email)
        .bind(user.role.as_ref().unwrap_or(&UserRole::User).to_string())
        .bind(now)
        .bind(now)
        .bind(oidc_subject)
        .bind(oidc_issuer)
        .bind(oidc_email)
        .bind(AuthProvider::Oidc.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(user_from_row(&row))
    }

    pub async fn get_user_by_email(&self, email: &str) -> Result<Option<User>> {
        let row = sqlx::query(&format!("SELECT {USER_COLUMNS} FROM users WHERE email = $1"))
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(user_from_row))
    }

    pub async fn link_user_to_oidc(
        &self,
        user_id: Uuid,
        oidc_subject: &str,
        oidc_issuer: &str,
        oidc_email: &str,
    ) -> Result<User> {
        let row = sqlx::query(&format!(
            "UPDATE users
             SET oidc_subject = $2,
                 oidc_issuer = $3,
                 oidc_email = $4,
                 auth_provider = $5,
                 updated_at = NOW()
             WHERE id = $1
             RETURNING {USER_COLUMNS}"
        ))
        .bind(user_id)
        .bind(oidc_subject)
        .bind(oidc_issuer)
        .bind(oidc_email)
        .bind(AuthProvider::Oidc.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(user_from_row(&row))
    }

    /// Reset a user's password by username
    ///
    /// This function is useful for password recovery and admin password resets.
    /// The password is automatically hashed with bcrypt before storing, and all
    /// of the user's outstanding session tokens are revoked.
    ///
    /// # Arguments
    /// * `username` - The username of the user whose password should be reset
    /// * `new_password` - The new plaintext password (will be hashed)
    ///
    /// # Returns
    /// The updated User object, or an error if the user doesn't exist
    pub async fn reset_user_password(&self, username: &str, new_password: &str) -> Result<User> {
        let password_hash = bcrypt::hash(new_password, BCRYPT_COST)?;

        let row = sqlx::query(&format!(
            "UPDATE users
             SET password_hash = $2,
                 password_changed_at = NOW(),
                 token_version = token_version + 1,
                 updated_at = NOW()
             WHERE username = $1
             RETURNING {USER_COLUMNS}"
        ))
        .bind(username)
        .bind(&password_hash)
        .fetch_one(&self.pool)
        .await?;

        Ok(user_from_row(&row))
    }
}
