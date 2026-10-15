# UI refinement and independent workflow settings

Status: proposed implementation checklist. Implement one numbered section at a time, with observable tests, reviewable commits, and evidence. This plan supersedes conflicting global-workflow-settings and Scraper Settings decisions in plans 06–08. Domain safety in [Trustworthy scraping](06-trustworthy-scraping.md), the data contracts in [Bounded ingestion and data management](09-bounded-ingestion-and-data-management.md), and the visual references in [NieR](assets/ref-nier-menu.png), [dither](assets/ref-dither.png), and [palette](assets/ref-palette.png) remain authoritative.

## Product decisions

- Keep a restrained unread indicator. Every workflow owns matching, schedule, ingestion limit, and retention. Remove the separate Scraper Settings interface; keep application-wide automatic-scraping pause.
- Run retention automatically while paused and while workflows are disabled. Put sorting inside **Filters & sort**.
- New workflows use explicit defaults: 20 matching jobs per run, daily at 09:00 in the device timezone, unrestricted matching, 30-day safe retention, and startup execution off.
- Use shared themed accessible selection/action primitives, semantic appearance tokens, and the existing Rust domain services and serialized status queue.

## 0. Reproducible starting point

Inspect branch, changes, tests, prior evidence, every app-owned selection/action control, build size/free space, and active build/dev processes. Checkpoint existing edits before revising them. Save this plan and update plan links/supersession status. Capture Paper and Obsidian baseline screens, including open menus, populated board, and reported clipping, only from committed source. Run current interaction tests and record baseline failures. Do not change application data.

## 1. Menus, unread marker, and date controls

Add `@headlessui/react@2.2.10`. Build shared themed Listbox, searchable Combobox, and ActionMenu wrappers. Replace app-owned native selects/inconsistent menus; support dialog top-layer ownership, unclipped popups, nested Escape, outside clicks, filter-panel persistence, focus restoration, and keyboard/typeahead behavior. Share square surfaces, thin borders, interface font, spacing, themed scrolling, SVG indicators, and focus tokens. Use a fixed title gutter with a 6px circular unread SVG marker, darker lavender in Paper and lighter lavender in Obsidian. Show only for unread Discovered jobs; opening details marks read, other card actions do not. Replace five date buttons with a shared select whose options show formatter-generated examples; retain 12/24-hour choices as a labeled radio group and stack fields at narrow widths.

Verify keyboard selection/typeahead/disabled options, nested popups, date persistence and legibility at narrow widths/200% zoom, and unread transitions. Inspect open menus in both themes.

## 2. Independent workflow configuration

Introduce WorkflowSpec v4 with explicit effective configuration. Convert v1–3 imports through one Rust compatibility resolver; v4 has no global inheritance. Transactionally materialize each existing workflow's effective matching/calendar configuration, preserve overrides, explicit empty sets, limits, retention and startup semantics, and preserve overdue catch-up and global pause. Use the consistent backup path and a forward migration. Invalid originals remain preserved and marked Needs review, blocking automatic execution without broadening matches or aborting startup. Legacy imports resolve once and review shows the effective values. Remove active global matching/calendar settings and the Scraper Settings UI. Keep pause, display preferences and process safety at app scope. Preview, manual, scheduled and Run all consume the same explicit workflow spec. Generate TS contracts from Rust.

Test populated historical upgrades, idempotence, restart/state preservation, imports/YAML/form round trips and consistency across execution routes. Editing one workflow must not change another.

## 3. Workflow registry and actions

Header order: **Add workflow**, **Run all**, **Import YAML** icon. Add is primary with one plus icon; import stays visible and named. Put progress and Cancel run in the same action area. Remove maintenance gear/manual expiry cleanup. Each Actions cell has Run and More actions. Menu order: Edit workflow, Preview matches, View run history, divider, Delete workflow. Preserve deletion confirmation/errors. Workflow names open configuration; failed status opens run details. Simplify source/cadence/next-run/timezone/retention/status metadata and allow header wrapping. Verify all actions, cancellation/failure/deletion/import behavior, and empty/single/multiple registries in both themes.

## 4. Automatic, observable retention

One retention service and eligibility predicate serve post-run and housekeeping paths. Run after startup when writes are idle, then every 24 elapsed hours, with one catch-up after sleep/restart; continue during pause and for disabled/manual workflows. Use non-cancelling write-coordinator acquisition and defer while busy. Capture one clock instant per sweep. Preserve tracked jobs, favorites, nonblank notes, removed records and suppression; zero days means forever. Record counts/failures, expose last outcome in Data management, report repeated failure, and invalidate lists, totals, pages, metrics and selection through the shared path. Keep explicit bulk deletion/reset separate.

Test temporary populated databases and fake clocks for boundaries, downtime, pause/disabled workflows, busy deferral, races/errors, protections, and UI refresh. Never use live storage.

## 5. Kanban colors and movement

Use shared stage tokens: Applied lavender, Interviewing warm cream/gold, Offer lime, Rejected cherry. Apply restrained header/card accents and reuse them in tags/lists/inspector. Extract common card presentation for real and drag-preview cards. After drag threshold, show a noninteractive pointer-following preview preserving offset, width and content; keep original mounted and show same-size placeholder. Highlight destination with label. Show pending placement during persistence; settle on success or return on failure with existing retry feedback. Respect reduced motion, keyboard movement, Move to, scrolling, focus restoration and cancellation. Keep repository ordering; no manual ranking.

Test real pointer coordinates, populated/empty destinations, scrolling, delayed save/failure/cancel/rapid moves, one history mutation, reduced motion and restart persistence. Synthetic browser events do not count as native pointer acceptance.

## 6. Sorting inside filters

Remove toolbar Sort; rename panel entry **Filters & sort**. Put sort above matching filters. Preserve repository sort, preferences and contextual defaults. Exclude sort from filter counts/matching. Clear filters preserves sort; Reset sort restores view default. Reset pagination on sort changes and use existing pending-note/selection lifecycle. Show a removable summary chip for nondefault sort. Verify ordering, persistence, page reset, match totals, independent clearing/reset, stale responses and note/search behavior.

## Verification and storage

For every section, record problem, implementation, actual commands/results, screenshots, limitations and commit ID. Final checks include Rust format/Clippy/workspace tests/binding drift; TS/full frontend tests/build/style guard; both-theme narrow/wide/zoom/keyboard/reduced-motion browser journeys; isolated native workflow/schedule/retention/persistence; and actual native pointer, dropdown, file chooser and visual QA. Keep screenshots aligned with committed source.

Resolve Cargo target directory via metadata. Make sidecar/test launchers respect `CARGO_TARGET_DIR`. Reuse one target per validation section, do not parallelize builds against it, record size/free space, use development-only reduced debug info and disabled test incremental compilation, prebuild before native runtime timers, preserve logs outside cleaned targets, and stop only task-owned processes. At section completion run `cargo clean --dry-run --target-dir <resolved>` then `cargo clean --target-dir <resolved>` only against an idle intended directory; measure reclaimed space. Never clean an active user target. Report remaining old targets and rebuild checks truthfully.
