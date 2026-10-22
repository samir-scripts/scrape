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
| [`0008_company_icon.sql`](0008_company_icon.sql) | **Company Favicon Support**: Adds `company_icon_url` to `jobs` for provider-supplied imagery; missing images use neutral monograms. |
| [`0009_work_auth_green_card.sql`](0009_work_auth_green_card.sql) | **Work Authorization**: Adds the `work_auth` column (`citizen_only`, `us_person`, `open`, `unknown`) to store U.S. citizenship and permanent residency requirements. |
| [`0010_deleted_jobs_tombstone.sql`](0010_deleted_jobs_tombstone.sql) | **Deletion Tombstone Table**: Creates the `deleted_jobs` table for historical workflow-dependent keys ensuring that jobs deleted by the user in Discovered or Tracked views are never re-ingested on subsequent scrapes. |

---

## Migration execution and recovery

Startup first opens existing databases read-only and validates their migration
history and actual schema against an independently built historical prefix.
Unknown states fail before writable pragmas or migration metadata changes.
Recognized databases receive a consistent SQLite `VACUUM INTO` backup before
pending upgrades. Applied migration SQL and checksums are immutable.

Additional forward migrations:

| Migration | Behavior |
|---|---|
| 0011 | Safe retention policy: protect tracking, notes and favorites. |
| 0012 | Shared posting identities, soft removal and legacy block metadata. |
| 0013 | Native compensation evidence, separate from annual analytics. |
| 0014 | Matching metadata used by shared worker/repository predicates. |
| 0015 | Provider edit time separate from publication time. |
| 0016 | Run-stage counts and effective execution specification. |
| 0017 | Recognized pre-0011 Greenhouse update timestamps preserved as edits, with false posting dates cleared. |
| 0018 | Finalize posting-removal provenance; exact historical variant gets missing schema through a separate immutable manifest. |
| 0019 | Preserve original description evidence separately from canonical text and sanitized structured description. |
| 0020 | Forward description-format checkpoint shared with the recognized historical variant. |
| 0021 | Explicit workflow configuration review state and transactional compatibility conversion. |
| 0022 | Additive run/source outcome columns; preserve coarse historical statuses and every existing row. |
| 0023 | Employer branding registry and saved-posting relationship references; one local image, separate last-attempt diagnostics, no source-delete cascade. |
| 0024 | Parent workflow ownership and multi-source allocation; creates workflows table, backfills workflow_id across sources, jobs, and scrape_runs, preserves user state, schedules, and historical checksums. |
| 0025 | Forward employer-branding `asset_data` checkpoint shared with the recognized six-column historical variant of migration 23. |
| 0026 | Isolated browser-session metadata and additive per-source script trust/hash columns; no source-kind CHECK changes or table rebuilds. Stores session references and paths, never cookies or credentials. |
| 0027 | Scope source endpoint uniqueness to workflow ownership, preserving job/run links through table replacement. |
| 0028 | Admit Workday sources while preserving existing source columns, constraints, indexes and foreign-key links. |
| 0029 | Admit cross-employer `search` sources; populated v28 upgrade tests cover complete historical projections, consistent backup, injected rollback, retry and idempotent restart. |
| 0030 | Creates indexed `role_titles` and token-prefix lookup tables; the embedded O*NET TSV is seeded after migrations only when the title table is empty. |

The Rust backfills preserve source identities, uncertain legacy salary values and
unresolved suppression keys. Populated upgrade tests preserve all tracked states,
notes, favorites, status events, workflow rules, custom settings and tombstones.
See [database recovery](../../plans/database-recovery.md); a Git revert does not
reverse database schema changes. Never update `_sqlx_migrations` to bypass errors.

`../historical-migrations/` contains verified immutable migration-12/18, migration-19/20, and migration-23/25 variants. The runner accepts only matching histories and independently rebuilt schemas, then preserves their stored checksums. See incidents 25, 26, and 27 in the recovery documentation.

Migration 27 scopes `(kind, identifier)` uniqueness to `workflow_id`, allowing two
workflows to reference the same board. It recreates `sources` with all existing
columns, constraints and indexes, preserving source IDs and rows. Foreign keys
remain enabled; temporary maps restore `jobs.source_id` and `scrape_runs.source_id`
after SQLite applies their existing `ON DELETE SET NULL` actions during the table
replacement. Populated v26 upgrade tests compare every historical table, verify
foreign-key integrity, backup contents, failure after dropping the old table,
rollback/retry and restart. Applied migrations 1–26 remain byte-for-byte unchanged.
