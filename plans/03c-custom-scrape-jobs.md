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

### Streamlined Scraper Hub Layout (Sidebar-Free & Consolidated Logs)
The root `SCRAPER` hub eliminates the redundant 2-item left sidebar (`[ WORKFLOWS ]` / `[ EXECUTION LOGS ]`). It provides a full-bleed, master-detail operational console:
1. **Full-Bleed Workflows Table:** Reclaims 100% horizontal canvas width for workflow telemetry, status pips, schedules, and actions.
2. **Consolidated Workflow Logs:** Execution logs belong to individual workflows. Clicking a workflow or `[LOGS]` opens an integrated inspector and live terminal log stream scoped specifically to that workflow.
3. **Dual-Mode Engine (Simple vs. Advanced):** An instant segmented switch in the Scraper header `[ MODE: ■ SIMPLE | ADVANCED ]` toggles between streamlined 2-field creation and deep power-user pipeline controls.
4. **Brutalist Monospace Hover Tooltips:** Replaces cluttered inline explanation copy with high-contrast, zero-radius hover blurbs (300ms delay) over headers, buttons, and field labels.
5. **Decluttered & Expanded Modal:** Expands from cramped `max-w-lg` (512px) to `max-w-xl` (Simple) and `max-w-5xl` (Advanced), completely eliminating multi-sentence helper paragraphs.

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

## 3.7.4 Scraper Ergonomics, Dual-Mode Engine & Brutalist Tooltip System

### 1. Master-Detail Layout & Workflow-Scoped Execution Logs
- **Sidebar Retirement:** The permanent 224px left sidebar in `ScraperView.tsx` is eliminated.
- **Workflow-Scoped Telemetry & Logs Dock:**
  - Selecting any workflow row or clicking `[LOGS]` opens a bottom split-dock / slide-up inspector panel.
  - Sub-views:
    1. **Recent Runs:** Isolated run history (`listSourceRuns(source.id)`), execution durations, +new postings, status badges.
    2. **Terminal Stream:** Terminal-style stdout/stderr and worker event stream (`getScrapeLogs(run.id)`) with log level filters (`ALL`, `INFO`, `WARN`, `ERROR`).
    3. **Action Triggers:** `[RUN WORKFLOW NOW]` executed in isolation with real-time streaming progress.

### 2. Dual-Mode Scraper Engine (Simple vs. Advanced)
- **Synced Mode State:** Persisted in `useUIStore.scraperMode` (`simple` | `advanced`) and `localStorage`, with an optional setting in `Settings > SystemSettings`.
- **Top Bar Switcher:** Segmented toggle in Scraper Hub header: `[ MODE: ■ SIMPLE | ADVANCED ]`.
- **Simple Mode Specification:**
  - Only 2 user inputs: `COMPANY / WORKFLOW NAME` and `CAREERS URL / ATS BOARD`.
  - Automatic ATS/Schema detection on paste/blur with visual pip `[✓ GREENHOUSE DETECTED]`.
  - Zero-configuration defaults: Daily at 09:00 AM cadence, 30-day untracked TTL retention, all role types enabled.
  - Modal dimensions: `max-w-xl` (576px), single viewport, zero scrolling.
- **Advanced Mode Specification:**
  - Full 5-tier pipeline configuration: Extractor (ATS / JSON-LD / Custom CSS Selectors + Regex), Schedule (Presets, ClockPicker, 6-field Cron), Ingestion Filters (Role types, Keywords, Locations), Data Retention TTL, and dual-mode `[FORM VIEW]` vs `[YAML SPEC V2]`.
  - Modal dimensions: `max-w-5xl` (1024px) split view.
  - Auto-elevation guard: If an existing workflow with custom selectors or custom cron is edited, the modal automatically opens in Advanced Mode.

### 3. NieR Brutalist Monospace Hover Tooltips (`Tooltip.tsx`)
- **Zero NPM Dependencies:** Custom portal-based hover tooltip component (`createPortal(..., document.body)`).
- **Brutalist Tokens:** 0px border-radius, 1px high-contrast border, `font-mono text-[10px] uppercase tracking-wider`, inverted palette (`bg-[var(--color-ink)] text-[var(--color-paper)]`).
- **Timing:** 300ms hover delay (avoids mouse-skimming visual flicker), instant 0ms dismissal on mouseleave or Escape.
- **Offloaded Copy:** Replaces inline helper text and hint paragraphs across buttons, table headers, and form labels.

---

## 3.7.5 Automated Test Coverage Matrix

- [x] **Database Migration 0005:** Verify `favorited`, `retention_days`, `retention_policy`, and `filters` columns and indexes.
- [x] **Retention Pruning Test:** Expired postings that are favorited, tracked, or have notes are NOT deleted.
- [x] **YAML Validator Tests (`yaml_workflow.test.ts`):** Valid v2 specs, syntax errors with line/column coordinates, parameter errors.
- [x] **Toast Store Tests (`toast.test.ts`):** Triggering error toast, 5000ms auto-dismissal, manual dismissal.
- [x] **Hotkeys Test:** Pressing `S` toggles favorite state.
- [x] **Workflow Modal Test:** Attempting save with invalid YAML triggers bottom-right toast and aborts save.
- [ ] **Brutalist Tooltip Tests (`tooltip.test.ts`):** Verify 300ms delay, portal DOM mounting, 0ms unmount, and keyboard/escape dismissal.
- [ ] **Simple Mode Workflow Creation Tests:** Verify 2-field form initializes and saves with correct default schedule, retention, and detected adapter.
- [ ] **Workflow-Scoped Logs Tests:** Verify `api.listSourceRuns` and `api.getScrapeLogs` render properly inside the workflow detail inspector dock.
