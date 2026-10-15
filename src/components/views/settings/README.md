# Settings and workflow configuration

Settings provides Appearance, System, Logs, and Removed & blocked jobs. Scraper
provides source workflows, automatic scheduling and global matching defaults.

`AddSourceModal` and `WorkflowSections` share one draft across Simple, Advanced and
YAML. Simple creation is URL → actual preview → schedule/save, with an explicit
Run now afterward. Advanced settings remain intact and are summarized in Simple
mode. Source configuration and YAML use the canonical desktop v3 validator.

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
checks are automated.
