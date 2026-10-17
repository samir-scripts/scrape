# `src/test` — Frontend Vitest Test Suites

## Overview

The `src/test` directory contains the automated test suites for the React frontend, executed using Vitest and React Testing Library. It verifies state stores, filtering logic, UI components, keyboard navigation, and YAML parsing.

---

## Test Manifest

| Test File | Covered Area / Feature |
|---|---|
| [`api.test.ts`](api.test.ts) | IPC client methods, backend error handling, and dev-mode mock data generator. |
| [`app_startup.test.ts`](app_startup.test.ts) | Root component initialization, default view selection, and layout mounting. |
| [`clock_picker.test.ts`](clock_picker.test.ts) | 24-hour visual clock face picker rendering and click/drag interaction math. |
| [`constants.test.ts`](constants.test.ts) | Stage configurations, color definitions, and default filter constants integrity. |
| [`contains_and_hourly_filter.test.ts`](contains_and_hourly_filter.test.ts) | Hourly compensation filtering, currency conversions, and substring query matches. |
| [`cron_clock.test.ts`](cron_clock.test.ts) | Cron string math, schedule conversion, and time serialization. |
| [`custom_scrape_jobs.test.ts`](custom_scrape_jobs.test.ts) | Custom website scraping configs, CSS selector validations, and regex capture tests. |
| [`dev_mode_workflow_execution.test.ts`](dev_mode_workflow_execution.test.ts) | Dev-mode simulation of workflow crawling and streaming event emission. |
| [`discovered_card_view_and_consolidated_window.test.ts`](discovered_card_view_and_consolidated_window.test.ts) | Discovered card view layouts, consolidated job details window, and viewport resizing. |
| [`discovered_panes_dedupe_and_icons.test.ts`](discovered_panes_dedupe_and_icons.test.ts) | Two-column split panes in Discovered view, job deduplication, and Google favicon icons. |
| [`experience_filter.test.ts`](experience_filter.test.ts) | Years of experience range filtering, senior/staff exclusion, and new-grad matching. |
| [`filters.test.ts`](filters.test.ts) | Multi-faceted filtering combinations (role + location + salary + workplace type). |
| [`font_settings.test.ts`](font_settings.test.ts) | User font preset selection (Mono, Sans, Serif) and typography application. |
| [`formatters.test.ts`](formatters.test.ts) | Formatting of currency ranges, relative times (`2h ago`), and percentages. |
| [`hotkeys_engine.test.ts`](hotkeys_engine.test.ts) | Keyboard shortcut dispatcher, text input suppression, and command palette keys. |
| [`job_deletion_and_confirmation.test.ts`](job_deletion_and_confirmation.test.ts) | Job deletion workflow, "Don't ask again" confirmation modal, and tombstone tracking. |
| [`job_url_navigation_and_authenticity.test.ts`](job_url_navigation_and_authenticity.test.ts) | URL opener invocation, authentic posting deep-links, and in-office badge rendering. |
| [`kanban_board.test.ts`](kanban_board.test.ts) | Kanban column layout, stage transition keyboard shortcuts, and drag-and-drop. |
| [`location_data.test.ts`](location_data.test.ts) | Geographic dataset lookup and state/province autocomplete options. |
| [`logs.test.ts`](logs.test.ts) | Telemetry log formatting, level filtering (DEBUG, INFO, WARN, ERROR), and search. |
| [`metrics.test.ts`](metrics.test.ts) | Metrics computations, conversion rates, and analytics card calculations. |
| [`navigation.test.ts`](navigation.test.ts) | Tab switching between Discovered, Tracked, Scraper, Metrics, and Settings. |
| [`notifications.test.ts`](notifications.test.ts) | Notification bell badge count, popover toggle, mark-read, and dismissal. |
| [`range_slider.test.ts`](range_slider.test.ts) | Dual-thumb brutalist range slider interactions, step increments, and value clamping. |
| [`readiness.test.ts`](readiness.test.ts) | Sanity and readiness smoke assertions. |
| [`salary_input_parser.test.ts`](salary_input_parser.test.ts) | Free-form user salary string parsing (`$100k`, `50/hr`). |
| [`sample_workflow.test.ts`](sample_workflow.test.ts) | Parsing and running bundled sample YAML workflows. |
| [`schedule_settings.test.ts`](schedule_settings.test.ts) | Schedule settings form controls, cron validity checks, and cadence pickers. |
| [`scraper_ergonomics.test.ts`](scraper_ergonomics.test.ts) | Scraper view hotkeys, run trigger button states, and keyboard accessibility. |
| [`scraper_tab.test.ts`](scraper_tab.test.ts) | Scraper tab layout, active source monitoring, and run history tables. |
| [`store.test.ts`](store.test.ts) | Zustand store actions, selection mutations, and state resets. |
| [`store_filters.test.ts`](store_filters.test.ts) | Filter state synchronization within the Zustand store. |
| [`telemetry_console_and_sources.test.ts`](telemetry_console_and_sources.test.ts) | Real-time crawl telemetry log viewer and source status badges. |
| [`toast.test.ts`](toast.test.ts) | Toast message dispatch, timeout auto-dismissal, and queue management. |
| [`toggle_switch.test.ts`](toggle_switch.test.ts) | Brutalist toggle switch component checked/unchecked state transitions. |
| [`tooltip.test.ts`](tooltip.test.ts) | Tooltip delay, viewport collision detection, and keyboard focus display. |
| [`tracked_list_sorting.test.ts`](tracked_list_sorting.test.ts) | Tracked table list sorting by company, title, date applied, and status. |
| [`tristate_filter_logic.test.ts`](tristate_filter_logic.test.ts) | TriState logic for visa sponsorship, security clearance, and new grad filters. |
| [`workflow_editor_interaction.test.tsx`](workflow_editor_interaction.test.tsx) | End-to-end `AddSourceModal` and `WorkflowSections` wizard navigation, preview gating, and save behavior. |
| [`workflow_browser_script.test.tsx`](workflow_browser_script.test.tsx) | Custom Extractor controls in `AddSourceModal`: mode switching, keyboard CSS/XPath locators, Test Extractor preview, Browser CDP sessions (`Open`/`Save`/`Clear` and `Needs login`/`Ready` badge transitions), and Script environment diagnostics, truthful disclosure, real `initialSource` edit path, and stale-draft trust gating. |
| [`sources_editor_interaction.test.tsx`](sources_editor_interaction.test.tsx) | Multi-source editor interactions in `WorkflowSections`: adding/removing sources, reordering, duplicate normalized endpoint detection, and independent per-source testing. |
| [`workflow_geography_interaction.test.tsx`](workflow_geography_interaction.test.tsx) | Searchable country/subdivision comboboxes, workplace selection, and unknown-geography opt-in. |
| [`workflow_import_interaction.test.tsx`](workflow_import_interaction.test.tsx) | Shared `WorkflowImportButton` file upload, validation, replacement confirmation, and error recovery. |
| [`keyword_entry_interaction.test.tsx`](keyword_entry_interaction.test.tsx) | `KeywordInput` chip commits, phrase preservation, deduplication, IME composition, and synchronous flush. |
| [`scan_diagnostics_interaction.test.tsx`](scan_diagnostics_interaction.test.tsx) | `ScanDiagnostics` categorized rejection breakdown and overlapping failure examples in preview and run details. |
| [`enrichment_outcomes_interaction.test.tsx`](enrichment_outcomes_interaction.test.tsx) | Optional enrichment warning vs. required-evidence partial outcomes, cancellation, and log filtering. |
| [`company_logo_interaction.test.tsx`](company_logo_interaction.test.tsx) | `CompanyLogo` local IPC asset loading, shared cache deduplication, initials fallback, and refresh retry. |
| [`kanban_movement_interaction.test.tsx`](kanban_movement_interaction.test.tsx), [`job_status_queue.test.tsx`](job_status_queue.test.tsx) | Pointer/keyboard Kanban movement, `Move to` menus, and serialized per-job status mutation queue. |
| [`job_inspector_interaction.test.tsx`](job_inspector_interaction.test.tsx), [`split_inspector_interaction.test.tsx`](split_inspector_interaction.test.tsx), [`resizable_split_primitive.test.tsx`](resizable_split_primitive.test.tsx) | Shared `JobInspector`, adjustable `ResizableSplitPane`, responsive modal fallback, and note persistence. |
| [`job_paginator_interaction.test.tsx`](job_paginator_interaction.test.tsx), [`repository_pagination_interaction.test.tsx`](repository_pagination_interaction.test.tsx) | Bottom `JobPaginator` and SQLite repository page query orchestration. |
| [`sort_in_filters_interaction.test.tsx`](sort_in_filters_interaction.test.tsx), [`table_sort_interaction.test.tsx`](table_sort_interaction.test.tsx), [`persisted_native_filters.test.ts`](persisted_native_filters.test.ts) | Consolidated **Filters & sort** panel, removable sort chip, and persisted native filter state. |
| [`removal_interaction.test.tsx`](removal_interaction.test.tsx), [`data_management_interaction.test.tsx`](data_management_interaction.test.tsx) | Soft removal with optional blocking, Undo, restoration, and confirmed Data Management bulk cleanup/reset. |
| [`select_interaction.test.tsx`](select_interaction.test.tsx), [`action_menu_interaction.test.tsx`](action_menu_interaction.test.tsx), [`icon_controls_interaction.test.tsx`](icon_controls_interaction.test.tsx) | Headless UI–based `Select`, `Combobox`, `ActionMenu`, and accessible `IconButton` primitives. |
| [`schedule_control.test.tsx`](schedule_control.test.tsx), [`scheduler_events.test.ts`](scheduler_events.test.ts), [`run_all_workflows.test.ts`](run_all_workflows.test.ts) | TopNav `ScheduleControl` status/pause popover, scheduler event updates, and Run all orchestration. |
| [`browser_runtime.test.ts`](browser_runtime.test.ts), [`recorded_execution_guard.test.ts`](recorded_execution_guard.test.ts), [`posting_opener.test.ts`](posting_opener.test.ts), [`api_error_boundary.test.ts`](api_error_boundary.test.ts), [`theme_contrast.test.ts`](theme_contrast.test.ts), [`settings_date_format_interaction.test.tsx`](settings_date_format_interaction.test.tsx), [`native_salary_interaction.test.tsx`](native_salary_interaction.test.tsx), [`location_filter_interaction.test.tsx`](location_filter_interaction.test.tsx), [`job_card_interaction.test.tsx`](job_card_interaction.test.tsx) | Runtime mode guards, recorded showcase execution blocks, external URL safety, theme token contrast, date formatting, and compensation/location controls. |
| [`workflow_modal_toast.test.ts`](workflow_modal_toast.test.ts) | Modal interactions during workflow creation and toast notification feedback. |
| [`workflow_statuses_and_log_filtering.test.ts`](workflow_statuses_and_log_filtering.test.ts) | Scrape run status badges and execution log filtering. |
| [`yaml_workflow.test.ts`](yaml_workflow.test.ts) | YAML workflow specification schema validation, error messages, and round-tripping. |

---

## Running Tests

```bash
# Run all Vitest test suites plus the Vite startup smoke check
npm test

# Run Vitest serially if concurrent workers time out under load
npx vitest run --maxWorkers=1
```
