# `src/components/views/discovered` — Discovered Jobs View

## Overview

The `src/components/views/discovered` directory contains the interface for browsing, inspecting, triaging, and acting upon newly crawled job postings before promoting them into the tracked pipeline or deleting them.

---

## File Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **DiscoveredView** | [`DiscoveredView.tsx`](DiscoveredView.tsx) | Top-level controller for the Discovered tab. Coordinates card vs. table layouts inside `ResizableSplitPane`, manages active job selection, marks unread postings as read on detail inspection, and binds keyboard navigation (`j/k` selection, `Enter` to open URL, `Shift+A` to track, `Delete` to remove). |
| **JobCard** | [`JobCard.tsx`](JobCard.tsx) | Brutalist job card rendering 24px `CompanyLogo`, circular 6px unread SVG indicator, job title, location/workplace tags (`IN-OFFICE`, `REMOTE`, `HYBRID`), native salary, date posted, `FavoriteButton`, and named `RemoveJobButton`. |
| **DiscoveredTable** | [`DiscoveredTable.tsx`](DiscoveredTable.tsx) | High-density tabular view displaying discovered jobs in rows with 24px `CompanyLogo`, unread marker, title, workplace type, native compensation, and action buttons. |
| **JobDetail** | [`JobDetail.tsx`](JobDetail.tsx) | Thin wrapper delegating to the shared `JobInspectorPanel` (`src/components/jobs/JobInspector.tsx`) so discovered and tracked jobs share one consistent detail hierarchy (32px `CompanyLogo`, Open posting, stage selector/Mark applied, favorite/remove/close SVGs, structured `JobDescriptionView`, Notes, and collapsible More details / Source evidence / Activity history). |
| **JobMetaPanel** | [`JobMetaPanel.tsx`](JobMetaPanel.tsx) | Legacy metadata panel component retained for compatibility tests; production views use the shared `JobInspector`. |
