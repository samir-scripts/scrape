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
| [`discovered/`](discovered/README.md) | **Discovered Jobs Feed** | High-density card grid or table of freshly crawled listings, split-pane inspector, authentic job URL opener, favoriting, and tombstone deletion. |
| [`tracked/`](tracked/README.md) | **Application Tracker** | Drag-and-drop Kanban board and tabular list tracking job applications across `APPLIED`, `INTERVIEWING`, `OFFER`, and `REJECTED` stages. |
| [`scraper/`](scraper/README.md) | **Crawler Control Room** | Source management, manual crawl trigger buttons, active execution monitoring, and streaming NDJSON log console. |
| [`metrics/`](metrics/README.md) | **Analytics Dashboard** | Visx visualizations showing application velocity over time, role distribution donuts, salary distribution histograms, and funnel conversion rates. |
| [`settings/`](settings/README.md) | **Configuration Center** | Six-tab configuration panel managing sources, automated cron crawl schedules, filter rules, appearance themes, database backups, and logs. |
