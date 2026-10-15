# `src/components/filters` — Multi-Faceted Job Filter Controls

## Overview

The `src/components/filters` directory contains matching filters and persisted query sorting. The toolbar opens a single **Filters & sort** panel with sorting above match criteria. These controls feed the repository query in `src/lib/store.ts` and preserve page, selection, and pending-note behavior.

---

## Component Manifest

| Component | File | Responsibilities & Controls |
|---|---|---|
| **FilterPanel** | [`FilterPanel.tsx`](FilterPanel.tsx) | Popover with persisted sort at the top, followed by keyword search, role types, locations, compensation sliders, experience bounds, workplace arrangements, and eligibility constraints. |
| **FilterChips** | [`FilterChips.tsx`](FilterChips.tsx) | Horizontal summary strip for active matching criteria and a removable nondefault sort chip. Clearing criteria preserves sort; resetting sort returns to the view's contextual default. |
| **RoleTypeFilter** | [`RoleTypeFilter.tsx`](RoleTypeFilter.tsx) | Multi-select toggle group for filtering jobs by role category (`SWE`, `DATA`, `PRODUCT`, `DESIGN`, `SEASONAL`, `OTHER`). |
| **LocationFilter** | [`LocationFilter.tsx`](LocationFilter.tsx) | Location input with autocomplete dropdown powered by `src/lib/locationData.ts`. Supports multi-city selection and remote-only toggling. |
| **SalaryRangeFilter** | [`SalaryRangeFilter.tsx`](SalaryRangeFilter.tsx) | Compensation filter featuring a dual-thumb slider, manual min/max number inputs, interval selector (`YEARLY` vs `HOURLY`), and currency filters. |
| **ExperienceFilter** | [`ExperienceFilter.tsx`](ExperienceFilter.tsx) | Years of experience slider (0 to 15+ years) and dedicated `NEW GRAD ONLY` toggle. |
| **TriStateFilter** | [`TriStateFilter.tsx`](TriStateFilter.tsx) | Three-state toggle group (`ANY`, `YES`, `NO`) used for visa sponsorship (`WILL SPONSOR`), security clearance (`REQUIRES CLEARANCE`), and U.S. work authorization (`US CITIZEN ONLY`). |
