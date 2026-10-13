# Trustworthy scraping, simpler workflows, and consistent UI

Status: implementation in progress. This specification supersedes conflicting behavior in older plans, especially permanent deletion and implicit browser demonstration. Source: user-supplied implementation plan, 2026-10-13. This is its implementation checklist; unchecked items are requirements, not shipped capabilities.

## Baseline

- Started on `develop` at `bfdc895`; preserved existing work on `implementation/trustworthy-scraping` in checkpoint `02dc881` (115 source/documentation paths; status directories had grouped some entries).
- Prior agent reported 326 frontend tests / 41 files, 189 Rust tests, TypeScript and Clippy passing; Rust formatting failing; Vite HTML smoke passing. These are inherited results, not new verification. Interactive UI/native QA was not performed.
- No databases, secrets, binaries, caches, or unrelated files were included in checkpoint.
- References: [NieR](assets/ref-nier-menu.png), [dither](assets/ref-dither.png), [palette](assets/ref-palette.png). Follow root [AGENTS.md](../AGENTS.md).

## Fixed product decisions

- Simple workflow: Source URL → actual matches preview → schedule and save. Editable detected name and board/adapter; concrete detection failures and Advanced route. Optional title/location. Show fetched/matched/blocked/invalid counts; permit zero matches with explanation.
- New workflows default to daily 09:00 in explicit device IANA timezone. Manual/Daily are simple options. Show next run and summary. Save never scrapes implicitly; offer Run now separately.
- Preview uses execution's extraction, enrichment, validation, matching and block checks; no persistence, retention or schedule advancement. Invalidate on any draft change; bounded/cancellable and labeled when truncated.
- Hourly filters match explicitly hourly native compensation only. Annual filters match explicitly annual compensation only. No implicit FX or interval conversion.
- Removal soft-removes all copies of an exact posting. Unchecked option: “Keep this job out of future scrapes.” Blocking spans workflows/recreation. Undo and Settings → Removed & blocked jobs manage search, pagination, restore, unblock and block. Preserve user state and legacy suppression.
- Advanced sections: Source, Matching, Schedule, Retention; YAML is an optional lossless representation. Keep current stack/adapters, no DAG framework.

## Phase 0: baseline and instructions

- [x] Inspect working tree, create branch and recoverable checkpoint.
- [x] Save implementation specification and root instructions.
- [ ] Separate mechanical Rust formatting commit.
- [ ] Correct style guard to inspect production styles, allow rounded-none/shadow-none and ignore comments/test fixtures.

## Phase 1: data protection

- [ ] Remove checksum rewriting. Audit authoritative migration history; subsequent changes are new migrations only.
- [ ] Consistent SQLite backup before existing database upgrades. Reject unknown checksum/schema states with recovery guidance without modifying metadata. Reconcile only explicitly recognized historical states with tested forward repair.
- [ ] Protect tracked jobs, favorites and nonblank notes under every retention policy; migrate legacy all semantics. Normalize time comparisons. Retain removal/block records and report prune counts.
- [ ] Populated upgrade tests preserve notes/history/favorites/tombstones/workflows; mismatches leave metadata/data unchanged. Document recovery.

## Phase 2: identity and extraction

- [ ] Shared Rust PostingIdentity: ATS adapter + board/account + provider ID; generic publisher origin + reliable ID or normalized detail URL. Independent of workflow ID. Remove only known tracking parameters, preserve gh_jid. Keep destination separate; use identity for grouping/suppression without wholesale deduplication rewrite.
- [ ] Preserve provider URLs, resolve relative URLs, accept HTTP(S) only; reject missing per-posting destination rather than use listing page. Validate optional selectors/patterns; fix CLI colon splitting. Known company metadata or neutral icon, never guessed board domains.
- [ ] Greenhouse absolute_url and bounded/cached details for first_published/pay transparency; update time distinct from posting time. Consult official endpoint documentation when implementing.
- [ ] Remove fabricated browser production results; explicit visibly labeled coherent fixture/demo mode. Never call hardcoded IDs live verified.
- [ ] Tests follow ID/title/destination through adapter, persistence, card/details and opener.

## Phase 3: salary and filters

- [ ] Explicit compensation ranges with original min/max, currency, interval, evidence/source; fixed-point cents and separately named derived annual values. Preserve multiple currencies/locations/ranges. Never infer interval by magnitude.
- [ ] Inclusive overlap; missing bound open-ended; entirely unknown excluded unless explicit inclusion; unknown/wrong currency excluded. Reject inverted/malformed input inline; retain cents.
- [ ] Re-extract legacy evidence or reverse known documented annualization and mark derived; uncertain units stay unknown until refresh; backup preserves prior data.
- [ ] Shared Rust predicate model and generated types for worker/repository semantics; distinguish saved-state fields. Parameterize all SQL. Align unknown/new-grad/workplace/text/partial ranges.
- [ ] Repository filtering/sort/totals/pagination replaces 2,000-row assumption. Reset pages on changes; prevent stale responses.
- [ ] Shared vectors test parity and salary boundaries/cents/one-sided/unknown/currency/annual exclusion/mode/reset/persistence; >2,000 records searchable.

