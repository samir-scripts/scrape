# Phase 3.7 — Autonomous Scrape Workflows, Data Retention, Job Favoriting, YAML Validation & Airflow Observability

**Goal:** Transform the Scraper feature from fragmented global tabs into self-contained **Autonomous Static Workflows** modeled after **Apache Airflow DAGs** and **Google Cloud Composer**. Consolidate the Scraper hub into `[ WORKFLOWS ]` and `[ EXECUTION LOGS ]`, implement a TTL **Data Retention Engine** with a strict multi-tier safety shield (protecting tracked applications, notes, and **favorited** postings), provide real-time **YAML Live Validation**, and display auto-dismissing **Bottom-Right Error Toasts**.

← [Phase 3.5](./03b-enrichment-filters.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 4](./04-metrics-dashboard.md)

---

## 3.7.0 Motivation & Architectural Mental Model

In traditional architectures, schedule and ingestion filters are global settings, while target sources are configured separately. This creates friction: a user monitoring Stripe for Staff Distributed Systems in SF on an hourly basis cannot simultaneously monitor an early-stage startup for Junior Rust on a weekly basis without rules clashing. Furthermore, scraped postings accumulate indefinitely without a TTL policy.

### The Static DAG Pipeline Lifecycle
Every scrape target is an autonomous linear pipeline DAG:
```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  1. EXTRACT  │ ──► │  2. FILTER   │ ──► │  3. ENRICH   │ ──► │  4. STORE    │ ──► │ 5. RETENTION │
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
 ATS / Selectors       Per-Source Roles,    Salaries, Geo,       SQLite Upsert &      Purge Expired
 HTML Extraction       Keywords, Locations  YOE, Sponsorship     Deduplication        Untracked Jobs
```

### Streamlined Scraper Hub Layout
The root `SCRAPER` hub eliminates separate `SCHEDULE` and `FILTERS` sub-tabs, consolidating into two clean categories:
1. **`[ WORKFLOWS ]`**: Airflow-style DAG registry & inspector.
2. **`[ EXECUTION LOGS ]`**: Central observability hub with brutalist red square (`■`) error diagnostics.

---

## 3.7.1 Data Retention Engine (TTL Policy) & Favorites Safety Shield

### Three-Tier Protection Shield
A job posting is **strictly protected** and will **NEVER** be pruned by the data retention engine if:
1. **Tracked Status**: Status is in `applied`, `interviewing`, `offer`, or `rejected`.
2. **Candidate Notes**: `notes IS NOT NULL AND notes != ''`.
3. **Favorited**: `favorited = 1`.

### Pruning SQL Query
```sql
DELETE FROM jobs
WHERE source_id = ?
  AND status IN ('scraped', 'viewed', 'ignored')
  AND favorited = 0
  AND (notes IS NULL OR notes = '')
  AND date_added <= datetime('now', '-' || ? || ' days');
```

---

## 3.7.2 Job Favoriting Architecture & UI Interactions

- **Schema:** `favorited INTEGER NOT NULL DEFAULT 0` and `favorited_at TEXT` in `jobs` with index `CREATE INDEX idx_jobs_favorited ON jobs(favorited, date_added DESC);`.
- **Hotkey:** Key `S` (Star / Save / Favorite) toggles favorite state for the active job in Discovered and Tracked views.
- **Discovered View:** Star toggle button `[★] / [☆]` in `ListRow` and `JobMetaPanel` triage actions.
- **Tracked View:** Star badge `★` on Kanban cards and sortable column in Tracked list.
- **Quick Filter:** `[★ FAVORITES]` toggle button in `SubNav` filters down to favorited postings with one click.

---

## 3.7.3 Declarative YAML Specification v2 & Real-Time Validation

```yaml
version: 2
metadata:
  name: "Stripe — Core Infrastructure"
  company: "Stripe"
  enabled: true
extractor:
  kind: greenhouse
  url: "https://boards.greenhouse.io/stripe"
schedule:
  cadence: daily
  time: "09:00"
  cron: "0 0 9 * * *"
filters:
  role_types: [full_time, contract]
  include_keywords: ["Software Engineer", "Rust"]
  exclude_keywords: ["Manager"]
  locations: ["Remote", "San Francisco, CA"]
retention:
  days: 30
  policy: untracked_only
```

### Real-Time Validation & Bottom-Right Error Toasts
1. **Live Editor Feedback:** `[✓ PARSABLE · SPEC V2]` (lime) vs `[■ SYNTAX ERROR: Line X, Col Y]` (cherry red).
2. **Save Enforcement:** If the user attempts to save a workflow with invalid YAML or parameters:
   - Save is blocked.
   - An auto-dismissing brutalist toast appears in the **bottom-right corner** (`fixed bottom-4 right-4 z-50`):
     `"YAML could not be saved due to parsing issues or invalid parameters: <error details>"`
   - Automatically disappears after 5 seconds or via manual `[×]` click.

---

## 3.7.4 Automated Test Matrix

- [x] **Database Migration 0005:** Verify `favorited`, `retention_days`, `retention_policy`, and `filters` columns and indexes.
- [x] **Retention Pruning Test:** Expired postings that are favorited, tracked, or have notes are NOT deleted.
- [x] **YAML Validator Tests (`yaml_workflow.test.ts`):** Valid v2 specs, syntax errors with line/column coordinates, parameter errors.
- [x] **Toast Store Tests (`toast.test.ts`):** Triggering error toast, 5000ms auto-dismissal, manual dismissal.
- [x] **Hotkeys Test:** Pressing `S` toggles favorite state.
- [x] **Workflow Modal Test:** Attempting save with invalid YAML triggers bottom-right toast and aborts save.
