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