## Phase 4: workflow contract and execution

- [ ] Versioned WorkflowSpec with explicit supported-version migration, path-specific unknown-field/version errors. Canonical backend YAML/query validation and execution; invalid queries never match all.
- [ ] Preserve form/YAML and Simple/Advanced hidden values in one draft. Absent inherits; explicit empty overrides. Resolve once and record effective spec per run.
- [ ] Advanced any/all/exclude keywords and title/description/company/location fields; role/workplace/salary/experience/eligibility; posting-date filters; daily/weekly/weekdays/time/timezone; expert query/selectors validated. Clearly label inheritance; expose only honored controls.
- [ ] Separate posted/provider-updated/discovered/last-seen dates. Relative age captured at run start; relative/absolute exclusive. Date-only ranges in workflow timezone, start inclusive/end next midnight exclusive; missing excluded unless opted in.
- [ ] Preview/single/Run all use identical effective rules and identities. Preview reports stage/exclusion counts and samples without side effects.

## Phase 5: scheduling and workers

- [ ] One due dispatcher; global pause covers startup and per-source automatic runs. Manual remains available; manual-only never automatic; Run all uses each enabled workflow's filters.
- [ ] Recalculate next run on creation/schedule/timezone/enabled changes. UTC instants, IANA calendar; numeric comparisons and injected test clock.
- [ ] DST gap → first valid instant; overlap → first occurrence once. Downtime coalesces one catch-up then next future run. One active worker; queue distinct workflows, coalesce duplicate triggers. Remove ineffective concurrency control.
- [ ] Validate required events and terminal completion; nonzero/malformed/timeout/missing completion fails. Preserve useful partial output with incomplete status; kill/reap cancellation/timeout; always release guards and flush buffered jobs.
- [ ] Separate fetched/rejected/blocked/inserted/updated/failed counts, accurate trigger and effective configuration. Actual worker-process tests prove failure/cancellation/recovery.

## Phase 6: removal and blocking

- [ ] Visible upper-right trash SVG, danger token, accessible title/company name, >=32px target.
- [ ] Compact confirmation with title/company, Remove job, unchecked block option; explain reappearance and multiple copies. Atomic soft removal/block across identity; persistence rechecks block for in-flight ingestion.
- [ ] Undo; optimistic rollback and error on failure. Preserve notes/favorites/history/source metadata; unblocked scrape can reactivate.
- [ ] Settings All/Removed/Blocked, search title/company/source, URL/date/status/metadata, paginated useful empty states. Restore clears block atomically; Unblock does not restore; Block future discovery available.
- [ ] Exclude removed from lists/current metrics. Migrate tombstones; retain unresolved keys as honest Legacy posting. Restart/workflow recreation/concurrent ingestion/restoration tests.

## Phase 7: UI and validation

- [ ] Palette distinct from semantic text/muted/surface/border/selection/focus/success/warning/info/danger tokens. Paper danger #A52A43; Obsidian #F08FA2 starting points. Lime background with dark text. Replace scattered status colors.
- [ ] Measured contrast >=4.5 ordinary text, >=3 large text and essential indicators, including hover/focus. Square/flat/thin rules; restrained accents; decoration away from controls/text.
- [ ] Noninteractive cards with separate details/external/favorite/remove controls; no bubbled Enter/Space. Preserve job title casing and text selection. Remove global smoothing suppression. Body >=14px, metadata generally >=12px.
- [ ] Focus visibility, dialog focus/Escape/restoration; active filters/reset/empty states. Distinguish enabled/paused/running.
- [ ] Verify both themes wide/narrow, 200% zoom, keyboard-only and reduced motion.

## Required verification and definition of done

Use deterministic local HTTP → actual worker → NDJSON → repository integration; DOM controls via Testing Library/user-event; browser E2E for workflows/filtering/removal/settings; screenshots both themes/layouts. Native smoke separately verifies startup/IPC/sidecar/opener. Optional live checks timestamped, never required by CI.

CI: Rust fmt, Clippy, workspace tests, generated binding drift, TypeScript, full frontend tests, production build, corrected style/accessibility checks, deterministic integration/browser tests. Build sidecar before dependent checks on clean checkout.

Completion requires every high-priority defect regression-tested, preview/run parity, explicit-hourly matching, source links preserved, manageable reversible cross-workflow blocks, pause/timezone/per-workflow scheduling, migration/retention preservation, visual/keyboard QA, clean-checkout checks, truthful docs, small commits and recovery path. Do not mark complete while any requirement remains unverified.
