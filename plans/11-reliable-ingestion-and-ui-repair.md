# Reliable ingestion, multi-source workflows, and UI repair

**2026-10-18 acceptance correction:** [Incident 28](errors/28-empty-seeded-workflows.md) reproduces seeded workflow ownership, matching and extraction defects. [Plan 12](12-workflow-ingestion-repair.md) reopens affected Sections 5–7 acceptance; the completion status below describes earlier checkpoints, not verified current seeded-workflow behavior.

Historical status: Sections 1–8 (diagnostics foundation, keyword chips, bounded enrichment, employer branding, multi-source workflows & fair budgeting, low-footprint static/browser/script extraction across Phases 6A–6F, provider/company verification, and regional settings & Kanban surfaces) implemented and verified. Original-report reproduction remains open (screenshots/config missing). Plan 11 implementation complete. See [implementation evidence](11-implementation-evidence.md). Source: user-approved plan, 2026-10-16. This document records the implementation contract; requirements below are not claims of shipped behavior. Preserve plans 06–09's identity, provenance, matching, suppression, retention, scheduling, migration, and user-data protections.

This plan supersedes plan 10's time-format radio control and Kanban surface decisions. Use a themed Select for time format and explicit Paper board/column surfaces. Other plan 10 contracts remain in force.

## Agreed defaults

- One workflow accepts 20 matching jobs total across its sources; refreshed postings count. Preview accepts at most min(configured limit, 10).
- Required company coverage: Meta, Apple, Amazon, Netflix, Google, Cloudflare, SEP, Linear, Bitwarden, and authored local fixtures. Unsupported or blocked sites remain open acceptance gaps.
- Kanban uses Paper #F5F5DC, cream cards #FFF6EE, ink #1A1A1A, and restrained lavender #C6C2F2, lime #D6DC82, cherry #D86072 accents.
- Custom extraction uses a three-tier, low-memory/low-CPU Rust-native architecture: (1) static HTTP with CSS, pure-Rust XPath 1.0 over `html5ever`, regex transforms, and SPA hydration JSON extraction; (2) direct Rust Chrome DevTools Protocol (CDP) over WebSocket driving an existing local Chromium-based browser or configured CDP endpoint with aggressive resource blocking (`Image`/`Media`/`Font`/`Stylesheet`/trackers), pre-document stealth evasion, and isolated authenticated sessions; and (3) a versioned stdin/NDJSON script runner supporting lightweight JS or explicitly selected local interpreters (`node`/`python3`) with dependency diagnostics. All paths use authoritative Rust validation, matching, accounting, and persistence. See the [Section 6 execution blueprint](11-section-6-browser-and-script-extraction.md).

## 1. Reproducible failures and diagnostics

Reproduce keyword typing through controls. Compare unrestricted, title Engineer, and the exact reported configuration against the same recorded Cloudflare response. Preserve all effective restrictions. Return matching decisions and structured reasons from authoritative Rust predicates; prove boolean, worker, and repository parity. Give each rejected candidate one primary category; explicitly label overlapping failures in bounded representative examples.

Report provider total only when supplied, candidates examined, invalid/duplicates, rejection categories, blocked, accepted, inserted/refreshed, enrichment warnings, and coverage termination (exhausted/limit/request or time bound/cancelled/failed). Counts must reconcile and must not imply all provider pages were visited.

Evidence availability: the six original screenshots and exact active workflow configuration were not attached to this implementation session. Existing review assets are preserved. Do not claim to have reproduced the exact 438/436/2 run from a reconstructed configuration.

## 2. Keyword entry and matching

Use reusable chips with unfinished-text state. Enter/Add/comma/newline commit; paste splits comma/newline; spaces preserve phrases. Ignore empty terms; case-insensitive deduplication preserves first spelling. Named removal, Backspace focus before deletion, IME safety, wrapping/scrolling. Commit unfinished text before Preview, Next, Save, and exports. Standard matching exposes Keywords, Any/All, Exclude, Search in (Title default). Mixed imported Any/All groups remain separately visible and conjunctive. Preserve substring behavior and Rust KeywordRules; no frontend matcher.

## 3. Bounded discovery and enrichment

Discover → normalize authoritative available evidence → reject only conclusively ineligible candidates → enrich unresolved evidence → final match → accept. Detail responses may change fields; prove early rejection safe per adapter. Prioritize required eligibility evidence; optional detail failure is a warning, required evidence failure yields partial coverage. Stop scheduling at the cap. Keep URL protections, cancellation, cache, pacing, request/time bounds, and unknown date/pay semantics. Verify late eligible candidates after many irrelevant records through actual worker processes.

Implementation contract: nonempty Greenhouse listing fields are the provider snapshot; detail responses fill gaps and do not overwrite its original ID/title/destination/description. This supersedes the former detail-over-listing merge precedence, while preserving plans 06–09’s identity and provenance protections. Default detail allowance remains 100 requests and 45 seconds, now spent on eligible/unresolved candidates rather than the first 100 records.

Outcomes: success for complete scans or intentional job limit; success with warnings for optional enrichment failures; partial for missing coverage/required evidence; failed for no usable source, invalid configuration, protocol/persistence failures; cancelled for explicit cancellation with persisted counts retained.

## 4. Company branding

One shared resolver prioritizes explicit employer metadata, matching JobPosting hiringOrganization.logo, employer-labelled source branding, then configured selectors. Resolve relative URLs against evidence page. Reject platform/guessed/favicon/social-preview substitutions. Employer identity is separate from workflow display name. Persist original URL, evidence URL/method, validation time, optional cached asset reference; cache per employer/source, retain last verified asset on temporary failure. Bound and protect fetches. Shared component: 24px list/card, 32px inspector, aspect ratio, light backing, no unnecessary padding, accessible initials. Test missing/mismatch/platform/relative/broken/wide/transparent/cache/offline/refresh/native visibility.

