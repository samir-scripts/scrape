# Phase 3.5 — Job Enrichment, Custom Filters & Scraper Logs

**Goal:** Every job carries structured **salary**, **location** (city / state / country / remote), **visa sponsorship**, and **security clearance** data. Users can filter on all of it, both in the UI (instantly, over stored jobs) and in the scraper (at ingest, in Settings → SCRAPER → ADVANCED). Settings also gains a dedicated **LOGS** section displaying chronological scraper run logs, with runs requiring attention marked by a brutalist red square.

← [Phase 3](./03-frontend-ui.md) · [Master Plan](./00-master-plan.md) · Next → [Phase 4](./04-metrics-dashboard.md)

> [!NOTE]
> This phase extends Phases 1–3 rather than rewriting them. It has its own migration (`0002`), protocol fields, adapter changes, UI work, and scraper execution log inspector. Branch: `feature/phase-3b-enrichment-filters`.

---

## 3.5.0 Research findings (drive the design)

| Source | Salary | Location | Description text (needed for sponsorship/clearance) |
|---|---|---|---|
| **Greenhouse** `GET /v1/boards/{t}/jobs` | Not in list. Needs a per-job call `GET /jobs/{id}?pay_transparency=true` → `pay_input_ranges[] { min_cents, max_cents, currency_type, title, blurb }`. **No interval field** (have to infer "hour"/"year" from `title`/`blurb`). Only present if the employer has turned on pay transparency. | `location.name` is free text (`"NYC"`, `"Remote - US"`). `?content=true` adds `offices[]`. | `?content=true` on the list endpoint returns HTML-escaped `content`. |
| **Lever** `GET /v0/postings/{s}?mode=json` | `salaryRange { min, max, currency, interval }` (optional) + `salaryDescriptionPlain`. | `country` (ISO 3166-1 alpha-2, may be null), `workplaceType` (`on-site / remote / hybrid / unspecified`), `categories.allLocations[]`. | `descriptionPlain`, `lists[]` (requirements etc.), `additionalPlain`. |
| **Ashby** `GET /posting-api/job-board/{n}` | Only with `?includeCompensation=true` → `compensation.summaryComponents[] { compensationType, minValue, maxValue, currencyCode, interval }`. | `address.postalAddress { addressLocality, addressRegion, addressCountry }`, `secondaryLocations[]`, `isRemote`, `workplaceType`. | `descriptionPlain`. |
| **JSON-LD `JobPosting`** | `baseSalary` (`MonetaryAmount` → `value.minValue/maxValue/unitText`, `currency`). | `jobLocation.address` (`addressRegion`, `addressCountry`), `jobLocationType: "TELECOMMUTE"`, `applicantLocationRequirements`. | `description` (HTML). Also optional `securityClearanceRequirement` and `eligibilityToWorkRequirement`. When present, these win over the text classifier. |
| **Selector** | Optional `salary` CSS selector, or regex over the item's text. | `location` selector → gazetteer. | Optional `description` selector. |

**What this means:**
1. **No ATS has a structured sponsorship or clearance field.** Both have to come from a **text classifier over the job description**, which means we have to fetch descriptions. Every result keeps the **sentence that triggered it** so you can check it.
2. **Salary is often missing.** Pay-transparency laws only cover some jurisdictions, and Greenhouse needs an extra request per job. Any filter on salary/sponsorship/clearance needs an **"include N/A"** option, or it will quietly drop most jobs.
3. **Locations are often free text and often multiple per job.** They need normalizing into a child table (`job_locations`). Regions use ISO 3166-2 codes (`US-CA` vs `CA-ON`) so `"CA"` isn't ambiguous.

---

## 3.5.1 Schema — `src-tauri/migrations/0002_enrichment.sql`

