# `src/components/views/discovered` — Discovered Jobs View

## Overview

The `src/components/views/discovered` directory contains the interface for browsing, inspecting, triaging, and acting upon newly crawled job postings before promoting them into the tracked pipeline or deleting them.

---

## File Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **DiscoveredView** | [`DiscoveredView.tsx`](DiscoveredView.tsx) | Top-level controller for the Discovered tab. Coordinates card vs. table layouts, manages active job selection, binds keyboard navigation (`j/k` selection, `Enter` to open URL, `Shift+A` to track, `Delete` to remove), and hosts the confirmation modal. |
| **JobCard** | [`JobCard.tsx`](JobCard.tsx) | Brutalist job card rendering company logo (`CompanyLogo`), job title, location, salary, authentic workplace arrangement tags (`IN-OFFICE`, `REMOTE`, `HYBRID`), date posted, optimistic favorite toggle (`FavoriteButton`), and quick delete trigger. |
| **DiscoveredTable** | [`DiscoveredTable.tsx`](DiscoveredTable.tsx) | Alternative high-density tabular view displaying discovered jobs in sortable rows with company favicon, title, workplace type, and action buttons. |
| **JobDetail** | [`JobDetail.tsx`](JobDetail.tsx) | Consolidated inspection panel displayed in the right pane. Renders the full job description (plain text or formatted), authentic deep-link navigation button (`APPLY / OPEN URL`), `TRACK APPLICATION` stage promoter, and `DELETE` button. |
| **JobMetaPanel** | [`JobMetaPanel.tsx`](JobMetaPanel.tsx) | Structured metadata sidebar grid displaying compensation tiers, geographic offices, visa sponsorship status, security clearance requirements, work authorization rules, and posting timestamps. |
