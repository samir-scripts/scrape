# Reliable ingestion, multi-source workflows, and UI repair

Status: Section 1 diagnostics foundation and Section 2 keyword entry implemented and verified; original-report reproduction, later adapter outcomes, and Sections 3–8 remain open. See [implementation evidence](11-implementation-evidence.md). Continue one numbered section at a time. Source: user-approved plan, 2026-10-16. This document records the implementation contract; requirements below are not claims of shipped behavior. Preserve plans 06–09's identity, provenance, matching, suppression, retention, scheduling, migration, and user-data protections.

This plan supersedes plan 10's time-format radio control and Kanban surface decisions. Use a themed Select for time format and explicit Paper board/column surfaces. Other plan 10 contracts remain in force.

## Agreed defaults

- One workflow accepts 20 matching jobs total across its sources; refreshed postings count. Preview accepts at most min(configured limit, 10).
- Required company coverage: Meta, Apple, Amazon, Netflix, Google, Cloudflare, SEP, Linear, Bitwarden, and authored local fixtures. Unsupported or blocked sites remain open acceptance gaps.
- Kanban uses Paper #F5F5DC, cream cards #FFF6EE, ink #1A1A1A, and restrained lavender #C6C2F2, lime #D6DC82, cherry #D86072 accents.
- Custom extraction includes CSS, XPath, regex, authenticated sessions, JavaScript scripts, and Python/Selenium with user-managed tooling. All paths use Rust validation, matching, accounting, and persistence.

## 1. Reproducible failures and diagnostics

Reproduce keyword typing through controls. Compare unrestricted, title Engineer, and the exact reported configuration against the same recorded Cloudflare response. Preserve all effective restrictions. Return matching decisions and structured reasons from authoritative Rust predicates; prove boolean, worker, and repository parity. Give each rejected candidate one primary category; explicitly label overlapping failures in bounded representative examples.

Report provider total only when supplied, candidates examined, invalid/duplicates, rejection categories, blocked, accepted, inserted/refreshed, enrichment warnings, and coverage termination (exhausted/limit/request or time bound/cancelled/failed). Counts must reconcile and must not imply all provider pages were visited.

Evidence availability: the six original screenshots and exact active workflow configuration were not attached to this implementation session. Existing review assets are preserved. Do not claim to have reproduced the exact 438/436/2 run from a reconstructed configuration.

## 2. Keyword entry and matching

Use reusable chips with unfinished-text state. Enter/Add/comma/newline commit; paste splits comma/newline; spaces preserve phrases. Ignore empty terms; case-insensitive deduplication preserves first spelling. Named removal, Backspace focus before deletion, IME safety, wrapping/scrolling. Commit unfinished text before Preview, Next, Save, and exports. Standard matching exposes Keywords, Any/All, Exclude, Search in (Title default). Mixed imported Any/All groups remain separately visible and conjunctive. Preserve substring behavior and Rust KeywordRules; no frontend matcher.

## 3. Bounded discovery and enrichment

Discover → normalize authoritative available evidence → reject only conclusively ineligible candidates → enrich unresolved evidence → final match → accept. Detail responses may change fields; prove early rejection safe per adapter. Prioritize required eligibility evidence; optional detail failure is a warning, required evidence failure yields partial coverage. Stop scheduling at the cap. Keep URL protections, cancellation, cache, pacing, request/time bounds, and unknown date/pay semantics. Verify late eligible candidates after many irrelevant records through actual worker processes.

Outcomes: success for complete scans or intentional job limit; success with warnings for optional enrichment failures; partial for missing coverage/required evidence; failed for no usable source, invalid configuration, protocol/persistence failures; cancelled for explicit cancellation with persisted counts retained.

## 4. Company branding

One shared resolver prioritizes explicit employer metadata, matching JobPosting hiringOrganization.logo, employer-labelled source branding, then configured selectors. Resolve relative URLs against evidence page. Reject platform/guessed/favicon/social-preview substitutions. Employer identity is separate from workflow display name. Persist original URL, evidence URL/method, validation time, optional cached asset reference; cache per employer/source, retain last verified asset on temporary failure. Bound and protect fetches. Shared component: 24px list/card, 32px inspector, aspect ratio, light backing, no unnecessary padding, accessible initials. Test missing/mismatch/platform/relative/broken/wide/transparent/cache/offline/refresh/native visibility.

