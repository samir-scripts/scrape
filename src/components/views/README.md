# `src/components/views` — Application Screens & Primary Views

## Overview

The `src/components/views` directory contains the top-level page views corresponding to the primary navigation tabs in the `TopNav` bar.

---

## View Manifest

```
src/components/views/
├── discovered/   # Ingested, unreviewed job listings feed
├── tracked/      # Job application pipeline (Kanban board & list view)
├── scraper/      # Web crawler control center and live execution telemetry
├── metrics/      # Application conversion metrics and visual charts
└── settings/     # User preferences, sources, schedules, and maintenance
```

| View Directory | Primary Purpose | Key Features |
|---|---|---|
| [`discovered/`](discovered/README.md) | **Discovered Jobs Feed** | High-density card grid or table of freshly crawled listings, adjustable `ResizableSplitPane` inspector (`JobInspector`), unread indicator, authentic job URL opener, favoriting, and soft removal with optional blocking. |
| [`tracked/`](tracked/README.md) | **Application Tracker** | Handle-driven Kanban board and 6-column default tabular list tracking job applications across `APPLIED`, `INTERVIEWING`, `OFFER`, and `REJECTED` stages. |
| [`scraper/`](scraper/README.md) | **Workflow Registry & Execution** | Independent workflow management (**Add workflow**, **Run all**, **Import YAML**), manual and preview triggers, cancellation, and reconciled run/source diagnostic logs. |
| [`metrics/`](metrics/README.md) | **Analytics Dashboard** | Visx visualizations showing application velocity over time, role distribution donuts, salary distribution histograms, and funnel conversion rates. |
| [`settings/`](settings/README.md) | **Preferences & Workflow Editor** | Four-category settings screen (**Appearance**, **System**, **Removed & blocked jobs**, **Data management**) plus the shared workflow configuration components (`AddSourceModal`, `WorkflowSections`, `KeywordMatching`, `GeographyMatching`, `ScanDiagnostics`). |
