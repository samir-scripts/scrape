# `workflows/` — Declarative Scrape Workflow Specifications

## Overview

The `workflows/` directory contains sample and user-defined declarative YAML workflow specifications. These specifications enable automated, recurring job board monitoring with pre-configured filters, scheduling, and retention policies.

---

## File Manifest

| File | Target Entity | Key Configurations |
|---|---|---|
| [`software-engineering-internships.yaml`](software-engineering-internships.yaml) | Cloudflare Engineering Internships | Bundled sample YAML workflow (written in legacy v2 format to exercise automatic v1–v3 → v4 compatibility conversion through `src-tauri/src/services/workflow.rs`). Targets Cloudflare's Greenhouse board, configures a daily 09:00 schedule, filters for internship roles and keywords, and sets a 45-day untracked-only retention policy. |

---

## Specification Schema Structure (`WorkflowSpec` v4)

New and exported workflows use `version: 4` (with legacy `version: 1..3` files converted automatically on import by the authoritative Rust resolver in `src-tauri/src/services/workflow.rs`):

```yaml
version: 4
max_jobs_per_run: 20
metadata:
  name: "Company — Target Role"
  company: "Company Name"
  enabled: true
extractor:
  kind: greenhouse | lever | ashby | jsonld | selector
  url: "https://boards.greenhouse.io/..."
schedule:
  cadence: daily | manual
  time: "09:00"
  timezone: "America/New_York"
  weekdays: [1, 2, 3, 4, 5]
  run_on_startup: false
filters:
  matching:
    keywords:
      any: ["Software Engineer"]
      all: []
      exclude: ["Senior", "Staff"]
      fields: [title]
    workplace_types: [remote, hybrid, onsite]
    countries: ["US"]
    regions: ["US-CA", "US-NY"]
retention:
  days: 30
  policy: untracked_only
```
