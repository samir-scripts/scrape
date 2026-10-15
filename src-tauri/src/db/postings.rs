use crate::error::{AppError, Result};
use scrape_core::{now_iso, PostingIdentity, SourceKind};
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, SqlitePool, Transaction};
use ts_rs::TS;

/// Snapshot for pre-ingestion budget checks; persistence still rechecks atomically.
pub async fn suppression_snapshot(pool: &SqlitePool) -> Result<Vec<String>> {
    Ok(sqlx::query_scalar("SELECT identity FROM blocked_postings UNION SELECT identity FROM posting_legacy_blocks UNION SELECT legacy_key FROM posting_legacy_blocks UNION SELECT dedupe_key FROM deleted_jobs")
        .fetch_all(pool).await?)
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RemovedPosting {
    pub identity: String,
    pub title: String,
    pub company: String,
    pub url: Option<String>,
    pub source_name: Option<String>,
    pub removed_at: Option<String>,
    pub blocked: bool,
    pub blocked_at: Option<String>,
    #[ts(type = "number")]
    pub copies: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct RemovalResult {
    pub identity: String,
    #[ts(type = "number")]
    pub copies: i64,
}

pub async fn identity_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    source_id: Option<&str>,
    external_id: Option<&str>,
    url: &str,
) -> Result<String> {
    let source: Option<(String, String)> =
        sqlx::query_as("SELECT kind, identifier FROM sources WHERE id = ?")
            .bind(source_id)
            .fetch_optional(&mut **tx)
            .await?;
    let kind = source
        .as_ref()
        .map(|(kind, _)| {
            serde_json::from_value::<SourceKind>(serde_json::Value::String(kind.clone()))
        })
        .transpose()?;
    PostingIdentity::for_posting(
        kind,
        source.as_ref().map(|(_, board)| board.as_str()),
        external_id,
        url,
    )
    .map(|id| id.key)
    .map_err(AppError::Validation)
}

/// Upgrade legacy rows once, without losing unresolved suppression keys.
pub async fn backfill(pool: &SqlitePool) -> Result<()> {
    let mut tx = pool.begin().await?;
    type LegacyJob = (String, String, Option<String>, Option<String>, String);
    let jobs: Vec<LegacyJob> = sqlx::query_as(
        "SELECT id, dedupe_key, source_id, external_id, url FROM jobs WHERE posting_identity IS NULL")
        .fetch_all(&mut *tx).await?;
    for (id, dedupe, source, external, url) in jobs {
        let identity =
            identity_in_transaction(&mut tx, source.as_deref(), external.as_deref(), &url)
                .await
                .or_else(|error| match error {
                    AppError::Validation(_) => Ok(format!("legacy:{dedupe}")),
                    other => Err(other),
                })?;
        sqlx::query("UPDATE jobs SET posting_identity = ?, source_name_snapshot = COALESCE(source_name_snapshot, (SELECT name FROM sources WHERE id = jobs.source_id)) WHERE id = ?")
            .bind(identity)
            .bind(id)
            .execute(&mut *tx)
            .await?;
    }
    let tombstones: Vec<(String, String)> = sqlx::query_as(
        "SELECT dedupe_key, deleted_at FROM deleted_jobs WHERE NOT EXISTS (SELECT 1 FROM posting_legacy_blocks WHERE legacy_key = deleted_jobs.dedupe_key)")
        .fetch_all(&mut *tx).await?;
    for (key, date) in tombstones {
        let mut identity = format!("legacy:{key}");
        if let Some((source_id, external)) = key.split_once(':') {
            let source: Option<(String, String)> =
                sqlx::query_as("SELECT kind, identifier FROM sources WHERE id = ?")
                    .bind(source_id)
                    .fetch_optional(&mut *tx)
                    .await?;
            if let Some((kind, board)) = source {
                let kind: SourceKind = serde_json::from_value(serde_json::Value::String(kind))?;
                if !kind.expects_url() {
                    identity = PostingIdentity::for_posting(
                        Some(kind),
                        Some(&board),
                        Some(external),
                        "https://legacy.invalid/",
                    )
                    .map_err(AppError::Validation)?
                    .key;
                }
            }
        }
        sqlx::query(
            "INSERT OR IGNORE INTO posting_legacy_blocks (legacy_key, identity) VALUES (?, ?)",
        )
        .bind(&key)
        .bind(&identity)
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT OR IGNORE INTO blocked_postings (identity, title, company, blocked_at, legacy_key) VALUES (?, 'Legacy posting', '', ?, ?)")
            .bind(identity).bind(date).bind(key).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn remove(pool: &SqlitePool, id: &str, block: bool) -> Result<RemovalResult> {
    backfill(pool).await?;
    let mut tx = pool.begin().await?;
    // Reserve the SQLite writer before the block check / changes.
    sqlx::query("UPDATE jobs SET removed_at = removed_at WHERE id = ?")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    let row: (String, String, String, String, Option<String>) = sqlx::query_as(
        "SELECT posting_identity, title, company, url, COALESCE(source_name_snapshot, (SELECT name FROM sources WHERE id = jobs.source_id)) FROM jobs WHERE id = ?")
        .bind(id).fetch_optional(&mut *tx).await?.ok_or_else(|| AppError::NotFound(format!("Job {id}")))?;
    let now = now_iso();
    let copies = sqlx::query(
        "UPDATE jobs SET removed_at = ? WHERE posting_identity = ? AND removed_at IS NULL",
    )
    .bind(&now)
    .bind(&row.0)
    .execute(&mut *tx)
    .await?
    .rows_affected() as i64;
    if block {
        sqlx::query("INSERT INTO blocked_postings (identity, title, company, url, source_name, blocked_at) VALUES (?, ?, ?, ?, ?, ?) ON CONFLICT(identity) DO UPDATE SET title=excluded.title, company=excluded.company, url=excluded.url, source_name=excluded.source_name")
            .bind(&row.0).bind(row.1).bind(row.2).bind(row.3).bind(row.4).bind(now).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(RemovalResult {
        identity: row.0,
        copies,
    })
}

pub async fn restore(pool: &SqlitePool, identity: &str) -> Result<()> {
    let mut tx = pool.begin().await?;
    sqlx::query("UPDATE jobs SET removed_at = NULL WHERE posting_identity = ?")
        .bind(identity)
        .execute(&mut *tx)
        .await?;
    clear_block(&mut tx, identity).await?;
    tx.commit().await?;
    Ok(())
}
async fn clear_block(tx: &mut Transaction<'_, Sqlite>, identity: &str) -> Result<()> {
    sqlx::query("DELETE FROM deleted_jobs WHERE dedupe_key IN (SELECT legacy_key FROM posting_legacy_blocks WHERE identity = ?)")
        .bind(identity).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM posting_legacy_blocks WHERE identity = ?")
        .bind(identity)
        .execute(&mut **tx)
        .await?;
    sqlx::query("DELETE FROM blocked_postings WHERE identity = ?")
        .bind(identity)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub async fn set_blocked(pool: &SqlitePool, identity: &str, blocked: bool) -> Result<()> {
    let mut tx = pool.begin().await?;
    if blocked {
        sqlx::query("INSERT OR IGNORE INTO blocked_postings (identity, title, company, url, source_name, blocked_at) SELECT posting_identity, title, company, url, COALESCE(source_name_snapshot, (SELECT name FROM sources WHERE id = jobs.source_id)), ? FROM jobs WHERE posting_identity = ? LIMIT 1")
            .bind(now_iso()).bind(identity).execute(&mut *tx).await?;
    } else {
        clear_block(&mut tx, identity).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn list(
    pool: &SqlitePool,
    search: &str,
    view: &str,
    offset: i64,
    limit: i64,
) -> Result<super::jobs::Page<RemovedPosting>> {
    if !matches!(view, "all" | "removed" | "blocked") {
        return Err(AppError::Validation("Invalid removal view".into()));
    }
    backfill(pool).await?;
    let query = r#"WITH entries AS (
        SELECT posting_identity AS identity, MIN(title) AS title, MIN(company) AS company, MIN(url) AS url,
            GROUP_CONCAT(DISTINCT COALESCE(source_name_snapshot, (SELECT name FROM sources WHERE id = jobs.source_id))) AS source_name,
            MAX(removed_at) AS removed_at, COUNT(*) AS copies FROM jobs WHERE removed_at IS NOT NULL GROUP BY posting_identity
        UNION ALL
        SELECT identity, title, company, url, source_name, NULL, 0 FROM blocked_postings b
            WHERE NOT EXISTS (SELECT 1 FROM jobs WHERE removed_at IS NOT NULL AND posting_identity = b.identity)
    ) SELECT e.*, b.blocked_at FROM entries e LEFT JOIN blocked_postings b ON b.identity=e.identity
        WHERE (? = 'all' OR (? = 'removed' AND e.removed_at IS NOT NULL) OR (? = 'blocked' AND b.identity IS NOT NULL))
        AND (e.title LIKE ? OR e.company LIKE ? OR e.source_name LIKE ?)"#;
    let pattern = format!("%{search}%");
    let count_sql = format!("SELECT count(*) FROM ({query})");
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(view)
        .bind(view)
        .bind(view)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(pool)
        .await?;
    let page_sql = format!(
        "{query} ORDER BY COALESCE(removed_at, blocked_at) DESC, e.identity LIMIT ? OFFSET ?"
    );
    let limit = limit.clamp(1, 100);
    let offset = offset.max(0);
    let rows = sqlx::query(&page_sql)
        .bind(view)
        .bind(view)
        .bind(view)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    use sqlx::Row;
    let items = rows
        .into_iter()
        .map(|r| {
            let blocked_at: Option<String> = r.get("blocked_at");
            RemovedPosting {
                identity: r.get("identity"),
                title: r.get("title"),
                company: r.get("company"),
                url: r.get("url"),
                source_name: r.get("source_name"),
                removed_at: r.get("removed_at"),
                copies: r.get("copies"),
                blocked: blocked_at.is_some(),
                blocked_at,
            }
        })
        .collect();
    Ok(super::jobs::Page {
        items,
        total,
        limit,
        offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::jobs::{JobFilter, JobsRepo};
    #[tokio::test]
    async fn removed_copies_preserve_saved_state_and_blocks_survive_source_recreation() {
        let pool = crate::db::init_memory_db().await.unwrap();
        for id in ["workflow-a", "workflow-b"] {
            sqlx::query("INSERT INTO sources (id, kind, name, identifier, created_at) VALUES (?, 'greenhouse', 'Acme', ?, '2026-01-01')")
                .bind(id).bind(id).execute(&pool).await.unwrap();
        }
        // Same ATS account across two internal workflows (legacy databases may
        // contain copies; the source uniqueness constraint prevents new duplicates).
        let identity = PostingIdentity::for_posting(
            Some(SourceKind::Greenhouse),
            Some("acme"),
            Some("42"),
            "https://example.com/42",
        )
        .unwrap()
        .key;
        for id in ["saved-a", "saved-b"] {
            sqlx::query("INSERT INTO jobs (id, dedupe_key, posting_identity, title, company, url, date_added, last_seen_at, status, status_changed_at, notes, favorited) VALUES (?1, ?1, ?2, 'Engineer', 'Acme', 'https://example.com/42', '2000-01-01', '2000-01-01', 'applied', '2000-01-01', 'Referral', 1)")
                .bind(id).bind(&identity).execute(&pool).await.unwrap();
            sqlx::query("INSERT INTO job_status_events (job_id, to_status, changed_at) VALUES (?, 'applied', '2000-01-01')")
                .bind(id).execute(&pool).await.unwrap();
        }
        assert_eq!(remove(&pool, "saved-a", true).await.unwrap().copies, 2);
        assert_eq!(
            JobsRepo::list(&pool, JobFilter::default())
                .await
                .unwrap()
                .total,
            0
        );
        assert_eq!(
            list(&pool, "Engineer", "blocked", 0, 10)
                .await
                .unwrap()
                .total,
            1
        );
        set_blocked(&pool, &identity, false).await.unwrap();
        assert_eq!(
            JobsRepo::list(&pool, JobFilter::default())
                .await
                .unwrap()
                .total,
            0,
            "Unblock does not restore"
        );
        restore(&pool, &identity).await.unwrap();
        for id in ["saved-a", "saved-b"] {
            let job = JobsRepo::get(&pool, id).await.unwrap();
            assert_eq!(job.notes.as_deref(), Some("Referral"));
            assert!(job.favorited);
            assert_eq!(job.status, scrape_core::JobStatus::Applied);
            assert_eq!(
                JobsRepo::get_status_events(&pool, id).await.unwrap().len(),
                1
            );
        }
    }
    #[tokio::test]
    async fn legacy_keys_are_preserved_and_unblock_does_not_recreate_them() {
        let pool = crate::db::init_memory_db().await.unwrap();
        sqlx::query("INSERT INTO deleted_jobs (dedupe_key, deleted_at) VALUES ('unresolved-old-key', '2020-01-01')").execute(&pool).await.unwrap();
        backfill(&pool).await.unwrap();
        let entries = list(&pool, "Legacy", "blocked", 0, 10).await.unwrap();
        assert_eq!(entries.total, 1);
        assert!(entries.items[0].url.is_none());
        set_blocked(&pool, &entries.items[0].identity, false)
            .await
            .unwrap();
        backfill(&pool).await.unwrap();
        assert_eq!(list(&pool, "", "all", 0, 10).await.unwrap().total, 0);
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;
    #[tokio::test]
    async fn blocking_survives_database_reopen_and_retention() {
        let directory =
            std::env::temp_dir().join(format!("scrape-blocks-{}", uuid::Uuid::new_v4()));
        let path = directory.join("jobs.db");
        let pool = crate::db::init_db(&path).await.unwrap();
        sqlx::query("INSERT INTO jobs (id, dedupe_key, title, company, url, date_added, last_seen_at, status_changed_at) VALUES ('saved', 'key', 'Engineer', 'Acme', 'https://example.com/jobs/42', '2000-01-01', '2000-01-01', '2000-01-01')").execute(&pool).await.unwrap();
        let removal = remove(&pool, "saved", true).await.unwrap();
        pool.close().await;
        let reopened = crate::db::init_db(&path).await.unwrap();
        let entries = list(&reopened, "", "blocked", 0, 25).await.unwrap();
        assert_eq!(entries.total, 1);
        assert_eq!(entries.items[0].identity, removal.identity);
        let removed: Option<String> =
            sqlx::query_scalar("SELECT removed_at FROM jobs WHERE id='saved'")
                .fetch_one(&reopened)
                .await
                .unwrap();
        assert!(removed.is_some());
        reopened.close().await;
        tokio::fs::remove_dir_all(directory).await.unwrap();
    }
}
