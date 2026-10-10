# Phase 3.6 — Custom Company Scrape Jobs, Cadence Scheduling & YAML Specs

**Goal:** Allow users to monitor any company career site (especially lesser-known companies and startups without standard ATS portals) as an independent **Scrape Job**. Jobs can be scraped on-demand, scheduled on independent cadences (or custom 6-field cron), tested with live dry-runs, inspected via telemetry drawers, and configured through either a brutalist form or declarative YAML.

← [Phase 3.5](./03b-enrichment-filters.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 4](./04-metrics-dashboard.md)

---

## 3.6.0 Motivation & Mental Model

In standard job boards, users are locked into platform-wide scraping schedules and predetermined ATS adapters. However, candidates frequently track specific target employers—regional firms, boutique engineering consultancies, stealth startups—that publish postings directly on their own corporate domains (`https://acmecorp.com/careers`).

### Progressive Extraction Cascade
```
[User inputs URL: https://acmecorp.com/careers]
                     │
                     ▼
       ┌───────────────────────────┐
       │ Tier 1: ATS Auto-Detector │ ── Found Greenhouse/Lever/Ashby? ──► [Auto-configured ATS Job]
       └─────────────┬─────────────┘
                     │ No
                     ▼
       ┌───────────────────────────┐
       │ Tier 2: JSON-LD Detector  │ ── Found schema.org/JobPosting? ──► [Auto-configured JSON-LD Job]
       └─────────────┬─────────────┘
                     │ No
                     ▼
       ┌───────────────────────────┐
       │ Tier 3: Custom Selectors  │ ── Inputs CSS selectors & regex ──► [Configures Selector Engine]
       │         & Regex Capture   │
       └─────────────┬─────────────┘
                     │ User clicks [TEST SCRAPER]
                     ▼
       ┌───────────────────────────┐
       │ Tier 4: Dry-Run Preview   │ ── Verifies Title, Link, Location ─► [Save & Activate Job]
       └───────────────────────────┘
```

---

## 3.6.1 Cadence Matrix & Custom Cron

Every Scrape Job supports independent execution cadence:

| Cadence Option | Behavior | Cron Mapping / Interval |
|---|---|---|
| `INHERIT_GLOBAL` | Runs whenever the master scraper cron fires (default: daily at 12:00) | `global` |
| `HOURLY` | Runs every hour | `0 0 * * * *` |
| `EVERY_6H` | Runs every 6 hours | `0 0 */6 * * *` |
| `EVERY_12H` | Runs every 12 hours | `0 0 */12 * * *` |
| `DAILY` | Runs daily at 09:00 local time | `0 0 9 * * *` |
| `WEEKLY` | Runs Monday at 09:00 local time | `0 0 9 * * MON` |
| `MANUAL_ONLY` | Never runs automatically; only triggered via `[RUN NOW]` | `manual` |
| `CUSTOM_CRON` | Arbitrary user-defined 6-field cron expression | User input with real-time preview of next 3 run times |

### Scheduler Engine
The `SchedulerService` runs a 60-second polling tick:
```sql
SELECT id, cadence, cadence_cron, next_run_at FROM sources
WHERE enabled = 1
  AND cadence != 'global'
  AND cadence != 'manual'
  AND next_run_at <= datetime('now');
```
When due, each source is scraped in isolation via `runner.trigger_single_source(app, &source.id)` and its `next_run_at` timestamp is recalculated.

---

## 3.6.2 Declarative YAML Specification

To give power users full portability and text-editor friendliness, jobs can be viewed, edited, exported, and imported as clean YAML:

```yaml
version: 1
company: Acme Corp
url: https://acmecorp.com/careers
cadence:
  type: every_12h # options: global, hourly, every_6h, every_12h, daily, weekly, manual, cron
  cron: "0 0 */12 * * *"
engine:
  kind: selector
  selectors:
    item: ".job-listing"
    title: "h3.title"
    link: "a.apply-link"
    location: ".location-pill"
    salary: ".compensation"
  regex:
    salary: '(?P<value>\$\d+[\d,]*\s*-\s*\$\d+[\d,]*)'
```

