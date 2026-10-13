# `src/components/views/scraper` — Crawler Control Room & Telemetry

## Overview

The `src/components/views/scraper` directory contains the control center for configuring, testing, manually executing, and monitoring job board crawl passes.

---

## File Manifest

| Component | File | Responsibilities & UI Features |
|---|---|---|
| **ScraperView** | [`ScraperView.tsx`](ScraperView.tsx) | Complete crawler control center. Features: <br>• **Source Inventory**: Lists all configured scraping targets with platform badges, cadence tags, last run timestamps, and direct `RUN` / `TEST` / `EDIT` buttons.<br>• **Global Controls**: One-click `[RUN ALL NOW]` trigger and emergency stop controls.<br>• **Active Execution Telemetry**: Live progress card showing the running sidecar process, current target source, duration, and jobs found vs. filtered.<br>• **Live Log Stream**: Embedded terminal console displaying real-time NDJSON logs emitted by `scrape-worker`, with severity level filters (`ALL`, `DEBUG`, `INFO`, `WARN`, `ERROR`) and text search.<br>• **Run History**: Table of historical crawl executions with status badges, run durations, discovered job counts, and error inspector drawers. |
