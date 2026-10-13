# `src/lib` — Core Frontend Business Logic & Libraries

## Overview

The `src/lib` directory contains the client-side domain logic, state management, IPC client, math algorithms, and parsing utilities that power the Scrape frontend.

---

## File Manifest

| File | Purpose & Key Exports |
|---|---|
| [`api.ts`](api.ts) | **Tauri IPC Client**: Strongly-typed wrappers for all backend commands (`getJobs`, `updateJobStatus`, `deleteJob`, `startScrape`, `getSources`, etc.). Includes a dev-mode mock generator that produces realistic, authentic seed jobs (e.g. Cloudflare internships) when running outside the Tauri runtime. |
| [`store.ts`](store.ts) | **Global Zustand Store**: Central reactive state container (`useStore`). Manages active navigation tab, selected job ID, search queries, active filter sets, toast notifications, confirmation dialog states, and user preferences. |
| [`filter.ts`](filter.ts) | **Client-Side Filtering & Sorting**: Core algorithms (`filterJobs`, `sortJobs`) evaluating full-text search, role types, salary ranges, location matches, workplace arrangement (`onsite`, `hybrid`, `remote`), visa sponsorship, security clearance, work authorization, and new grad status. |
| [`constants.ts`](constants.ts) | **Application Constants**: Status display metadata (`STATUS_CONFIG`), tracked pipeline stages (`TRACKED_STAGES`), role color mapping (`ROLE_CONFIG`), theme variables, and default filter initializers. |
| [`cronClock.ts`](cronClock.ts) | **Cron & Clock Math**: Math utilities converting 24-hour clock face coordinates (angles, radii) into standard cron strings and human-readable time strings (`HH:MM`). |
| [`events.ts`](events.ts) | **Backend Event Subscriptions**: Strongly-typed listeners wrapping `@tauri-apps/api/event` (`listenToScrapeEvents`, `listenToNotifications`). |
| [`formatters.ts`](formatters.ts) | **Display Formatters**: Functions formatting currency numbers (`$150K`), salary intervals (`/yr`, `/hr`), dates, relative timestamps (`3m ago`, `2d ago`), and metrics percentages. |
| [`kanbanNavigation.ts`](kanbanNavigation.ts) | **Spatial Kanban Navigation**: Algorithms for keyboard navigation across columns and cards. Handles stage transitions (`Shift+Left`, `Shift+Right`) and card selection (`Up`, `Down`, `j`, `k`). |
| [`locationData.ts`](locationData.ts) | **Geographic Datasets**: Static database of North American states, Canadian provinces, and global tech hubs used for auto-completing location filter chips. |
| [`positioning.ts`](positioning.ts) | **Popover Positioning Engine**: Calculates geometric viewport coordinates for tooltips, context menus, and notification popovers to prevent screen edge clipping. |
| [`salary.ts`](salary.ts) | **Salary Input Parser**: Parses user-typed salary query expressions (e.g. `"$120k"`, `"60/hr"`) into normalized numeric values for the range slider filters. |
| [`status.ts`](status.ts) | **Status Workflow Logic**: Helper functions governing application pipeline stage transitions and tracking statuses. |
| [`toast.ts`](toast.ts) | **Toast Dispatch API**: Lightweight imperatively-callable notification dispatcher (`toast.success()`, `toast.error()`, `toast.info()`). |
| [`yamlJob.ts`](yamlJob.ts) | **YAML Job Bridge**: Transforms job listings to and from YAML workflow representations. |
| [`yamlWorkflow.ts`](yamlWorkflow.ts) | **Workflow Specification Parser**: Parses, validates, and serializes Version 2 declarative YAML workflow specifications using the `yaml` library. |
