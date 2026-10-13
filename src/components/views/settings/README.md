# `src/components/views/settings` — Configuration & Preferences Center

## Overview

The `src/components/views/settings` directory contains the configuration screens for customizing Scrape's crawler sources, automation schedules, default filter policies, visual appearance, data retention rules, and diagnostic logs.

---

## File Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **SettingsView** | [`SettingsView.tsx`](SettingsView.tsx) | Top-level settings container organizing settings into discrete tabs (`SOURCES`, `SCHEDULE`, `FILTERS`, `APPEARANCE`, `SYSTEM`, `LOGS`). |
| **SourcesSettings** | [`SourcesSettings.tsx`](SourcesSettings.tsx) | Management interface for crawl targets: toggle enabled states, edit URL identifiers, trigger test crawls, and invoke `AddSourceModal`. |
| **AddSourceModal** | [`AddSourceModal.tsx`](AddSourceModal.tsx) | Modal dialog for creating new sources. Includes ATS auto-detection probing, custom CSS selector and regex configuration forms, and live dry-run validation previews. |
| **ScheduleSettings** | [`ScheduleSettings.tsx`](ScheduleSettings.tsx) | Crawl automation controls. Features standard cadence selectors (`HOURLY`, `DAILY`, `WEEKLY`), custom cron expression inputs with syntax validation, and an interactive 24-hour analog `ClockPicker` for choosing execution times. |
| **FiltersSettings** | [`FiltersSettings.tsx`](FiltersSettings.tsx) | Management of default search constraints, persistent keyword exclusion lists (e.g. "MBA", "Sales"), and pre-filter criteria applied to incoming crawls. |
| **AppearanceSettings** | [`AppearanceSettings.tsx`](AppearanceSettings.tsx) | Visual customization controls: base theme selection (`Paper` light mode vs. `Obsidian` dark mode), custom typography selection (Mono, Sans, Serif, Display), and UI density adjustments. |
| **SystemSettings** | [`SystemSettings.tsx`](SystemSettings.tsx) | Data retention and safety controls: auto-purge age thresholds for untracked listings, "Confirm before deleting jobs" dialog toggle (untoggleable here if previously suppressed via "Don't ask again"), full database JSON backup export, and backup restoration. |
| **LogsSettings** | [`LogsSettings.tsx`](LogsSettings.tsx) | System diagnostics log viewer displaying application-level logs with level filters (`DEBUG`, `INFO`, `WARN`, `ERROR`), search filtering, and log file export. |
