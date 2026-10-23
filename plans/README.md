Latest implementation revision: **searchable workflow role and company selection**,
specified in [Plan 13](13-searchable-workflow-role-company-selection.md).
The role catalog includes all detailed BLS 2018 occupations and pinned O*NET
31.0 technology alternate titles; company filters attach only known available
boards, preserve source identity, and leave imported drafts unchanged until an
explicit user action. Keyword completion reuses current workflow context. No
matching semantics, persisted schema, or workflow/protocol versions changed.
The [implementation evidence](13-implementation-evidence.md) records passing
browser/frontend/Rust checks and the blocked native workflow acceptance gate.

Current completed integration base: [Plan 12 — Workflow ingestion repair](12-workflow-ingestion-repair.md), based on [Incident 28](errors/28-empty-seeded-workflows.md). Sections 1–8 are implemented and verified with [evidence](12-implementation-evidence.md). Plan 12 remains complete; Plan 13 adds editor selection behavior without changing its provider, persistence, or execution contracts.

Prior revision: [Reliable ingestion, multi-source workflows, and UI repair](11-reliable-ingestion-and-ui-repair.md). Sections 1–8 are implemented and verified; original-report reproduction remains open because source screenshots/configuration are missing. See the [Plan 11 evidence ledger](11-implementation-evidence.md).

Plan 11 remains a completed prior revision. Its eight implementation sections, evidence and unresolved original-report reproduction status are retained in [the Plan 11 ledger](11-implementation-evidence.md).

Plan 14 is the active, one-section-at-a-time workflow usability repair. Sections 1, 4, 6, 9 and 10 are implemented and evidence-recorded; Section 9 native selection/preview/save acceptance remains open. Section 6 enables both existing safe public feeds for new workflows and records why USAJOBS and the reviewed NYC dataset are not supported. Sections 2, 3, 7, 8, 11 and 12 remain open; see the [evidence ledger](14-implementation-evidence.md). The legacy static role catalog/XLSX pipeline is superseded by Section 9.

Current UI refinement and independent workflow configuration is specified in
[plan 10](10-ui-workflow-refinement.md). It supersedes conflicting global
workflow settings and Scraper Settings decisions in plans 06–08, while
preserving data-protection and domain contracts.

| [`14-workflow-editor-usability.md`](14-workflow-editor-usability.md) | One-section-at-a-time workflow editor repair: navigation, themed controls, criteria, geography, sources, clock, Review, role/company search, colors and end-to-end acceptance. |
| [`13-searchable-workflow-role-company-selection.md`](13-searchable-workflow-role-company-selection.md) | Offline BLS/O*NET literal role catalog, searchable multi-select role/company filters, atomic company-board attachment and contextual keyword autocomplete. |
| [`10-ui-workflow-refinement.md`](10-ui-workflow-refinement.md) | Themed accessible menus, independent workflow settings, automatic retention, Kanban movement, and consolidated sorting. |

# Plan status

[Interface simplification and real-source verification](07-interface-simplification.md)
is the implemented UI revision, with [verification evidence](07-interface-implementation-evidence.md). It updates presentation
choices such as separate workflow step buttons while preserving the data and
execution contracts below.

[Trustworthy scraping](06-trustworthy-scraping.md) is the current behavior and
verification specification. Older plans below are historical design records;
conflicting claims about permanent deletion, guessed icons, browser live data,
client-only filtering, or scheduling are superseded. See also
[Workflow v3](workflow-v3.md) and [database recovery](database-recovery.md).

# `plans/` — Architecture Specifications & Implementation Blueprints

## Overview

The `plans/` directory serves as the engineering ledger for the Scrape project. It contains phased specifications, technical architecture designs, UI/UX blueprints, and database migration roadmaps that have guided development since the project's inception.

---

## Architectural & Phased Plans Manifest

| Document | Focus & Scope |
|---|---|
| [`00-master-plan.md`](00-master-plan.md) | **Master Plan**: System-wide architecture, stack choices (Tauri v2, SQLite, React, Tailwind v4, Visx), data flows, and phased milestones. |
| [`01-database-config.md`](01-database-config.md) | **Database & Storage**: SQLite schema design, connection pooling, and configuration storage. |
| [`02-background-services.md`](02-background-services.md) | **Background Services**: Sidecar architecture, scheduler loops, and system tray integration. |
| [`03-frontend-ui.md`](03-frontend-ui.md) | **Frontend UI**: Brutalist NieR-inspired UI foundations, navigation, and core views. |
| [`03b-enrichment-filters.md`](03b-enrichment-filters.md) | **Enrichment & Filtering**: Heuristics for salary, visa sponsorship, clearance, and experience extraction. |
| [`03c-custom-scrape-jobs.md`](03c-custom-scrape-jobs.md) | **Custom Scrape Jobs**: CSS selector and regex scraper engines for non-ATS career websites. |
| [`03d-ergonomics-typography-filters.md`](03d-ergonomics-typography-filters.md) | **Ergonomics & Typography**: System font choices, keyboard shortcuts, and clock-face time pickers. |
| [`03e-notifications-and-updater.md`](03e-notifications-and-updater.md) | **Notifications & Updates**: In-app alert notification center and update checking. |
| [`03f-telemetry-console-and-workflow-hardening.md`](03f-telemetry-console-and-workflow-hardening.md) | **Telemetry Console**: Real-time run streaming, error logs, and workflow validation. |
| [`03g-workflow-statuses-job-discovery-and-log-filtering.md`](03g-workflow-statuses-job-discovery-and-log-filtering.md) | **Workflow Statuses & Logs**: Discovery metrics, run states, and log filter controls. |
| [`03h-workflow-policy-engine-and-execution-log-test.md`](03h-workflow-policy-engine-and-execution-log-test.md) | **Policy Engine**: Declarative YAML policy engine for crawler rules and execution logging. |
| [`03i-discovered-panes-dedupe-and-company-icons.md`](03i-discovered-panes-dedupe-and-company-icons.md) | **Panes & Company Icons**: Discovered layout split panes, job deduplication, and Google favicon icons. |
| [`03j-discovered-card-view-and-consolidated-job-window.md`](03j-discovered-card-view-and-consolidated-job-window.md) | **Card View & Single Window**: Kanban-style cards for discovered jobs and unified detail view. |
| [`03k-job-url-navigation-and-authenticity.md`](03k-job-url-navigation-and-authenticity.md) | **URL Navigation & Authenticity**: Deep-linking directly to authentic job postings and in-office tagging. |
| [`03l-job-deletion-and-confirmation-dialog.md`](03l-job-deletion-and-confirmation-dialog.md) | **Job Deletion & Tombstones**: Confirmation modals, permanent job deletion, and database tombstones. |
| [`04-metrics-dashboard.md`](04-metrics-dashboard.md) | **Metrics Dashboard**: Application funnel charts, salary distribution plots, and Visx data visualizations. |
| [`05-ci-cd.md`](05-ci-cd.md) | **CI/CD & Release Pipeline**: GitHub Actions workflows, cross-platform builds, and signing. |

---

## Subdirectories

| Directory | Description |
|---|---|
| [`assets/`](assets/README.md) | Visual design references, UI palettes, and dithering assets. |
| [`errors/`](errors/README.md) | Documented solutions and playbooks for cross-cutting technical issues. |
