# `src/lib` — Core Frontend Business Logic & Libraries

## Overview

The `src/lib` directory contains the Tauri IPC client, Zustand UI state store, local company logo cache, formatting utilities, YAML workflow bridges, and explicit browser demonstration (`?demo=1`) helpers. In desktop execution, SQLite (`src-tauri/src/db/jobs.rs`) is authoritative for job filtering, sorting, counting, and pagination.

---

## File Manifest

| File | Purpose & Key Exports |
|---|---|
| [`api.ts`](api.ts) | **Tauri IPC Client**: Strongly-typed wrappers for backend commands, including native indexed role-title search (`searchRoleTitles`) and bounded company suggestions (`searchCompanies`); browser demonstration mode does not claim either native SQLite index is populated. |
| [`companyLogoCache.ts`](companyLogoCache.ts) | **Local Logo Asset Cache**: Content-addressed in-memory cache (`loadCompanyLogoAsset`, `clearCompanyLogoCache`) that reads verified PNG artwork once per reference over local IPC (`get_company_logo_asset`). |
| [`constants.ts`](constants.ts) | **Application Constants**: Status display metadata (`STATUS_CONFIG`), tracked pipeline stages (`TRACKED_STAGES`), role metadata (`ROLE_CONFIG`), navigation tabs (`NAV_TABS`), and default filter initializers. |
| [`cronClock.ts`](cronClock.ts) | **Cron & Clock Math**: Utilities converting 24-hour clock face coordinates into cron expressions and `HH:MM` time strings. |
| [`dataCleanup.ts`](dataCleanup.ts) | **Data Cleanup State Sync**: Shared `applyDataCleanup` and `applyAutomaticRetention` handlers that clear stale selections/Undo, reset pagination and React Query caches, and clear `scrape:` preferences and logo cache on full reset. |
| [`errors.ts`](errors.ts) | **IPC Error Normalizer**: Extracts readable messages (`errorMessage`) from serialized Tauri `{ kind, message }` errors and JavaScript `Error` instances. |
| [`events.ts`](events.ts) | **Backend Event Subscriptions**: Strongly-typed listeners wrapping `@tauri-apps/api/event` (`scrape://event`, `jobs://changed`, `sources://changed`, `scheduler://status`, `retention://updated`, `data://cleaned`). |
| [`filter.ts`](filter.ts) | **Display Formatters & Demo Filter Mirror**: Display helpers (`formatSalary`, `formatExperience`, `formatWorkAuth`, `triStateToLabel`, `countActiveFilters`) and a client-side mirror (`filterJobs`, `sortJobs`) used only in explicit `?demo=1` browser mode and parity tests. Production desktop queries filter, sort, and paginate in SQLite. |
| [`formatters.ts`](formatters.ts) | **Date, Time & Number Formatters**: Formats timestamps according to user-selected date (`MM/DD/YY`, `DD/MM/YY`, `MM/DD/YYYY`, `DD/MM/YYYY`, `YYYY-MM-DD`) and time (`12h` / `24h`) preferences, plus relative times and compact numbers. |
| [`jobLocationLabel.ts`](jobLocationLabel.ts) | **Location Label Formatter**: Formats normalized `ParsedLocation` records and `WorkplaceType` into concise card/table/inspector location summaries. |
| [`jobSort.ts`](jobSort.ts) | **Sort Options & Persistence**: Defines `JOB_SORT_OPTIONS`, view-contextual defaults (`date_added_desc` for Discovered, `status_changed_at_desc` for Tracked), and persisted sort preferences. |
| [`kanbanNavigation.ts`](kanbanNavigation.ts) | **Spatial Kanban Navigation**: Algorithms for keyboard navigation across columns and cards (`Shift+Left`, `Shift+Right`, `Up`, `Down`, `j`, `k`). |
| [`locationData.ts`](locationData.ts) | **Geography Catalog Re-export**: Exposes Rust-generated `GEOGRAPHY_CATALOG` (`COUNTRIES`, `REGIONS_BY_COUNTRY`) and legacy location filter autocomplete options. |
| [`pagination.ts`](pagination.ts) | **Pagination Boundary Math**: Converts 1-based UI page numbers and page sizes into repository `limit`/`offset` parameters and bounded page counts. |
| [`positioning.ts`](positioning.ts) | **Popover Positioning Engine**: Calculates geometric viewport coordinates for tooltips, menus, and popovers to prevent screen edge clipping. |
| [`postingUrl.ts`](postingUrl.ts) | **Safe External URL Opener**: Validates `http:`/`https:` destination URLs and opens them through `@tauri-apps/plugin-opener`. |
| [`recordedExample.json`](recordedExample.json), [`recordedJobs.ts`](recordedJobs.ts) | **Recorded Provider Showcase**: Worker-generated artifact of recorded public SEP postings (with capture provenance from `fixtures/providers/manifest.json`) displayed in `?demo=1` mode. |
| [`runtime.ts`](runtime.ts) | **Runtime Mode Detection**: Distinguishes native Tauri execution from the desktop-required browser screen, explicit `?demo=1` recorded showcase, and dev-only `?demo=1&fixtures=1` synthetic test harness. |
| [`salary.ts`](salary.ts) | **Salary Input Parser**: Parses user-typed salary expressions and converts between display units and fixed-point native compensation hundredths. |
| [`schedulePresentation.ts`](schedulePresentation.ts) | **Schedule Instant Formatter**: Formats backend UTC schedule instants in the workflow's or device's IANA timezone, including date context across day/year boundaries. |
| [`status.ts`](status.ts) | **Workflow & Run Status Logic**: Maps `Source`, `ScrapeRun`, `ScanOutcome`, and `ScrapeLogEntry` records into display statuses and error/warning log filters. |
| [`store.ts`](store.ts) | **Global Zustand Store**: Central reactive UI state container (`useUIStore`) managing active navigation tab, selected job ID, persisted `JobFilter` and `JobSort` specs, pagination, modal states, and user preferences. |
| [`toast.ts`](toast.ts) | **Toast Dispatch Store**: Imperative and reactive notification store (`useToastStore`) supporting action buttons (e.g. soft-removal Undo). |
| [`workflowImport.ts`](workflowImport.ts) | **Shared YAML Import Pipeline**: Parses uploaded or edited YAML documents and validates them through the canonical Rust `validate_workflow` command without mutating the active draft on failure. |
| [`yamlJob.ts`](yamlJob.ts) | **Legacy v1 YAML Bridge**: Deprecated v1 compatibility serializer retained for historical migration/compatibility tests. |
| [`yamlWorkflow.ts`](yamlWorkflow.ts) | **WorkflowSpec v4 YAML Serializer**: Converts between `Source` records, `WorkflowSpec` v4 drafts, and declarative YAML representations. |

