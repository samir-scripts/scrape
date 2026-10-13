# `src/components/layout` — Application Shell & Framing Layouts

## Overview

The `src/components/layout` directory provides the structural grid and framing containers for Scrape's desktop interface.

---

## Component Manifest

| Component | File | Responsibilities & Behavior |
|---|---|---|
| **Shell** | [`Shell.tsx`](Shell.tsx) | Root application framing container. Arranges the persistent `TopNav`, dynamic view router container, and persistent `Footer` within a strict full-height (`h-screen overflow-hidden`) flex layout. |
| **TopNav** | [`TopNav.tsx`](TopNav.tsx) | Primary application header. Houses the brutalist `SCRAPE` brand glyph, top-level view tabs (`DISCOVERED`, `TRACKED`, `SCRAPER`, `METRICS`, `SETTINGS`), the in-app notification bell with unread badge, and a real-time system clock. |
| **SubNav** | [`SubNav.tsx`](SubNav.tsx) | Secondary action bar rendered below `TopNav`. Contains search input, filter toggle buttons, view switchers (e.g. Kanban vs. List), and total result counts. |
| **ThreeColumn** | [`ThreeColumn.tsx`](ThreeColumn.tsx) | Classic 3-column desktop layout pane: left collapsible filter sidebar, center job list, and right job detail inspector. |
| **DiscoveredTwoColumn** | [`DiscoveredTwoColumn.tsx`](DiscoveredTwoColumn.tsx) | Optimized 2-column split layout for the Discovered view: left scrollable column of job cards and right sticky consolidated detail panel. |
| **Footer** | [`Footer.tsx`](Footer.tsx) | Bottom status bar displaying current crawl engine status, next scheduled run time, database statistics, and keyboard shortcut legends (`[J/K] NAVIGATE`, `[ENTER] OPEN`, `[DEL] DELETE`). |
