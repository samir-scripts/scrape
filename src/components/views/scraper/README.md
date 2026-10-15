# Scraper view

`ScraperView` hosts the workflow registry and an explicit Scraper settings dialog.
The registry header provides YAML upload, Run all, Add workflow, cancellation while
active, and secondary maintenance. Run all uses each enabled workflow's existing
backend rules and remains available while automatic scheduling is paused.

Global schedule and matching defaults live in Scraper settings. Opening the dialog
performs reads; changing a control is an explicit global edit. Per-workflow values
and inheritance stay in the Source → Matching → Schedule → Review wizard.

Source/run logs are loaded for the selected workflow/run and refreshed on execution
completion. Error and cancellation messages remain visible; partial results are
not reported as successful runs. Recorded examples disable execution, including
saved and transient Test APIs. Browser fixtures are explicitly labeled.

See [workflow editor ownership](../settings/README.md) and
[field representation](../../../../plans/08-workflow-fields.md).