- **Storage:** SQLite remains the transactional database of record (guaranteeing ACID referential integrity with `jobs.source_id` and indexed queries).
- **UI:** The Add/Edit Job modal features a toggle between **[FORM VIEW]** and **[YAML SPEC]**. Changes in either view sync bidirectionally.
- **Import/Export:** One-click copy/paste of YAML specifications.

---

## 3.6.3 Database Migration — `0004_scrape_jobs_cadence.sql`

```sql
-- Migration 0004: Scrape jobs cadence, execution duration, and single source runs

ALTER TABLE sources ADD COLUMN cadence TEXT NOT NULL DEFAULT 'global'
  CHECK (cadence IN ('global', 'hourly', 'every_6h', 'every_12h', 'daily', 'weekly', 'manual', 'cron'));
ALTER TABLE sources ADD COLUMN cadence_cron TEXT;
ALTER TABLE sources ADD COLUMN next_run_at TEXT;
ALTER TABLE sources ADD COLUMN last_run_duration_ms INTEGER;

ALTER TABLE scrape_runs ADD COLUMN source_id TEXT REFERENCES sources(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_sources_cadence ON sources(cadence, enabled);
CREATE INDEX IF NOT EXISTS idx_scrape_runs_source_id ON scrape_runs(source_id, started_at DESC);
```

---

## 3.6.4 Backend IPC Protocol & Commands

```rust
// 1. Run a single source on demand into SQLite
#[tauri::command]
pub async fn run_source_now(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<ScrapeRun>;

// 2. Pre-flight test of transient config via sidecar stdin without saving
#[tauri::command]
pub async fn test_source_transient(
    app: AppHandle,
    state: State<'_, AppState>,
    input: TransientSourceConfig,
) -> Result<Vec<WorkerEvent>>;

// 3. List recent runs for a specific source
#[tauri::command]
pub async fn list_source_runs(
    state: State<'_, AppState>,
    source_id: String,
    limit: Option<i64>,
) -> Result<Vec<ScrapeRun>>;
```

---

## 3.6.5 UI/UX Specification (NieR Brutalist)

1. **SCRAPE JOBS Registry View:**
   - Replaces static Sources list under `SCRAPER` tab.
   - Header with `[ALL]`, `[ATS]`, `[CUSTOM]`, `[ERRORS]` filter segmented toggle.
   - Per-row action buttons: `[RUN]`, `[TEST]`, `[EDIT]`, `[DELETE]`.
   - Status indicators: `[ OK ]` (Lime), `[ EMPTY ]` (Lavender), `[ ERROR ]` (Cherry), `[ RUNNING ]` (Signal).
2. **Inspector Drawer:**
   - Clicking a row expands telemetry: URL, cadence, lifetime postings discovered, last 5 runs, error logs.
   - Direct button `[VIEW POSTINGS IN INBOX →]` pre-filtering the Discovered inbox.
3. **Add/Edit Modal:**
   - 4-step workflow: Target URL & Auto-Detect → Metadata & Cadence → Selectors & Regex → Live Dry-Run Preview.
   - Dual-mode editor: `[FORM]` vs `[YAML]`.

---

## 3.6.6 Automated Test Coverage Matrix

- [x] **Database Migration 0004:** Unit test verifying SQLite migration, column additions, check constraints, and index creation.
- [x] **Cadence Calculation:** Unit tests for `next_run_at` computation across presets and custom cron expressions.
- [x] **YAML Conversion:** Round-trip tests converting between `Source` / input structures and YAML strings.
- [x] **Single Source Execution:** Runner test verifying `trigger_single_source` runs only the target source and updates `last_run_duration_ms`.
- [x] **Frontend Vitest Suite:**
  - Cadence picker preset selection and custom cron input.
  - YAML preview toggle and syntax parsing.
  - Single-source scrape trigger button and status reflection.
