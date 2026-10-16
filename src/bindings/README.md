# `src/bindings` — TypeScript Type Bindings (ts-rs)

## Overview

The `src/bindings` directory contains TypeScript type definitions and interfaces generated automatically from Rust structs and enums in `crates/scrape-core` and `src-tauri` using the `ts-rs` crate. This guarantees strict compile-time type safety between the Rust backend and the React frontend.

> **Note**: Do not edit files in this directory manually. Regenerate them from Rust with `npm run gen:types` (`cargo test --workspace export_bindings`) and verify zero drift with `git diff --exit-code src/bindings`.

---

## File Manifest

| Binding File | Underlying Rust Type / Export | Description |
|---|---|---|
| [`AppNotification.ts`](AppNotification.ts) | `AppNotification` | In-app notification entity with id, kind, title, message, read state, timestamp. |
| [`CalendarSchedule.ts`](CalendarSchedule.ts) | `CalendarSchedule` | Per-workflow IANA calendar schedule (local time, ISO weekdays, timezone, startup flag). |
| [`ClassifiedClearance.ts`](ClassifiedClearance.ts) | `ClassifiedClearance` | Clearance classification result with `ClearanceLevel` and confidence. |
| [`ClassifiedSponsorship.ts`](ClassifiedSponsorship.ts) | `ClassifiedSponsorship` | Visa sponsorship result with `TriState` flag and matched snippet. |
| [`ClassifiedWorkAuth.ts`](ClassifiedWorkAuth.ts) | `ClassifiedWorkAuth` | Work authorization requirement (`citizen_only`, `us_person`, `open`, `unknown`). |
| [`CleanupMode.ts`](CleanupMode.ts), [`CleanupPreview.ts`](CleanupPreview.ts), [`CleanupRequest.ts`](CleanupRequest.ts), [`CleanupResult.ts`](CleanupResult.ts) | `Cleanup*` | Data Management bulk cleanup modes, preview counts, validated request, and snapshot result. |
| [`ClearanceLevel.ts`](ClearanceLevel.ts) | `ClearanceLevel` | Clearance requirement enum (`none`, `secret`, `top_secret`, etc.). |
| [`CompanyLogo.ts`](CompanyLogo.ts), [`LogoEvidence.ts`](LogoEvidence.ts), [`LogoMethod.ts`](LogoMethod.ts), [`LogoStatus.ts`](LogoStatus.ts) | `CompanyLogo`, `Logo*` | Employer branding evidence, verification status, extraction method, and local cached asset reference. |
| [`CompensationFilter.ts`](CompensationFilter.ts), [`CompensationRange.ts`](CompensationRange.ts) | `Compensation*` | Native compensation range (fixed-point hundredths, currency, interval) and filter criteria. |
| [`CountryOption.ts`](CountryOption.ts), [`RegionOption.ts`](RegionOption.ts), [`geographyCatalog.ts`](geographyCatalog.ts) | `GEOGRAPHY_CATALOG` | Generated ISO country and qualified subdivision options owned by `scrape-core::geography`. |
| [`CoverageTermination.ts`](CoverageTermination.ts), [`ScanOutcome.ts`](ScanOutcome.ts), [`SourceScanStats.ts`](SourceScanStats.ts) | `CoverageTermination`, `ScanOutcome`, `SourceScanStats` | Reconciled per-source scan diagnostics, coverage termination cause, and execution outcome (`success`, `success_with_warnings`, `partial`, `failed`, `cancelled`). |
| [`CreateNotificationInput.ts`](CreateNotificationInput.ts) | `CreateNotificationInput` | Payload for creating an in-app notification. |
| [`CreateSourceInput.ts`](CreateSourceInput.ts), [`UpdateSourceInput.ts`](UpdateSourceInput.ts), [`TransientSourceInput.ts`](TransientSourceInput.ts) | `*SourceInput` | Input payloads for creating, updating, and dry-run testing sources. |
| [`CronValidationResult.ts`](CronValidationResult.ts) | `CronValidationResult` | Validation response for cron syntax check. |
| [`DescriptionBlock.ts`](DescriptionBlock.ts), [`DescriptionFormat.ts`](DescriptionFormat.ts), [`DescriptionInline.ts`](DescriptionInline.ts), [`JobDescription.ts`](JobDescription.ts) | `JobDescription`, `Description*` | Sanitized structured job description AST (blocks, headings, lists, links) and input format metadata. |
| [`EnrichmentLimits.ts`](EnrichmentLimits.ts), [`ingestionLimits.ts`](ingestionLimits.ts) | `EnrichmentLimits`, `INGESTION_LIMITS` | Bounded detail enrichment limits (`max_requests`, `timeout_ms`) and per-workflow ingestion limit constants. |
| [`ExperienceClassification.ts`](ExperienceClassification.ts) | `ExperienceClassification` | Parsed min/max years of experience and `is_new_grad` flag. |
| [`FilterField.ts`](FilterField.ts), [`KeywordRules.ts`](KeywordRules.ts), [`PostingDateFilter.ts`](PostingDateFilter.ts), [`PostingRules.ts`](PostingRules.ts) | `PostingRules`, `KeywordRules`, … | Authoritative workflow matching rules for Any/All/Exclude keywords, fields, geography, workplace, pay, and posting dates. |
| [`FilterImpactPreview.ts`](FilterImpactPreview.ts) | `FilterImpactPreview` | Estimated job match count when testing filter adjustments. |
| [`Job.ts`](Job.ts) | `Job` | Primary job application entity stored in SQLite and displayed in the UI. |
| [`JobFilter.ts`](JobFilter.ts), [`JobSort.ts`](JobSort.ts) | `JobFilter`, `JobSort` | Repository filter criteria and sorting configuration. |
| [`JobStatus.ts`](JobStatus.ts), [`JobStatusEvent.ts`](JobStatusEvent.ts) | `JobStatus`, `JobStatusEvent` | Application pipeline state (`discovered`, `applied`, `interviewing`, `offer`, `rejected`) and audit log event. |
| [`LegacyCadence.ts`](LegacyCadence.ts), [`LegacyEngine.ts`](LegacyEngine.ts), [`LegacyRegex.ts`](LegacyRegex.ts), [`LegacySelectors.ts`](LegacySelectors.ts), [`LegacyWorkflowSpec.ts`](LegacyWorkflowSpec.ts) | `Legacy*` | Legacy v1 workflow specification compatibility types. |
| [`LocationOptions.ts`](LocationOptions.ts) | `LocationOptions` | Available location suggestions and filter counts. |
| [`LogLevel.ts`](LogLevel.ts) | `LogLevel` | Diagnostic log level (`debug`, `info`, `warn`, `error`). |
| [`Metrics.ts`](Metrics.ts), [`MetricsSeriesBucket.ts`](MetricsSeriesBucket.ts), [`MetricsStreak.ts`](MetricsStreak.ts), [`MetricsTotals.ts`](MetricsTotals.ts), [`RoleTypeMetric.ts`](RoleTypeMetric.ts), [`SalaryDistributionBucket.ts`](SalaryDistributionBucket.ts) | `Metrics*`, `RoleTypeMetric`, `SalaryDistributionBucket` | Analytics dashboard totals, velocity time-series, streaks, role breakdowns, and salary histogram buckets. |
| [`NotificationKind.ts`](NotificationKind.ts) | `NotificationKind` | Notification category (`crawl_completed`, `crawl_failed`, `job_match`). |
| [`Page.ts`](Page.ts) | `Page<T>` | Generic paginated repository response wrapper. |
| [`ParsedLocation.ts`](ParsedLocation.ts) | `ParsedLocation` | Normalized location with city, state/region, country, remote flag, and raw provenance. |
| [`PostingIdentity.ts`](PostingIdentity.ts) | `PostingIdentity` | Canonical posting identity key and provenance metadata. |
| [`PreviewPosting.ts`](PreviewPosting.ts), [`WorkflowPreview.ts`](WorkflowPreview.ts) | `WorkflowPreview`, `PreviewPosting` | Live workflow preview matches, counts, and scan diagnostics. |
| [`RejectionCount.ts`](RejectionCount.ts), [`RejectionExample.ts`](RejectionExample.ts), [`RejectionReason.ts`](RejectionReason.ts) | `Rejection*` | Categorized primary rejection counts and representative examples with overlapping reasons. |
| [`RemovalResult.ts`](RemovalResult.ts), [`RemovedPosting.ts`](RemovedPosting.ts) | `RemovalResult`, `RemovedPosting` | Soft-removal result and removed/blocked posting registry record. |
| [`RetentionStatus.ts`](RetentionStatus.ts) | `RetentionStatus` | Last automatic retention sweep timestamp, counts, and consecutive failure state. |
| [`RoleType.ts`](RoleType.ts) | `RoleType` | High-level role enum (`swe`, `data`, `product`, `design`, `seasonal`, `other`). |
| [`RunCounts.ts`](RunCounts.ts), [`RunStatus.ts`](RunStatus.ts), [`ScrapeLogEntry.ts`](ScrapeLogEntry.ts), [`ScrapeRun.ts`](ScrapeRun.ts) | `ScrapeRun`, `Run*`, `ScrapeLogEntry` | Scrape execution record, status, reconciled run/source counts, and log entry. |
| [`Salary.ts`](Salary.ts), [`SalaryInterval.ts`](SalaryInterval.ts), [`SalarySource.ts`](SalarySource.ts) | `Salary*` | Structured compensation entity, interval (`hourly`, `monthly`, `yearly`), and provenance source. |
| [`ScheduledWorkflow.ts`](ScheduledWorkflow.ts), [`SchedulerStatus.ts`](SchedulerStatus.ts) | `SchedulerStatus`, `ScheduledWorkflow` | Scheduling pause/run state, enabled workflow count, and next dispatch candidates. |
| [`ScrapedJob.ts`](ScrapedJob.ts) | `ScrapedJob` | Transport model of an incoming job listing from the worker. |
| [`Settings.ts`](Settings.ts) | `Settings` | User configuration settings entity. |
| [`Source.ts`](Source.ts), [`SourceCadence.ts`](SourceCadence.ts), [`SourceKind.ts`](SourceKind.ts) | `Source*` | Persisted workflow/source record, schedule frequency, and adapter kind (`greenhouse`, `lever`, `ashby`, `jsonld`, `selector`). |
| [`TriState.ts`](TriState.ts) | `TriState` | Three-state boolean enum (`yes`, `no`, `unknown`). |
| [`TriggerType.ts`](TriggerType.ts) | `TriggerType` | Run trigger origin (`manual`, `scheduled`, `startup`, `workflow`). |
| [`UpdateCheckResult.ts`](UpdateCheckResult.ts) | `UpdateCheckResult` | Application update check response. |
| [`UpsertStats.ts`](UpsertStats.ts) | `UpsertStats` | Statistics on inserted, updated, and skipped jobs during crawl ingestion. |
| [`WorkAuthRequirement.ts`](WorkAuthRequirement.ts) | `WorkAuthRequirement` | Work authorization enum (`citizen_only`, `us_person`, `open`, `unknown`). |
| [`WorkerConfig.ts`](WorkerConfig.ts), [`WorkerEvent.ts`](WorkerEvent.ts), [`WorkerFilters.ts`](WorkerFilters.ts), [`WorkerLimits.ts`](WorkerLimits.ts), [`WorkerSource.ts`](WorkerSource.ts), [`WorkerStats.ts`](WorkerStats.ts) | `Worker*` | Subprocess NDJSON protocol v6 configuration, streaming events, filters, limits, and completion stats. |
| [`WorkflowExtractor.ts`](WorkflowExtractor.ts), [`WorkflowFilters.ts`](WorkflowFilters.ts), [`WorkflowMetadata.ts`](WorkflowMetadata.ts), [`WorkflowRetention.ts`](WorkflowRetention.ts), [`WorkflowSchedule.ts`](WorkflowSchedule.ts), [`WorkflowSpec.ts`](WorkflowSpec.ts) | `Workflow*` | Authoritative `WorkflowSpec` v4 structure shared across form editing, YAML import/export, preview, and execution. |
| [`WorkplaceType.ts`](WorkplaceType.ts) | `WorkplaceType` | Workplace arrangement enum (`onsite`, `hybrid`, `remote`, `unknown`). |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`serde_json/`](serde_json/README.md) | Bindings for raw JSON representations. |