## 5. Multi-source workflow ownership and fair budgeting

WorkflowSpec v5 has nonempty sources[] (ID, label, enabled, employer hint, extractor/config). Workflow owns matching, schedule/startup, total cap, retention, aggregate state. No per-source matching overrides. Forward migration creates parent ownership and converts existing workflows to one source while preserving source/job IDs, user state, suppression, history, schedules; invalid originals remain Needs review. Backup and populated historical tests required.

Sources editor supports add/edit/enable/reorder/remove, independent detection/validation/test, duplicate normalized endpoint rejection, source attribution, at least one enabled source. Removal stops ingestion and preserves saved jobs/history/retention ownership.

Coordinator visits sources in rounds with at most one eligible unique posting per source per round, resumable adapters and bounded discovery yields. Redistribute capacity; rotate persisted starting source, previews do not rotate; workflow-wide canonical deduplication and persistence cap. Three healthy sources yield 7/7/6 at 20. Slow/failed sources consume no slot. Run all has one cap per workflow; cancellation ends all child processes.

## 6. Browser and script extraction

Pinned Playwright runner and explicit managed-browser installation; missing runtime gives setup guidance, no scheduled auto-download. Expose container/field/detail/next/load-more/readiness locators, CSS/XPath, attributes/text, Rust regex transforms, navigation timeout/page/duration/session controls, bounded selector test samples. Initial limits: one process, two contexts, 30s navigation, 100 pages/180s source, 600s workflow; authoritative config and truthful termination.

Open/Save/Clear isolated sessions, external restrictive storage and OS credentials where applicable; exports contain references only. Expiry is Needs login; scheduled runs never unexpectedly open login windows.

Versioned stdin/NDJSON commands/candidates/continuations/diagnostics/completion; JS managed runner, explicitly selected Python interpreter/environment and Selenium template/dependency diagnostics. Rust owns acceptance; backpressure/cancellation, bounded redacted stderr, strict malformed/exit/version failures. Structured argv, process-tree termination/reaping. Imported executable configs disabled until trusted; edits invalidate trust. Explain scripts run with local privileges, without claiming OS sandboxing or guaranteed challenge bypass. Test authored dynamic/authenticated fixtures through both modes and persistence.

## 7. Provider/company verification

Manifest-driven recorded and separately labelled live verification, per company. Inspect official sites, capture permitted listing/detail/pagination/branding contracts with URL/time/checksum/extractor version/classification. Existing adapters only when compatible; otherwise versioned extractors. Replay actual worker → temporary database. No live CI. All FAANG entries need explicit evidence; blocked/unavailable/unsupported remain incomplete.

## 8. Regional settings and Kanban

Time Select: “12-hour · 3:25 PM”, “24-hour · 15:25”, matching date control and preserving save/error semantics; aligned wide, stacked narrow.

Explicit board/column/card/stage semantic tokens; Paper colors above, dark Obsidian base with matching palette and legible labels. Applied lavender, Interviewing cream with ink boundary, Offer lime, Rejected cherry. Shared mapping across tags/list/inspector/drag preview. Stage names remain essential indicators. Remove repeated page-scope paragraphs; keep “0 loaded” and “No jobs on this page”. Preserve pointer/keyboard movement, pending/failure recovery, selection, screen-reader instructions. Verify both themes/native/200% zoom; text 4.5:1 and controls/focus 3:1; visually inspect snapshot changes.

## Delivery and verification

Inspect/checkpoint before each section, commit coherent behavior/tests/docs with explicit staging and staged review. Generate TS boundary types from Rust, never hand-edit. Forward-only schema changes and consistent backup; Git revert is insufficient database rollback. Never test on live user storage or fabricate production evidence.

At integration completion: Rust format, Clippy, workspace tests, generated-binding drift, frontend tests, TypeScript/build, styles, browser journeys, isolated native startup/IPC/workflow checks. Record actual commands/results and classify deterministic fixtures, browser QA, native QA, live checks and unperformed checks separately. All acceptance requirements must pass before claiming this revision complete.
