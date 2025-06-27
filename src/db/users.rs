use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::models::{CreateUser, User, AuthProvider};
use super::Database;

impl Database {
    pub async fn create_user(&self, user: CreateUser) -> Result<User> {
        let password_hash = bcrypt::hash(&user.password, 12)?;
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO users (username, email, password_hash, role, created_at, updated_at, auth_provider)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id, username, email, password_hash, role, created_at, updated_at, 
                      oidc_subject, oidc_issuer, oidc_email, auth_provider
            "#
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&password_hash)
        .bind(user.role.as_ref().unwrap_or(&crate::models::UserRole::User).to_string())
        .bind(now)
        .bind(now)
        .bind(AuthProvider::Local.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            oidc_subject: row.get("oidc_subject"),
            oidc_issuer: row.get("oidc_issuer"),
            oidc_email: row.get("oidc_email"),
            auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Local),
        })
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at, 
             oidc_subject, oidc_issuer, oidc_email, auth_provider FROM users WHERE username = $1"
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                oidc_subject: row.get("oidc_subject"),
                oidc_issuer: row.get("oidc_issuer"),
                oidc_email: row.get("oidc_email"),
                auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Local),
            })),
            None => Ok(None),
        }
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at,
             oidc_subject, oidc_issuer, oidc_email, auth_provider FROM users WHERE id = $1"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                oidc_subject: row.get("oidc_subject"),
                oidc_issuer: row.get("oidc_issuer"),
                oidc_email: row.get("oidc_email"),
                auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Local),
            })),
            None => Ok(None),
        }
    }

    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at,
             oidc_subject, oidc_issuer, oidc_email, auth_provider FROM users ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        let users = rows
            .into_iter()
            .map(|row| User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                oidc_subject: row.get("oidc_subject"),
                oidc_issuer: row.get("oidc_issuer"),
                oidc_email: row.get("oidc_email"),
                auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Local),
            })
            .collect();

        Ok(users)
    }

    pub async fn update_user(&self, id: Uuid, username: Option<String>, email: Option<String>, password: Option<String>) -> Result<User> {
        let user = self.get_user_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("User not found"))?;
        
        let username = username.unwrap_or(user.username);
        let email = email.unwrap_or(user.email);
        let password_hash = if let Some(pwd) = password {
            Some(bcrypt::hash(&pwd, 12)?)
        } else {
            user.password_hash
        };

        let row = sqlx::query(
            r#"
            UPDATE users SET username = $1, email = $2, password_hash = $3, updated_at = NOW()
            WHERE id = $4
            RETURNING id, username, email, password_hash, role, created_at, updated_at,
                      oidc_subject, oidc_issuer, oidc_email, auth_provider
            "#
        )
        .bind(&username)
        .bind(&email)
        .bind(&password_hash)
        .bind(id)
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            oidc_subject: row.get("oidc_subject"),
            oidc_issuer: row.get("oidc_issuer"),
            oidc_email: row.get("oidc_email"),
            auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Local),
        })
    }

    pub async fn delete_user(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_user_by_oidc_subject(&self, subject: &str, issuer: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at,
             oidc_subject, oidc_issuer, oidc_email, auth_provider 
             FROM users WHERE oidc_subject = $1 AND oidc_issuer = $2"
        )
        .bind(subject)
        .bind(issuer)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(User {
                id: row.get("id"),
                username: row.get("username"),
                email: row.get("email"),
                password_hash: row.get("password_hash"),
                role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
                created_at: row.get("created_at"),
                updated_at: row.get("updated_at"),
                oidc_subject: row.get("oidc_subject"),
                oidc_issuer: row.get("oidc_issuer"),
                oidc_email: row.get("oidc_email"),
                auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Local),
            })),
            None => Ok(None),
        }
    }

    pub async fn create_oidc_user(
        &self,
        user: CreateUser,
        oidc_subject: &str,
        oidc_issuer: &str,
        oidc_email: &str,
    ) -> Result<User> {
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO users (username, email, role, created_at, updated_at, 
                             oidc_subject, oidc_issuer, oidc_email, auth_provider)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING id, username, email, password_hash, role, created_at, updated_at,
                      oidc_subject, oidc_issuer, oidc_email, auth_provider
            "#
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(user.role.as_ref().unwrap_or(&crate::models::UserRole::User).to_string())
        .bind(now)
        .bind(now)
        .bind(oidc_subject)
        .bind(oidc_issuer)
        .bind(oidc_email)
        .bind(AuthProvider::Oidc.to_string())
        .fetch_one(&self.pool)
        .await?;

        Ok(User {
            id: row.get("id"),
            username: row.get("username"),
            email: row.get("email"),
            password_hash: row.get("password_hash"),
            role: row.get::<String, _>("role").try_into().unwrap_or(crate::models::UserRole::User),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            oidc_subject: row.get("oidc_subject"),
            oidc_issuer: row.get("oidc_issuer"),
            oidc_email: row.get("oidc_email"),
            auth_provider: row.get::<String, _>("auth_provider").try_into().unwrap_or(AuthProvider::Oidc),
        })
    }
}