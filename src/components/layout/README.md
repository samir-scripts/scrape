# `src/components/layout` — Application Shell & Framing Layouts

## Overview

The `src/components/layout` directory provides the structural grid and framing containers for Scrape's desktop interface.

---

## Component Manifest

| Component | File | Responsibilities & Behavior |
|---|---|---|
| **Shell** | [`Shell.tsx`](Shell.tsx) | Root application framing container. Arranges the persistent `TopNav`, dynamic view router container, and persistent `Footer` within a strict full-height (`h-screen overflow-hidden`) flex layout. |
| **TopNav** | [`TopNav.tsx`](TopNav.tsx) | Primary application header. Houses the brutalist `SCRAPE` brand glyph, top-level view tabs (`DISCOVERED`, `TRACKED`, `SCRAPER`, `METRICS`, `SETTINGS`), the in-app notification bell with unread badge, and `ScheduleControl`. |
| **ScheduleControl** | [`ScheduleControl.tsx`](ScheduleControl.tsx) | Truthful scheduling status button and popover in `TopNav`. Displays active run state, global pause state, or next scheduled workflow instant, with direct pause/resume and navigation to Scraper workflows. |
| **SubNav** | [`SubNav.tsx`](SubNav.tsx) | Secondary contextual toolbar rendered below `TopNav`. Contains search input, the **Filters & sort** panel trigger, optional table column picker, and view switchers (e.g. Cards vs. Table, Kanban vs. List). Pagination belongs at the bottom of jobs views. |
| **ResizableSplitPane** | [`ResizableSplitPane.tsx`](ResizableSplitPane.tsx) | Shared adjustable results/inspector split layout with pointer capture, keyboard resizing, persisted ratio (`scrape:job_split:v1`), and responsive modal fallback below 812px. |
| **ThreeColumn** | [`ThreeColumn.tsx`](ThreeColumn.tsx) | 3-column desktop layout pane container. |
| **DiscoveredTwoColumn** | [`DiscoveredTwoColumn.tsx`](DiscoveredTwoColumn.tsx) | 2-column split layout container. |
| **Footer** | [`Footer.tsx`](Footer.tsx) | Bottom status bar displaying current crawl engine status, next scheduled run time, database statistics, and keyboard shortcut legends (`[J/K] NAVIGATE`, `[ENTER] OPEN`, and search). |

## Adjustable job inspector

`ResizableSplitPane` is shared by Discovered and both Tracked presentations.
It measures its container, reserves minimum results/details widths, and falls back
to a modal inspector below 812 CSS pixels. Its separator supports pointer capture,
Arrow keys, Home/End, cancellation, and a reset action. Completed changes persist
one versioned ratio at `scrape:job_split:v1`; invalid storage uses the default.
Resizing preserves one inspector instance, including pending notes and scroll.

`App` owns repository queries and page selection; `JobPaginator` only emits page
requests. Page/filter/sort changes close the inspector and hand pending notes to
the existing save queue. A failed save offers a retry for the original posting.
