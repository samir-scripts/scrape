# Scraper view

`ScraperView` hosts the workflow registry.
The registry header order is **Add workflow** (primary, with plus icon), **Run all**, and **Import YAML**, with active run progress and **Cancel run** in the same action area. Each workflow row provides **Run** and a **More actions** menu (**Edit workflow**, **Preview matches**, **View run history**, divider, **Delete workflow**). Clicking a workflow name opens configuration; clicking a failed status opens run details. Run all uses each enabled workflow's explicit `WorkflowSpec` rules and per-workflow job budget, and remains available while automatic scheduling is paused.

Each workflow owns its matching, schedule, ingestion limit (`max_jobs_per_run`) and retention values.
The registry exposes only the application-wide automatic-scraping pause; workflow
settings stay in the `Source → Matching → Schedule → Review` editor.

Source/run logs and `ScanDiagnostics` summaries are loaded for the selected workflow/run and refreshed on execution completion. Run and source outcomes distinguish `success`, `success_with_warnings`, `partial`, `failed`, and `cancelled`; error filters exclude explicit cancellation and optional enrichment warnings. Recorded examples disable execution, including saved and transient Test APIs. Browser fixtures are explicitly labeled.

See [workflow editor ownership](../settings/README.md) and
[field representation](../../../../plans/08-workflow-fields.md).
