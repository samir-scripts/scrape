# `src/components/views/metrics` — Analytics Dashboard & Visualizations

## Overview

The `src/components/views/metrics` directory implements the metrics and data visualization dashboard. Powered by the `@visx` charting library, it visualizes job hunting velocity, funnel conversion rates, role distributions, and compensation ranges.

---

## File Manifest

| Component | File | Responsibilities & Visualizations |
|---|---|---|
| **MetricsView** | [`MetricsView.tsx`](MetricsView.tsx) | Analytics dashboard layout. Fetches metrics from `src-tauri` via TanStack Query, rendering KPI cards (Total Discovered, Active Applications, Offers Received, Conversion Rate) alongside charts. |
| **MetricCard** | [`MetricCard.tsx`](MetricCard.tsx) | Brutalist KPI widget displaying metric label, bold numeric value, percentage changes, and trend arrows. |
| **ApplicationsChart** | [`ApplicationsChart.tsx`](ApplicationsChart.tsx) | Interactive time-series area/bar chart built with `@visx/shape` and `@visx/scale`. Visualizes application velocity (submissions per day, week, or month) over customizable time ranges. |
| **RoleTypeDonut** | [`RoleTypeDonut.tsx`](RoleTypeDonut.tsx) | Pie/donut visualization built with `@visx/pie`. Breaks down application volumes by role specialization (Software Engineering, Data Science, Product, Design, Seasonal). |
| **SalaryDistributionBar** | [`SalaryDistributionBar.tsx`](SalaryDistributionBar.tsx) | Histogram chart built with `@visx/shape`. Depicts compensation distributions across salary buckets, highlighting median and quartile values. |
