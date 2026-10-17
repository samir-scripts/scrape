# Settings and workflow configuration

Settings provides Appearance, System, Removed & blocked jobs, and Data management.
System contains the application-wide automatic-scraping pause. Scraper provides workflow configuration.

`AddSourceModal` and `WorkflowSections` share one draft across `Sources → Matching → Schedule → Review`, Advanced controls, and
YAML. In `Sources`, users can add, edit, enable/disable, reorder, and remove sources, with independent per-source ATS detection and duplicate normalized endpoint validation. Review runs or refreshes the live preview before Save, with an explicit
Run now action afterward. Advanced settings remain intact and are summarized in Simple
mode. Source configuration and YAML use the canonical desktop v5 validator, with
v1–v4 compatibility conversion at import.

`KeywordInput` keeps unfinished text separate from committed chips. Enter, Add,
comma and newline commit terms; multi-term paste preserves phrases, ignores empty
terms and deduplicates case-insensitively. Empty-input Backspace focuses the last
removal control before deletion. Composition events preserve IME input. Pending
edits invalidate preview, and editor actions synchronously commit all keyword
inputs before reading the draft. Input sessions reset only when replacing or
reopening a workflow, so abandoned text cannot leak across workflows.

`KeywordMatching` exposes Any/All, exclusion and Search in in standard Matching.
Imported mixed Any/All rules keep both groups visible, including temporarily empty
groups. Empty imported fields mean every supported field, as in Rust. At least
one field remains selected during editing. This UI edits Rust `KeywordRules` and
adds no matcher. Matching decisions and run explanations use Rust's shared
`MatchingEvaluator`; `ScanDiagnostics` shows reconciled primary counts and explicitly
overlapping examples in previews and saved run details.

Matching includes Maximum matching jobs per run: default 20, positive whole numbers up to 10,000, preserved through YAML/edit/reopen. Changing any limit draft invalidates preview; invalid text prevents continuing or previewing. Review explains the run/preview caps. `JobLimitInput` and other consumers use constants generated from Rust by `export_bindings_ingestion_limits`. Preview accepts at most 10 (or the lower configured limit) and labels partial coverage. Existing records refreshed by a run count toward the limit; saved jobs are not removed by changing it.

`GeographyMatching` uses the generated Rust catalog for country/subdivision search and the same `PostingRules` draft as YAML and execution. Enter or blur commits a known name/code; Escape restores a search draft. Any input edit invalidates preview and invalid text blocks continuation. Changing country clears incompatible subdivisions; imported multiple-country rules remain visible and intact until explicitly changed. Workplace choices are multi-select in Simple mode. Unknown geography is opt-in. Advanced contains additional location text restrictions and optional raw geography codes. The catalog is complete for US states/DC, Canadian provinces/territories, UK constituent countries and Australian states/territories; other subdivision coverage is explicitly bounded in the catalog README.

`RemovedJobsSettings` searches and paginates removed and blocked postings. Restore
restores saved state and clears suppression atomically; Unblock permits later
rediscovery. Job removal always offers the initially unchecked block option.

Workflow schedules, matching, limits and retention are explicit per workflow.
Explicit empty arrays remove restrictions. Legacy application-level values are
retained only to convert older workflows and imports. See
[workflow refinement plan](../../../../plans/10-ui-workflow-refinement.md).


## Workflow editor

The workflow editor uses `useWorkflowDraft` as its only draft/request controller.
Source → Matching → Schedule → Review is an ordered step navigation, with completed
steps available for backtracking. Hidden section inputs retain their draft state.
Schedule contains collapsed retention controls; Advanced settings and YAML retain
supported fields across navigation. Review shows the complete configured rules,
preview and next run before Save. Saving never runs a scrape.

`WorkflowImportButton` is shared by the registry and editor. It reads a selected
file, parses it through `workflowImport`, and calls the canonical workflow
validator before opening/replacing an unsaved draft. Invalid files leave the prior
draft intact; replacing edited input requires confirmation. The file input resets
so the same file can be selected again. Closed/disabled importers reject stale
validation completions.

The Scraper registry owns Run all, cancellation/progress, upload and Add workflow.
Routine retention runs automatically at startup and every 24 elapsed hours,
including while scraping is paused or a workflow is disabled. Only the automatic-scraping pause is application-wide;
schedule, startup, matching, ingestion limit and retention belong to each workflow.
Recorded examples disable execution and imports;
Test and transient-test APIs cannot manufacture scrape results in recorded mode.

See [workflow refinement](../../../../plans/10-ui-workflow-refinement.md) for controls,
YAML-only expert fields and legacy conversion. Native file-chooser gestures remain an
acceptance gate even though browser file-chooser and native local-worker wizard
checks are automated. Workflow deletion uses a named SVG trash action and reports
persistence failures through the shared accessible toast.


## Explicit bulk cleanup

`DataManagementSettings` previews affected postings, saved records, notes and history
before submitting one generated Rust `CleanupRequest`. Delete all, keep favorites,
keep tracked applications, and reset all use one backend selection rule. A qualifying
copy protects all copies of its posting identity, including soft-removed jobs.
These operations permanently delete records; they do not add discovery blocks.

Cancel receives focus. Escape cancels before execution; buttons and dismissal are
blocked during execution. Reset requires exact `RESET` and always pauses scraping.
Other operations default to pausing but allow leaving the current scheduling state.
Every action requires a successful SQLite recovery snapshot; its path and recovery
instructions are shown after success. See [database recovery](../../../../plans/database-recovery.md).

Browser mode disables these controls and cannot simulate clearing a desktop database.
`applyDataCleanup` is shared by the response and `data://cleaned` event. It clears
stale selection/Undo, resets pagination and query caches, and clears only `scrape:`
frontend preferences on full reset. Errors preserve caches and require fresh counts
before another destructive submission. Preview requests cannot repopulate a closed
dialog. Native checks use temporary profiles and never the user's active database.
The page also reports the last automatic retention outcome. Three consecutive
failures show an actionable warning; routine cleanup records failures and defers
while another coordinated write is active.
