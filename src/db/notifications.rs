use anyhow::Result;
use sqlx::Row;
use uuid::Uuid;

use super::Database;

impl Database {
    pub async fn create_notification(&self, user_id: Uuid, notification: &crate::models::CreateNotification) -> Result<crate::models::Notification> {
        self.with_retry(|| async {
            let row = sqlx::query(
                r#"INSERT INTO notifications (user_id, notification_type, title, message, action_url, metadata)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   RETURNING id, user_id, notification_type, title, message, read, action_url, metadata, created_at"#
            )
            .bind(user_id)
            .bind(&notification.notification_type)
            .bind(&notification.title)
            .bind(&notification.message)
            .bind(&notification.action_url)
            .bind(&notification.metadata)
            .fetch_one(&self.pool)
            .await
            .map_err(|e| anyhow::anyhow!("Database insert failed: {}", e))?;

        Ok(crate::models::Notification {
            id: row.get("id"),
            user_id: row.get("user_id"),
            notification_type: row.get("notification_type"),
            title: row.get("title"),
            message: row.get("message"),
            read: row.get("read"),
            action_url: row.get("action_url"),
            metadata: row.get("metadata"),
            created_at: row.get("created_at"),
        })
        }).await
    }

    pub async fn get_user_notifications(&self, user_id: Uuid, limit: i64, offset: i64) -> Result<Vec<crate::models::Notification>> {
        let rows = sqlx::query(
            r#"SELECT id, user_id, notification_type, title, message, read, action_url, metadata, created_at
               FROM notifications 
               WHERE user_id = $1 
               ORDER BY created_at DESC 
               LIMIT $2 OFFSET $3"#
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        let mut notifications = Vec::new();
        for row in rows {
            notifications.push(crate::models::Notification {
                id: row.get("id"),
                user_id: row.get("user_id"),
                notification_type: row.get("notification_type"),
                title: row.get("title"),
                message: row.get("message"),
                read: row.get("read"),
                action_url: row.get("action_url"),
                metadata: row.get("metadata"),
                created_at: row.get("created_at"),
            });
        }

        Ok(notifications)
    }

    pub async fn get_unread_notification_count(&self, user_id: Uuid) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM notifications WHERE user_id = $1 AND read = false")
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;

        Ok(row.get("count"))
    }

    pub async fn mark_notification_read(&self, user_id: Uuid, notification_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE notifications SET read = true WHERE id = $1 AND user_id = $2")
            .bind(notification_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn mark_all_notifications_read(&self, user_id: Uuid) -> Result<()> {
        sqlx::query("UPDATE notifications SET read = true WHERE user_id = $1 AND read = false")
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn delete_notification(&self, user_id: Uuid, notification_id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM notifications WHERE id = $1 AND user_id = $2")
            .bind(notification_id)
            .bind(user_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_notification_summary(&self, user_id: Uuid) -> Result<crate::models::NotificationSummary> {
        let unread_count = self.get_unread_notification_count(user_id).await?;
        let recent_notifications = self.get_user_notifications(user_id, 5, 0).await?;

        Ok(crate::models::NotificationSummary {
            unread_count,
            recent_notifications,
        })
    }
}