```sql
ALTER TABLE jobs ADD COLUMN salary_min        INTEGER;               -- annualized, whole currency units
ALTER TABLE jobs ADD COLUMN salary_max        INTEGER;               -- annualized
ALTER TABLE jobs ADD COLUMN salary_currency   TEXT;                  -- ISO 4217 (USD, CAD, GBP…)
ALTER TABLE jobs ADD COLUMN salary_interval   TEXT CHECK (salary_interval IN ('year','month','week','day','hour'));
ALTER TABLE jobs ADD COLUMN salary_raw        TEXT;                  -- original string for display
ALTER TABLE jobs ADD COLUMN salary_source     TEXT CHECK (salary_source IN ('structured','text'));
ALTER TABLE jobs ADD COLUMN workplace_type    TEXT NOT NULL DEFAULT 'unknown'
                         CHECK (workplace_type IN ('remote','hybrid','onsite','unknown'));
ALTER TABLE jobs ADD COLUMN sponsorship       TEXT NOT NULL DEFAULT 'unknown' CHECK (sponsorship IN ('yes','no','unknown'));
ALTER TABLE jobs ADD COLUMN sponsorship_evidence TEXT;
ALTER TABLE jobs ADD COLUMN clearance         TEXT NOT NULL DEFAULT 'unknown' CHECK (clearance IN ('yes','no','unknown'));
ALTER TABLE jobs ADD COLUMN clearance_level   TEXT CHECK (clearance_level IN ('public_trust','confidential','secret','top_secret','ts_sci'));
ALTER TABLE jobs ADD COLUMN clearance_evidence TEXT;
ALTER TABLE jobs ADD COLUMN description_text  TEXT;                  -- plain text, capped at 32 KB
ALTER TABLE jobs ADD COLUMN enrichment_version INTEGER NOT NULL DEFAULT 0;

CREATE TABLE IF NOT EXISTS job_locations (
  job_id        TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  raw           TEXT NOT NULL,
  city          TEXT,
  region_code   TEXT,             -- ISO 3166-2, e.g. 'US-CA', 'CA-ON'
  country_code  TEXT,             -- ISO 3166-1 alpha-2, e.g. 'US'
  is_remote     INTEGER NOT NULL DEFAULT 0,
  PRIMARY KEY (job_id, raw)
);
CREATE INDEX IF NOT EXISTS idx_job_locations_geo ON job_locations(country_code, region_code);
CREATE INDEX IF NOT EXISTS idx_jobs_salary       ON jobs(salary_currency, salary_max);
CREATE INDEX IF NOT EXISTS idx_jobs_eligibility  ON jobs(sponsorship, clearance);
```