App list queries use view/filter/sort/page keys and SQLite repository pages rather than
fetching a capped set for client filtering. Changing filters or sort resets to the first
page; older responses populate their own query keys and cannot overwrite newer
results. Navigation counts use independent repository totals. Native filter values
are persisted under a versioned storage key, including hourly cents and interval.

Workflow file/YAML import uses the desktop `import_workflow` quarantine command.
The single draft preserves import origin through Save, so an imported executable
configuration cannot inherit previously stored trust. Browser demonstrations
reject browser/script extraction and cannot grant script trust.

Company autocomplete queries verified career-board templates and prefix-matched
employer names from locally saved postings through `searchCompanies`. Unknown
employers remain literal filter values; no company URL is inferred, and the
suggestions are not an exhaustive directory.

Plan 12 section 2 adds generated `SeedWorkflowReport` IPC results. The seed UI
reports created, retained and needs-review workflows. Script review uses the
owned child's saved configuration/fingerprint; aggregate parent review retains
the validated multi-source draft. Existing raw/invalid originals remain guarded.

Workflow compatibility is resolved by Rust. `yamlWorkflow.ts` serializes/parses
supported configuration without migrating legacy matching into structured rules.
`api.describeWorkflowFilters` returns generated `FilterCompatibilityRestriction`
entries from desktop IPC; browser demonstrations preserve restrictions and report
that authoritative explanations require desktop, rather than implementing a copy.

Plan 12 Section 5 removes independent TypeScript template definitions from `yamlWorkflow.ts`: it consumes `src/generated/workflowTemplates.json` under generated Rust contracts. Run `npm run gen:templates` to refresh picker/YAML/extractor projections. Disabled workflows may contain no enabled source during review; execution still requires one. Historical demo filter/extraction settings come from the explicitly restrictive catalog example and stay visibly labelled.
