use crate::error::Result;
use scrape_core::now_iso;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use ts_rs::TS;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum NotificationKind {
    Update,
    News,
    System,
    Scraper,
    Error,
}

impl NotificationKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Update => "update",
            Self::News => "news",
            Self::System => "system",
            Self::Scraper => "scraper",
            Self::Error => "error",
        }
    }

    pub fn from_db_str(s: &str) -> Self {
        use std::str::FromStr;
        Self::from_str(s).unwrap_or(Self::System)
    }
}

impl std::str::FromStr for NotificationKind {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Ok(match s {
            "update" => Self::Update,
            "news" => Self::News,
            "system" => Self::System,
            "scraper" => Self::Scraper,
            "error" => Self::Error,
            _ => Self::System,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct AppNotification {
    pub id: String,
    pub kind: NotificationKind,
    pub title: String,
    pub message: String,
    pub action_label: Option<String>,
    pub action_payload: Option<String>,
    pub read: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct CreateNotificationInput {
    pub kind: NotificationKind,
    pub title: String,
    pub message: String,
    pub action_label: Option<String>,
    pub action_payload: Option<String>,
}

pub struct NotificationRepo;

impl NotificationRepo {
    pub async fn create(
        pool: &SqlitePool,
        input: CreateNotificationInput,
    ) -> Result<AppNotification> {
        let id = Uuid::new_v4().to_string();
        let now = now_iso();
        let kind_str = input.kind.as_str();

        sqlx::query(
            r#"
            INSERT INTO app_notifications (id, kind, title, message, action_label, action_payload, read, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, ?7)
            "#,
        )
        .bind(&id)
        .bind(kind_str)
        .bind(&input.title)
        .bind(&input.message)
        .bind(&input.action_label)
        .bind(&input.action_payload)
        .bind(&now)
        .execute(pool)
        .await?;

        Ok(AppNotification {
            id,
            kind: input.kind,
            title: input.title,
            message: input.message,
            action_label: input.action_label,
            action_payload: input.action_payload,
            read: false,
            created_at: now,
        })
    }

    pub async fn list(pool: &SqlitePool, limit: i64) -> Result<Vec<AppNotification>> {
        let rows = sqlx::query(
            r#"
            SELECT id, kind, title, message, action_label, action_payload, read, created_at
            FROM app_notifications
            ORDER BY created_at DESC
            LIMIT ?
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        let notifications = rows
            .into_iter()
            .map(|r| {
                let kind_str: String = r.get("kind");
                let read_int: i64 = r.get("read");
                AppNotification {
                    id: r.get("id"),
                    kind: NotificationKind::from_db_str(&kind_str),
                    title: r.get("title"),
                    message: r.get("message"),
                    action_label: r.get("action_label"),
                    action_payload: r.get("action_payload"),
                    read: read_int == 1,
                    created_at: r.get("created_at"),
                }
            })
            .collect();

        Ok(notifications)
    }

    pub async fn mark_as_read(pool: &SqlitePool, id: &str) -> Result<()> {
        sqlx::query("UPDATE app_notifications SET read = 1 WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn mark_all_as_read(pool: &SqlitePool) -> Result<()> {
        sqlx::query("UPDATE app_notifications SET read = 1")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn clear_all(pool: &SqlitePool) -> Result<()> {
        sqlx::query("DELETE FROM app_notifications")
            .execute(pool)
            .await?;
        Ok(())
    }

    pub async fn count_unread(pool: &SqlitePool) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM app_notifications WHERE read = 0")
            .fetch_one(pool)
            .await?;
        Ok(row.get("count"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory_db;

    #[tokio::test]
    async fn test_notification_lifecycle() {
        let pool = init_memory_db().await.unwrap();

        let n1 = NotificationRepo::create(
            &pool,
            CreateNotificationInput {
                kind: NotificationKind::Update,
                title: "UPDATE AVAILABLE v0.2.0".to_string(),
                message: "A new version of Scrape is ready.".to_string(),
                action_label: Some("INSTALL".to_string()),
                action_payload: Some("v0.2.0".to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(n1.read, false);
        assert_eq!(n1.kind, NotificationKind::Update);

        let unread = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(unread, 1);

        let list = NotificationRepo::list(&pool, 10).await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, n1.id);

        NotificationRepo::mark_as_read(&pool, &n1.id).await.unwrap();
        let unread_after = NotificationRepo::count_unread(&pool).await.unwrap();
        assert_eq!(unread_after, 0);

        NotificationRepo::clear_all(&pool).await.unwrap();
        let list_empty = NotificationRepo::list(&pool, 10).await.unwrap();
        assert_eq!(list_empty.len(), 0);

        // Test Error notification creation
        let n_err = NotificationRepo::create(
            &pool,
            CreateNotificationInput {
                kind: NotificationKind::Error,
                title: "WORKFLOW PARSE ERROR".to_string(),
                message: "Invalid YAML parameter on line 12".to_string(),
                action_label: Some("INSPECT".to_string()),
                action_payload: Some("source-1".to_string()),
            },
        )
        .await
        .unwrap();

        assert_eq!(n_err.kind, NotificationKind::Error);
        assert_eq!(n_err.kind.as_str(), "error");
        assert_eq!(
            NotificationKind::from_db_str("error"),
            NotificationKind::Error
        );
    }
}
