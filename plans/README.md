Current priority: [Workflow ingestion repair — Plan 12](12-workflow-ingestion-repair.md), following [Incident 28](errors/28-empty-seeded-workflows.md). Sections 1–5 are implemented with [evidence](12-implementation-evidence.md): diagnostics, ownership, matching compatibility, field evidence/geography and the canonical template catalog/portable script setup. Section 6 site extraction/pagination/URL identity is next. Saved user filters remain unchanged; disabled examples do not establish provider coverage. Provider and native acceptance remains open. Historical completion statements below are superseded for that scope.

Current revision: [Reliable ingestion, multi-source workflows, and UI repair](11-reliable-ingestion-and-ui-repair.md). Sections 1 diagnostics, 2 keywords, 3 bounded enrichment/outcomes, 4 employer branding, 5 multi-source workflow ownership, 6 low-resource browser and script extraction (Phases 6A–6F), 7 provider/company verification, and 8 regional settings and Kanban have [implementation evidence](11-implementation-evidence.md); Sections 1–8 are fully implemented and verified. Original-report reproduction remains open (screenshots/config missing). Plan 11 implementation complete. Next: Desktop usability and extraction (Plan 08 native acceptance) or subsequent roadmap priorities. See [Section 6 execution blueprint](11-section-6-browser-and-script-extraction.md) for browser and script extraction and [Section 7 execution blueprint](11-section-7-provider-and-company-verification.md) for provider and company verification.

The latest proposed desktop repair is [Desktop usability and extraction](08-desktop-usability-and-extraction.md). It documents the screenshot-reported regressions, replaces conflicting plan 07 layout decisions, and requires actual native interaction verification before completion.

Current UI refinement and independent workflow configuration is specified in
[plan 10](10-ui-workflow-refinement.md). It supersedes conflicting global
workflow settings and Scraper Settings decisions in plans 06–08, while
preserving data-protection and domain contracts.

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