- [ ] Keep `jobs.location` (the raw display string) for backward compatibility. Backfill `job_locations` from it with the gazetteer during migration (a Rust post-migration step, not SQL).
- [ ] **Annualization:** hour × 2080, day × 260, week × 52, month × 12. Keep `salary_interval` so the UI can show the original (`$55/HR`).
- [ ] `ts-rs`: annotate every `i64` that goes over IPC with `#[ts(type = "number")]` (salary, `limit`, `offset`). This removes the BigInt workaround in `api.ts` ([solutions #12](./errors/solutions.md#12-typescript-ts-rs-bigint-ipc-serialization)).

## 3.5.2 Shared enrichment library — `crates/scrape-core/src/enrich/`

All parsing/classification lives in `scrape-core`, so the **worker** (to filter at ingest) and the **host** (to reclassify stored jobs) run identical code.

| Module | API | Notes |
|---|---|---|
| `salary.rs` | `from_structured(min, max, currency, interval)`, `from_text(&str) -> Option<Salary>` | Matches `$120,000 – $150,000`, `$120k-150k`, `USD 120,000`, `$55/hr`, `£60,000`. **Rejects false positives:** needs a salary context word nearby (`salary`, `compensation`, `pay`, `base`, `range`, `/yr`, `per hour`). Ignores `$5M Series A`, `401(k)`, `$100 stipend`, and annualized amounts < 10,000. |
| `location.rs` | `parse(raw) -> Vec<ParsedLocation>` | Splits on `;` `\|` `/` ` or `. Embedded gazetteer: US states + DC + territories, Canadian provinces, ISO 3166 country names / alpha-2 / alpha-3 + aliases (`USA`, `U.S.`, `UK`), and about 300 major cities mapped to region (`NYC` → `US-NY`). Handles `Remote - US`, `Remote (Canada)`, `Anywhere`. Ambiguity rules: `City, CA` → `US-CA`. `Georgia` → `US-GA` unless paired with `Tbilisi`. |
| `eligibility.rs` | `classify_sponsorship(text) -> Classified`, `classify_clearance(text) -> Classified { value, level, evidence }` | Works per sentence. Patterns below. **"No" beats "Yes" in the same posting** (e.g. "we sponsor for some roles but cannot sponsor for this one"). |
| `html.rs` | `to_plain_text(html, max_bytes)` | Turns ATS HTML into text for storage and classification. |

**Sponsorship patterns (case-insensitive, sentence-scoped):**
- `no` → `unable|not able|cannot|can't|will not|won't|does not|do not … sponsor`, `without (the need for)? (current or future)? (visa)? sponsorship`, `no (visa)? sponsorship`, `must be (a)? U.S. citizen`, `U.S. citizenship (is)? required`, `green card holders? only`, ITAR/EAR "U.S. person" requirements.
- `yes` → `visa sponsorship (is)? (available|provided|offered)`, `(we|will) sponsor`, `H-1B (sponsorship|transfer)s? (available|supported|welcome)`, `open to sponsoring`.
- Ignore Greenhouse application questions like "Will you require sponsorship?". Asking the question doesn't mean they refuse to sponsor.

**Clearance patterns:**
- `yes` (+ level) → `(active|current)? (TS/SCI|top secret|secret|public trust) (security)? clearance`, `security clearance (is)? required`, `must (be able to)? (obtain|maintain|hold) … clearance`, `polygraph`.
- `no` → `no (security)? clearance (is)? required`, `clearance (is)? not required`.
- Plain "clearance" with no qualifier doesn't count (e.g. "clearance sale"). The text needs `security clearance` or a level term.

- [ ] `ENRICHMENT_VERSION` constant. Bump it whenever the rules change. A host command `reclassify_jobs()` re-runs the classifiers on stored `description_text` for every row with an older `enrichment_version`, with no rescrape needed. Runs automatically on startup after an upgrade.

## 3.5.3 Worker & protocol changes

- [ ] `WorkerEvent::Job` gains fields, all `#[serde(default)]`, so `PROTOCOL_VERSION` stays `1`:
  ```json
  {"type":"job", "...": "...",
   "salary": {"min":160000,"max":210000,"currency":"USD","interval":"year","raw":"$160,000 – $210,000","source":"structured"},
   "locations": [{"raw":"San Francisco, CA","city":"San Francisco","region_code":"US-CA","country_code":"US","is_remote":false}],
   "workplace_type": "hybrid",
   "sponsorship": {"value":"no","evidence":"Applicants must be authorized to work in the U.S. without sponsorship."},
   "clearance":   {"value":"unknown","level":null,"evidence":null},
   "description_text": "…"}
  ```
- [ ] **Greenhouse:** list with `?content=true`. Per-job pay call (`?pay_transparency=true`) only for jobs that **pass the cheap pre-filters** (keyword/role/location). Uses the existing per-host semaphore and delay. Can be turned off with `scraper.fetch_details` (ADVANCED).
- [ ] **Lever:** parse `salaryRange`, `salaryDescriptionPlain`, `country`, `workplaceType`, `categories.allLocations`, and build the description from `descriptionPlain + lists + additionalPlain`.
- [ ] **Ashby:** add `?includeCompensation=true`. Take the `summaryComponents` entry with `compensationType == "Salary"`. Parse `address`, `secondaryLocations`, `isRemote`, `workplaceType`, `descriptionPlain`.
- [ ] **JSON-LD:** `baseSalary`, `jobLocation`, `jobLocationType`, `applicantLocationRequirements`, `description`, `securityClearanceRequirement`, `eligibilityToWorkRequirement`.
- [ ] **Selector:** optional `salary` and `description` selectors in the source config form.
- [ ] **Two-stage filter in the worker:** (1) cheap filter on title/role/location before any detail fetch. (2) full filter (salary/sponsorship/clearance) after enrichment. Both use `scrape_core::filter::JobPredicate`.
- [ ] Upsert: enrichment columns use `COALESCE(excluded.x, jobs.x)` so a scrape that's missing data never erases good data. `job_locations` is replaced per job inside the same transaction. `status`, `notes`, `date_added` are untouched.

## 3.5.4 Two filter layers

| | **Scraper filters** (Settings → SCRAPER) | **View filters** (Discovered / Tracked) |
|---|---|---|
| Runs in | `scrape-worker`, at ingest | SQL (`list_jobs`), at query time |
| Effect | Jobs that don't match are **never saved** | Hides saved jobs. Undo is instant. |
| Stored in | `app_settings` (`filters.*`) | Zustand + `localStorage` |
| Recommended use | Keep **broad** (drop obvious misses) | Day-to-day narrowing |

Both layers use the same filter model (`scrape_core::filter::FilterSpec`, exported to TS):

```ts
type FilterSpec = {
  include_keywords: string[]; exclude_keywords: string[];
  role_types: RoleType[];
  location: { countries: string[]; regions: string[]; remote: "any" | "remote_only" | "exclude_remote"; include_unknown: boolean };
  salary:   { min: number | null; max: number | null; currency: string; include_unknown: boolean };
  sponsorship: Tri[];        // subset of ["yes","no","unknown"]; all three = no filter
  clearance:   Tri[];
  workplace_types: WorkplaceType[];
  posted_within_days: number | null;
};
```

- **Salary matching uses range overlap:** a job matches `[lo, hi]` if `salary_max >= lo AND salary_min <= hi`. Missing bounds count as open. Jobs in other currencies only match when `include_unknown` is on (**no FX conversion**, because rates drift and the source data is approximate).
- **Location matching:** `EXISTS (SELECT 1 FROM job_locations l WHERE l.job_id = jobs.id AND (l.country_code IN (…) OR l.region_code IN (…)))`, so a job listed in "NYC; London" matches either. Built with `sqlx::QueryBuilder`.
- Loosening scraper filters later is safe. ATS APIs return the full board every run, so previously dropped jobs come back on the next scrape.

## 3.5.5 UI changes

### Shell
```
┌──────────────────────────────────────────────────────────────────────────┐
│ [■ DISCOVERED (42) ■]  TRACKED (12)  METRICS  SETTINGS     ● NEXT 18:00 │  TopNav (no logo)
├──────────────────────────────────────────────────────────────────────────┤
│ ROLE [ALL ▾]  [F] FILTERS (3)  US-CA ×  $150K–$250K ×  SPON: YES,N/A ×  [/]│  SubNav: active filters as chips
├──────────────────────────────────────────────────────────────────────────┤
│ LOCATION                 SALARY (ANNUAL, USD)        ELIGIBILITY         │  Filter panel (toggled with F;
│ COUNTRY [search…]        ├──■━━━━━━━━━━■────┤       SPONSORSHIP         │   pushes content down, no overlay)
│ ■ United States          [ 150,000 ]–[ 250,000 ]     ■ YES ■ NO ■ N/A    │
│ STATE  ■ CA □ NY □ WA    ■ INCLUDE N/A               CLEARANCE           │
│ REMOTE [ANY ▾]           CURRENCY [USD ▾]            ■ YES ■ NO ■ N/A    │
│ ■ INCLUDE N/A                                        [RESET]  [SAVE AS…] │
├───────────────────────┬───────────────────────────────┬──────────────────┤
│ Software Eng, Data    │ SOFTWARE ENGINEER, DATA       │ Salary $160–210K │
│ Stripe · SF, CA       │ STRIPE · SAN FRANCISCO, CA    │ Sponsor  NO      │
│ $160–210K  SPON N  CLR│ ───────────────────────────── │  "…without spon… │
```

- [ ] **Remove the `SCRAPE // SYS.02` logo** from `TopNav`. Tabs start at the left edge.
- [ ] **Filter panel** (`components/filters/`): `FilterPanel`, `LocationFilter`, `SalaryRangeFilter`, `TriStateFilter`, `FilterChips`. `F` toggles it, `Esc` closes it, `Shift+F` resets it. The role dropdown stays inline because it's the most-used filter.
- [ ] **`RangeSlider` primitive** (`components/ui/RangeSlider.tsx`, no library): two overlaid native `<input type="range">` (the track has `pointer-events:none`, the thumbs have `pointer-events:auto`), square thumbs styled through `::-webkit-slider-thumb` (WKWebView / WebView2 / WebKitGTK), and a fill between the thumbs. Thumbs can't cross. Bounds 0 – 500,000, step 5,000. The top stop means **"500K+"** (no upper bound). Arrow keys move 5K, `Shift`+arrow moves 25K.
- [ ] **Number inputs** next to the slider stay in sync both ways. They accept `150000`, `150,000`, and `150k`, and are validated on blur (min ≤ max, clamped).
- [ ] **Location filter:** searchable country checklist (gazetteer list). The state/province checklist appears when US or CA is selected. Remote policy uses a `<Select>`. Include-N/A checkbox.
- [ ] **Saved presets** (stretch): name a filter set. Stored in `app_settings` under `ui.filter_presets`.

### Columns & display
- [ ] **Tracked → LIST table:** add **SALARY**, **LOCATION**, **SPONSORSHIP**, **CLEARANCE** columns. All sortable.
- [ ] **Discovered list rows:** compact second line `Stripe · SF, CA · $160–210K` plus two fixed-width tags `SPON Y|N|—` and `CLR Y|N|—`.
- [ ] **Value display:** `YES` / `NO` / `N/A`. **No good/bad colors.** Whether clearance is good depends on the user. `YES` = inverted ink block, `NO` = 1px outline, `N/A` = muted text + checker dither. Hovering (or the meta panel) shows the **evidence sentence** and the clearance level (`TS/SCI`).
- [ ] **Meta panel:** rows for Salary (with original interval), Workplace, Locations (all of them), Sponsorship + evidence, Clearance + level + evidence.
- [ ] **Job detail pane:** show `description_text`. It's stored anyway, and right now the detail pane has no posting body.

### Settings → SCRAPER (replaces FILTERS)
Left category list becomes `SCHEDULE · SOURCES · SCRAPER · LOGS · APPEARANCE · SYSTEM`. SCRAPER sections:
- [ ] **KEYWORDS & ROLES:** include/exclude keyword chips, role types (existing).
- [ ] **LOCATION:** same `LocationFilter` component as the view panel.
- [ ] **COMPENSATION:** same `SalaryRangeFilter`. **"Keep jobs with no salary listed" defaults to ON.**
- [ ] **ELIGIBILITY:** sponsorship and clearance tri-state checkboxes. **N/A defaults to checked.**
- [ ] **ADVANCED:** fetch per-job details (Greenhouse pay ranges) on/off, concurrency, per-host delay, request timeout, max pages per `jsonld` source, store descriptions on/off.
- [ ] **IMPACT PREVIEW:** "These filters would keep **N of M** saved jobs." The shared predicate runs against the DB through `preview_filter_impact(spec)`, so you can see the effect before the next scrape.

### Settings → LOGS (Execution History & Error Tracking)
Dedicated audit log for background and manual scraper runs.
- [ ] **Chronological Run List:** Sorted strictly by date and time (`started_at DESC`, newest first).
- [ ] **Red Square Error Markers:**
  - Any run that encountered an issue needing user attention (`status = 'failed'` or `status = 'partial'`, or containing error-level log events such as HTTP 403/429 rate limits, network timeouts, selector extraction errors, or sidecar process crashes) is marked with a prominent brutalist **red square** (`<span className="inline-block w-2.5 h-2.5 bg-red-600 border border-red-700" />` / `■` in `#DC2626`).
  - Healthy/successful runs are marked with a muted neutral block (`▫` or `■` in `var(--color-ink-mute)`).
- [ ] **Filter Controls:** Toggle between `ALL RUNS` and `ERRORS ONLY`.
- [ ] **Master-Detail Inspector:**
  - Left pane: Scrape run cards showing status square, timestamp, trigger badge (`CRON` / `MANUAL` / `STARTUP`), duration, stats (`+N NEW · M FOUND`), and one-line error summary if failed.
  - Right pane: Full chronological execution log for the selected run:
    - Worker spawn events, per-source fetch milestones, matched counts.
    - Error lines highlighted with red square marker, full error message, HTTP status code, and target URL.
    - Actionable "EDIT SOURCE" button linking directly to the problematic board in `SOURCES` tab.

### Copy cleanup (no flavor text)
- [ ] Remove `SCRAPE // SYS.02` (TopNav).
- [ ] `SCRAPER SCHEDULE & BACKGROUND ENGINE` → `SCHEDULE`.
- [ ] `POSTINGS ARE FILTERED ON INGESTION BY THE SCRAPE WORKER SIDECAR` → `Jobs that don't match these filters are not saved.`
- [ ] Rule: every string must name a thing, a value, an action, or a consequence.

## 3.5.6 Commands & API

- [ ] `list_jobs(filter: JobFilter)`: `JobFilter` embeds `FilterSpec` plus `statuses`, `search`, `sort` (`salary_desc`, `salary_asc` added), `limit`, `offset`.
- [ ] `get_job_locations(job_id)` (or include locations inline on `Job` as `locations: ParsedLocation[]`; preferred, it saves a round trip).
- [ ] `reclassify_jobs() -> { updated }`, `preview_filter_impact(spec) -> { kept, total }`, `list_location_options() -> { countries, regions }` (only values present in the DB, so the checklists stay short).
- [ ] `list_scrape_runs(limit: Option<i64>) -> Vec<ScrapeRun>` (chronological runs, newest first).
- [ ] `get_scrape_run(run_id: String) -> ScrapeRun` and `get_scrape_logs(run_id: String) -> Vec<ScrapeLogEntry>`.

## 3.5.7 Tests

- [ ] **scrape-core:** table-driven tests. Salary parser: ≥ 25 cases including the false positives above. Location parser: ≥ 40 cases (`San Francisco, CA`, `Remote - US`, `Toronto, ON, Canada`, `London, UK`, `NYC`, `Remote (EMEA)`, `Atlanta, Georgia`, `Tbilisi, Georgia`). Sponsorship and clearance: ≥ 30 labeled sentences each, checking value **and** evidence.
- [ ] **Adapter fixtures:** Lever with `salaryRange`, Ashby with `includeCompensation`, JSON-LD with `baseSalary` + `securityClearanceRequirement`, Greenhouse list (`content=true`) + job detail (`pay_input_ranges`).
- [ ] **DB:** migration on a DB with existing data (defaults + location backfill), range-overlap query, `include_unknown` for every optional field, multi-location `EXISTS`, `COALESCE` upsert keeps existing salary.
- [ ] **Frontend:** add `@testing-library/react` + `jsdom` (**not installed yet**). Test `RangeSlider` (thumbs can't cross, keyboard steps, "500K+" top), number-input parsing (`150k`), filter store → `JobFilter` mapping, chip removal.
- [ ] **Logs tests:** Test `LogsSettings` rendering, verify runs sorted by `started_at DESC`, verify error runs render the red square marker, and verify `ERRORS ONLY` filter.

## Acceptance Criteria
- [ ] Jobs from Lever/Ashby/JSON-LD fixtures carry structured salary + location. Greenhouse jobs get salary when `pay_input_ranges` exists.
- [ ] Sponsorship and clearance show `YES` / `NO` / `N/A` with an evidence sentence for every non-N/A value.
- [ ] Discovered can be filtered by country, state, remote policy, salary range (slider **and** typed numbers), sponsorship, and clearance, entirely by keyboard.
- [ ] Settings → SCRAPER → ADVANCED filters change what the next `RUN NOW` stores, and the impact preview matches the actual result.
- [ ] Settings includes a `LOGS` section showing chronological scraper runs sorted by date and time (newest first).
- [ ] Scraper runs with errors or issues needing addressing are marked with a distinct brutalist red square (`■`).
- [ ] Selecting a run opens the inspector showing per-source execution logs, timestamped error events, and remediation actions.
- [ ] Tracked list table shows SALARY, LOCATION, SPONSORSHIP, CLEARANCE columns.
- [ ] No `SCRAPE` logo and no flavor text anywhere in the UI.
- [ ] Bumping `ENRICHMENT_VERSION` and restarting reclassifies stored jobs without a rescrape.
