# Settings and workflow configuration

Settings provides Appearance, System, Removed & blocked jobs, and Data management. Scraper
provides source workflows, automatic scheduling and global matching defaults.

`AddSourceModal` and `WorkflowSections` share one draft across Simple, Advanced and
YAML. Simple creation is URL → actual preview → schedule/save, with an explicit
Run now afterward. Advanced settings remain intact and are summarized in Simple
mode. Source configuration and YAML use the canonical desktop v3 validator.

Matching includes Maximum matching jobs per run: default 20, positive whole numbers up to 10,000, preserved through YAML/edit/reopen. Changing any limit draft invalidates preview; invalid text prevents continuing or previewing. Review explains the run/preview caps. `JobLimitInput` and other consumers use constants generated from Rust by `export_bindings_ingestion_limits`. Preview accepts at most 10 (or the lower configured limit) and labels partial coverage. Existing records refreshed by a run count toward the limit; saved jobs are not removed by changing it.

`GeographyMatching` uses the generated Rust catalog for country/subdivision search and the same `PostingRules` draft as YAML and execution. Enter or blur commits a known name/code; Escape restores a search draft. Any input edit invalidates preview and invalid text blocks continuation. Changing country clears incompatible subdivisions; imported multiple-country rules remain visible and intact until explicitly changed. Workplace choices are multi-select in Simple mode. Unknown geography is opt-in. Advanced contains additional location text restrictions and optional raw geography codes. The catalog is complete for US states/DC, Canadian provinces/territories, UK constituent countries and Australian states/territories; other subdivision coverage is explicitly bounded in the catalog README.

`RemovedJobsSettings` searches and paginates removed and blocked postings. Restore
restores saved state and clears suppression atomically; Unblock permits later
rediscovery. Job removal always offers the initially unchecked block option.

`ScheduleSettings` controls automatic pause and global calendar expressions;
per-workflow schedules expose time, weekdays and IANA timezone. `FiltersSettings`
defines inherited defaults. Explicit empty workflow arrays remove restrictions.
See [workflow specification](../../../../plans/workflow-v3.md).


## Workflow editor and Scraper defaults

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

The Scraper registry owns Run all, cancellation/progress, upload, Add workflow and
secondary maintenance. Truly global settings live in the explicit Scraper settings
dialog. Per-workflow schedule/matching values remain in the wizard. Opening a view
or dialog does not write defaults. Recorded examples disable execution and imports;
Test and transient-test APIs cannot manufacture scrape results in recorded mode.

See [workflow field ownership](../../../../plans/08-workflow-fields.md) for controls,
YAML-only expert fields and inheritance. Native file-chooser gestures remain an
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