## 5. Multi-source workflow ownership and fair budgeting

WorkflowSpec v5 has nonempty sources[] (ID, label, enabled, employer hint, extractor/config). Workflow owns matching, schedule/startup, total cap, retention, aggregate state. No per-source matching overrides. Forward migration creates parent ownership and converts existing workflows to one source while preserving source/job IDs, user state, suppression, history, schedules; invalid originals remain Needs review. Backup and populated historical tests required.

Sources editor supports add/edit/enable/reorder/remove, independent detection/validation/test, duplicate normalized endpoint rejection, source attribution, at least one enabled source. Removal stops ingestion and preserves saved jobs/history/retention ownership.

Coordinator visits sources in rounds with at most one eligible unique posting per source per round, resumable adapters and bounded discovery yields. Redistribute capacity; rotate persisted starting source, previews do not rotate; workflow-wide canonical deduplication and persistence cap. Three healthy sources yield 7/7/6 at 20. Slow/failed sources consume no slot. Run all has one cap per workflow; cancellation ends all child processes.

## 6. Browser and script extraction

Use a low-footprint, three-tier Rust extraction architecture (detailed in [11-section-6-browser-and-script-extraction.md](11-section-6-browser-and-script-extraction.md)): (1) static HTTP fast path with single-pass `html5ever` DOM parsing (`scraper`), pure-Rust XPath 1.0 evaluation over the same DOM tree, and SPA hydration JSON extraction (`__NEXT_DATA__`, `__NUXT_DATA__`); (2) direct Rust Chrome DevTools Protocol (CDP) client over WebSocket driving an existing local Chromium-based browser (Chrome, Brave, Edge, Chromium) or configured CDP endpoint without bundling a heavy Node/Playwright or Python/Selenium runtime; missing browser runtime gives actionable setup guidance and never auto-downloads on a schedule. Enforce aggressive CDP resource blocking (`Image`, `Media`, `Font`, `Stylesheet`, trackers) and pre-document stealth evasion to minimize RAM and CPU. Expose container/field/detail/next/load-more/readiness locators, CSS/XPath, attributes/text, Rust regex transforms, navigation timeout/page/duration/session controls, and bounded selector test samples. Initial limits: one shared browser process, two contexts (immediate tab teardown per turn), 30s navigation, 100 pages/180s source, 600s workflow; authoritative config and truthful termination (`PageLimit`, `TimeLimit`, `NeedsLogin`).

Open/Save/Clear isolated sessions in external restrictive storage (`0600`/`0700`) and OS credentials where applicable; exports contain `session_id` references only. Expiry or login redirect is `Needs login` (`needs_login`); scheduled runs never unexpectedly open visible login windows.

Versioned stdin/NDJSON commands/candidates/continuations/diagnostics/completion (`SCRIPT_PROTOCOL_VERSION = 1`); lightweight JS runner or explicitly selected local interpreter (`node`, `python3`, or custom binary) with structured `argv` (never `sh -c`) and environment/dependency diagnostics. Rust owns candidate validation, regex transforms, canonical identity, matching, and acceptance; enforce backpressure/cancellation, bounded redacted stderr (16 KiB), strict malformed/exit/version failures, and process-tree termination/reaping. Imported executable configs start disabled until explicitly trusted; edits invalidate trust. Explain scripts run with local privileges, without claiming OS sandboxing or guaranteed challenge bypass. Test authored dynamic/authenticated fixtures through static/CDP and script modes into SQLite persistence.

## 7. Provider/company verification

Manifest-driven recorded and separately labelled live verification, per company. Inspect official sites, capture permitted listing/detail/pagination/branding contracts with URL/time/checksum/extractor version/classification. Existing adapters only when compatible; otherwise versioned extractors. Replay actual worker → temporary database. No live CI. All FAANG entries need explicit evidence; blocked/unavailable/unsupported remain incomplete.

## 8. Regional settings and Kanban

Time Select: “12-hour · 3:25 PM”, “24-hour · 15:25”, matching date control and preserving save/error semantics; aligned wide, stacked narrow.

Explicit board/column/card/stage semantic tokens; Paper colors above, dark Obsidian base with matching palette and legible labels. Applied lavender, Interviewing cream with ink boundary, Offer lime, Rejected cherry. Shared mapping across tags/list/inspector/drag preview. Stage names remain essential indicators. Remove repeated page-scope paragraphs; keep “0 loaded” and “No jobs on this page”. Preserve pointer/keyboard movement, pending/failure recovery, selection, screen-reader instructions. Verify both themes/native/200% zoom; text 4.5:1 and controls/focus 3:1; visually inspect snapshot changes.

## Delivery and verification

Inspect/checkpoint before each section, commit coherent behavior/tests/docs with explicit staging and staged review. Generate TS boundary types from Rust, never hand-edit. Forward-only schema changes and consistent backup; Git revert is insufficient database rollback. Never test on live user storage or fabricate production evidence.

At integration completion: Rust format, Clippy, workspace tests, generated-binding drift, frontend tests, TypeScript/build, styles, browser journeys, isolated native startup/IPC/workflow checks. Record actual commands/results and classify deterministic fixtures, browser QA, native QA, live checks and unperformed checks separately. All acceptance requirements must pass before claiming this revision complete.
