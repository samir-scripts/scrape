# Scraper view

`ScraperView` hosts the workflow registry.
The registry header provides YAML upload, Run all, Add workflow, cancellation while
active, and secondary maintenance. Run all uses each enabled workflow's existing
backend rules and remains available while automatic scheduling is paused.

Each workflow owns its matching, schedule, ingestion limit and retention values.
The registry exposes only the application-wide automatic-scraping pause; workflow
settings stay in the Source → Matching → Schedule → Review editor.

Source/run logs are loaded for the selected workflow/run and refreshed on execution
completion. Error and cancellation messages remain visible; partial results are
not reported as successful runs. Recorded examples disable execution, including
saved and transient Test APIs. Browser fixtures are explicitly labeled.

See [workflow editor ownership](../settings/README.md) and
[field representation](../../../../plans/08-workflow-fields.md).
