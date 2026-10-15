# `src/components/layout` — Application Shell & Framing Layouts

## Overview

The `src/components/layout` directory provides the structural grid and framing containers for Scrape's desktop interface.

---

## Component Manifest

| Component | File | Responsibilities & Behavior |
|---|---|---|
| **Shell** | [`Shell.tsx`](Shell.tsx) | Root application framing container. Arranges the persistent `TopNav`, dynamic view router container, and persistent `Footer` within a strict full-height (`h-screen overflow-hidden`) flex layout. |
| **TopNav** | [`TopNav.tsx`](TopNav.tsx) | Primary application header. Houses the brutalist `SCRAPE` brand glyph, top-level view tabs (`DISCOVERED`, `TRACKED`, `SCRAPER`, `METRICS`, `SETTINGS`), the in-app notification bell with unread badge, and a real-time system clock. |
| **SubNav** | [`SubNav.tsx`](SubNav.tsx) | Secondary action bar rendered below `TopNav`. Contains search input, filter toggle buttons, view switchers (e.g. Kanban vs. List), and sort selection. Pagination belongs at the bottom of jobs views. |
| **ThreeColumn** | [`ThreeColumn.tsx`](ThreeColumn.tsx) | Classic 3-column desktop layout pane: left collapsible filter sidebar, center job list, and right job detail inspector. |
| **DiscoveredTwoColumn** | [`DiscoveredTwoColumn.tsx`](DiscoveredTwoColumn.tsx) | Optimized 2-column split layout for the Discovered view: left scrollable column of job cards and right sticky consolidated detail panel. |
| **Footer** | [`Footer.tsx`](Footer.tsx) | Bottom status bar displaying current crawl engine status, next scheduled run time, database statistics, and keyboard shortcut legends (`[J/K] NAVIGATE`, `[ENTER] OPEN`, `[DEL] DELETE`). |

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
