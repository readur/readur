use anyhow::Result;
use chrono::Utc;
use sqlx::Row;
use uuid::Uuid;

use crate::models::{CreateUser, User};
use super::Database;

impl Database {
    pub async fn create_user(&self, user: CreateUser) -> Result<User> {
        let password_hash = bcrypt::hash(&user.password, 12)?;
        let now = Utc::now();

        let row = sqlx::query(
            r#"
            INSERT INTO users (username, email, password_hash, role, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING id, username, email, password_hash, role, created_at, updated_at
            "#
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&password_hash)
        .bind(user.role.as_ref().unwrap_or(&crate::models::UserRole::User).to_string())
        .bind(now)
        .bind(now)
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
        })
    }

    pub async fn get_user_by_username(&self, username: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE username = $1"
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
            })),
            None => Ok(None),
        }
    }

    pub async fn get_user_by_id(&self, id: Uuid) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users WHERE id = $1"
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
            })),
            None => Ok(None),
        }
    }

    pub async fn get_all_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, email, password_hash, role, created_at, updated_at FROM users ORDER BY created_at DESC"
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
            })
            .collect();

        Ok(users)
    }

    pub async fn update_user(&self, id: Uuid, username: Option<String>, email: Option<String>, password: Option<String>) -> Result<User> {
        let user = self.get_user_by_id(id).await?.ok_or_else(|| anyhow::anyhow!("User not found"))?;
        
        let username = username.unwrap_or(user.username);
        let email = email.unwrap_or(user.email);
        let password_hash = if let Some(pwd) = password {
            bcrypt::hash(&pwd, 12)?
        } else {
            user.password_hash
        };

        let row = sqlx::query(
            r#"
            UPDATE users SET username = $1, email = $2, password_hash = $3, updated_at = NOW()
            WHERE id = $4
            RETURNING id, username, email, password_hash, role, created_at, updated_at
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
        })
    }

    pub async fn delete_user(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}