# `src-tauri/migrations` — Versioned SQLite Schema Migrations

## Overview

The `src-tauri/migrations` directory contains incremental, versioned SQL migration scripts executed automatically at application startup by `sqlx::migrate!()`. This maintains schema integrity and manages local-first database evolution without data loss.

---

## Migration Manifest

| Migration File | Description & Schema Evolution |
|---|---|
| [`0001_init.sql`](0001_init.sql) | **Initial Core Schema**: Establishes primary application tables: `jobs` (titles, URLs, status, timestamps), `sources` (ATS configs, targets, active flags), `scrape_runs` (run logs, counts, durations), and `settings` (key-value user preferences). |
| [`0002_enrichment.sql`](0002_enrichment.sql) | **Data Enrichment Fields**: Adds structured columns to `jobs` for salary (`salary_min`, `salary_max`, `salary_currency`, `salary_interval`), normalized locations, visa sponsorship (`sponsorship`), security clearance (`clearance`), experience bounds (`min_years`, `max_years`), and `is_new_grad`. |
| [`0003_role_type_seasonal.sql`](0003_role_type_seasonal.sql) | **Role Type Expansion**: Introduces the `seasonal` role type classification for co-ops, summer internships, and seasonal contracts. |
| [`0004_scrape_jobs_cadence.sql`](0004_scrape_jobs_cadence.sql) | **Per-Job Scheduling Cadence**: Expands `sources` with individual cadence intervals (`hourly`, `six_hours`, `twelve_hours`, `daily`, `manual`), custom HTTP headers, and selector configs. |
| [`0005_workflow_retention_filters_favorites.sql`](0005_workflow_retention_filters_favorites.sql) | **Workflow & Retention Support**: Adds `is_favorite` boolean column to `jobs`, workflow rule storage, and data retention parameters. |
| [`0006_notifications.sql`](0006_notifications.sql) | **In-App Notifications**: Creates the `notifications` table storing run summaries, crawl failures, and high-match job discovery alerts. |
| [`0007_notification_error_kind.sql`](0007_notification_error_kind.sql) | **Error Categorization**: Adds error classification taxonomy to notification records to differentiate network timeouts, ATS rate limits, and CSS selector breakage. |
| [`0008_company_icon.sql`](0008_company_icon.sql) | **Company Favicon Support**: Adds `company_icon_url` to `jobs` to cache and display Google favicon proxy icons. |
| [`0009_work_auth_green_card.sql`](0009_work_auth_green_card.sql) | **Work Authorization**: Adds the `work_auth` column (`citizen_only`, `us_person`, `open`, `unknown`) to store U.S. citizenship and permanent residency requirements. |
| [`0010_deleted_jobs_tombstone.sql`](0010_deleted_jobs_tombstone.sql) | **Deletion Tombstone Table**: Creates the `deleted_jobs` table (`external_id`, `source_id`, `url`, `deleted_at`) ensuring that jobs deleted by the user in Discovered or Tracked views are never re-ingested on subsequent scrapes. |

---

## Migration Execution

Migrations are compiled directly into the Rust binary at build time and executed on database connection initialization in [`src-tauri/src/db/mod.rs`](../src/db/mod.rs):

```rust
sqlx::migrate!("./migrations")
    .run(&pool)
    .await?;
```
