# `src/bindings` — TypeScript Type Bindings (ts-rs)

## Overview

The `src/bindings` directory contains TypeScript type definitions and interfaces generated automatically from Rust structs and enums in `crates/scrape-core` and `src-tauri` using the `ts-rs` crate. This guarantees strict compile-time type safety between the Rust backend and the React frontend.

> **Note**: Do not edit files in this directory manually. They are regenerated automatically by running `cargo test` in `crates/scrape-core`.

---

## File Manifest

| Binding File | Underlying Rust Type | Description |
|---|---|---|
| [`AppNotification.ts`](AppNotification.ts) | `AppNotification` | In-app notification entity with id, kind, title, message, read state, timestamp. |
| [`ClassifiedClearance.ts`](ClassifiedClearance.ts) | `ClassifiedClearance` | Clearance classification result with `ClearanceLevel` and confidence. |
| [`ClassifiedSponsorship.ts`](ClassifiedSponsorship.ts) | `ClassifiedSponsorship` | Visa sponsorship result with `TriState` flag and matched snippet. |
| [`ClassifiedWorkAuth.ts`](ClassifiedWorkAuth.ts) | `ClassifiedWorkAuth` | Work authorization requirement (`citizen_only`, `us_person`, `open`, `unknown`). |
| [`ClearanceLevel.ts`](ClearanceLevel.ts) | `ClearanceLevel` | Clearance requirement enum (`none`, `secret`, `top_secret`, etc.). |
| [`CreateNotificationInput.ts`](CreateNotificationInput.ts) | `CreateNotificationInput` | Payload for creating an in-app notification. |
| [`CreateSourceInput.ts`](CreateSourceInput.ts) | `CreateSourceInput` | Input payload for adding a new job board scraping source. |
| [`CronValidationResult.ts`](CronValidationResult.ts) | `CronValidationResult` | Validation response for cron syntax check. |
| [`ExperienceClassification.ts`](ExperienceClassification.ts) | `ExperienceClassification` | Parsed min/max years of experience and `is_new_grad` flag. |
| [`FilterImpactPreview.ts`](FilterImpactPreview.ts) | `FilterImpactPreview` | Estimated job match count when testing filter adjustments. |
| [`Job.ts`](Job.ts) | `Job` | Primary job application entity stored in SQLite and displayed in the UI. |
| [`JobFilter.ts`](JobFilter.ts) | `JobFilter` | Filter criteria structure passed to backend search queries. |
| [`JobSort.ts`](JobSort.ts) | `JobSort` | Sorting column and direction configuration. |
| [`JobStatus.ts`](JobStatus.ts) | `JobStatus` | Application pipeline state (`discovered`, `applied`, `interviewing`, `offer`, `rejected`). |
| [`JobStatusEvent.ts`](JobStatusEvent.ts) | `JobStatusEvent` | Audit log record of a job stage change. |
| [`LocationOptions.ts`](LocationOptions.ts) | `LocationOptions` | Available location suggestions and filter counts. |
| [`LogLevel.ts`](LogLevel.ts) | `LogLevel` | Diagnostic log level (`debug`, `info`, `warn`, `error`). |
| [`Metrics.ts`](Metrics.ts) | `Metrics` | Complete analytics dashboard payload. |
| [`MetricsSeriesBucket.ts`](MetricsSeriesBucket.ts) | `MetricsSeriesBucket` | Time-series data point for velocity charts. |
| [`MetricsStreak.ts`](MetricsStreak.ts) | `MetricsStreak` | User application streak metrics (current streak, longest streak). |
| [`MetricsTotals.ts`](MetricsTotals.ts) | `MetricsTotals` | Aggregate counts of applications across all stages. |
| [`NotificationKind.ts`](NotificationKind.ts) | `NotificationKind` | Notification category (`crawl_completed`, `crawl_failed`, `job_match`). |
| [`Page.ts`](Page.ts) | `Page<T>` | Generic paginated response wrapper. |
| [`ParsedLocation.ts`](ParsedLocation.ts) | `ParsedLocation` | Normalized location with city, state, country, and remote flag. |
| [`RoleType.ts`](RoleType.ts) | `RoleType` | High-level role enum (`swe`, `data`, `product`, `design`, `seasonal`, `other`). |
| [`RoleTypeMetric.ts`](RoleTypeMetric.ts) | `RoleTypeMetric` | Distribution metric for a single role category. |
| [`RunStatus.ts`](RunStatus.ts) | `RunStatus` | Execution status of a scrape run (`pending`, `running`, `completed`, `failed`). |
| [`Salary.ts`](Salary.ts) | `Salary` | Structured salary entity (min, max, currency, interval, source). |
| [`SalaryDistributionBucket.ts`](SalaryDistributionBucket.ts) | `SalaryDistributionBucket` | Histogram bucket for compensation visualization. |
| [`SalaryInterval.ts`](SalaryInterval.ts) | `SalaryInterval` | Pay period enum (`hourly`, `monthly`, `yearly`). |
| [`SalarySource.ts`](SalarySource.ts) | `SalarySource` | Origin of salary data (`structured`, `parsed_title`, `parsed_body`). |
| [`SchedulerStatus.ts`](SchedulerStatus.ts) | `SchedulerStatus` | Background scheduler state and next run timestamps. |
| [`ScrapeLogEntry.ts`](ScrapeLogEntry.ts) | `ScrapeLogEntry` | Individual log entry from a crawl execution. |
| [`ScrapeRun.ts`](ScrapeRun.ts) | `ScrapeRun` | Database record of a scrape execution pass. |
| [`ScrapedJob.ts`](ScrapedJob.ts) | `ScrapedJob` | Transport model of an incoming job listing from the worker. |
| [`Settings.ts`](Settings.ts) | `Settings` | User configuration settings entity. |
| [`Source.ts`](Source.ts) | `Source` | Database record representing a scrape target source. |
| [`SourceCadence.ts`](SourceCadence.ts) | `SourceCadence` | Schedule frequency (`hourly`, `six_hours`, `twelve_hours`, `daily`, `manual`). |
| [`SourceKind.ts`](SourceKind.ts) | `SourceKind` | Ingestion type (`greenhouse`, `lever`, `ashby`, `jsonld`, `selector`). |
| [`TransientSourceInput.ts`](TransientSourceInput.ts) | `TransientSourceInput` | Ephemeral source configuration used during dry-run testing. |
| [`TriState.ts`](TriState.ts) | `TriState` | Three-state boolean enum (`yes`, `no`, `unknown`). |
| [`TriggerType.ts`](TriggerType.ts) | `TriggerType` | Run trigger origin (`manual`, `scheduled`, `workflow`). |
| [`UpdateCheckResult.ts`](UpdateCheckResult.ts) | `UpdateCheckResult` | Application update check response. |
| [`UpdateSourceInput.ts`](UpdateSourceInput.ts) | `UpdateSourceInput` | Input payload for modifying an existing source. |
| [`UpsertStats.ts`](UpsertStats.ts) | `UpsertStats` | Statistics on inserted, updated, and skipped jobs during crawl ingestion. |
| [`WorkAuthRequirement.ts`](WorkAuthRequirement.ts) | `WorkAuthRequirement` | Work authorization enum (`citizen_only`, `us_person`, `open`, `unknown`). |
| [`WorkerConfig.ts`](WorkerConfig.ts) | `WorkerConfig` | Full configuration payload passed to `scrape-worker` via stdin. |
| [`WorkerEvent.ts`](WorkerEvent.ts) | `WorkerEvent` | Streaming event emitted by `scrape-worker` over stdout. |
| [`WorkerFilters.ts`](WorkerFilters.ts) | `WorkerFilters` | Pre-filter rules applied by the worker process. |
| [`WorkerLimits.ts`](WorkerLimits.ts) | `WorkerLimits` | Rate limits and timeout limits for worker crawls. |
| [`WorkerSource.ts`](WorkerSource.ts) | `WorkerSource` | Source definition within `WorkerConfig`. |
| [`WorkerStats.ts`](WorkerStats.ts) | `WorkerStats` | Summary statistics emitted at the end of a worker pass. |
| [`WorkplaceType.ts`](WorkplaceType.ts) | `WorkplaceType` | Workplace arrangement enum (`onsite`, `hybrid`, `remote`, `unknown`). |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`serde_json/`](serde_json/README.md) | Bindings for raw JSON representations. |
