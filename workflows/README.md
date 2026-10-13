# `workflows/` — Declarative Scrape Workflow Specifications

## Overview

The `workflows/` directory contains sample and user-defined declarative YAML workflow specifications. These specifications enable automated, recurring job board monitoring with pre-configured filters, scheduling, and retention policies.

---

## File Manifest

| File | Target Entity | Key Configurations |
|---|---|---|
| [`software-engineering-internships.yaml`](software-engineering-internships.yaml) | Cloudflare Engineering Internships | Demonstrates Version 2 of the Scrape Workflow Specification: targets Cloudflare's Greenhouse board (`kind: greenhouse`), configures a daily morning schedule at 09:00, filters for internship roles with SQL queries and keyword rules (`include_keywords`, `exclude_keywords`), targets specific cities (San Francisco, Austin, London, Lisbon, Remote), and sets a 45-day retention policy for untracked listings. |

---

## Specification Schema Structure

```yaml
version: 2
metadata:
  name: "Company — Target Role"
  company: "Company Name"
  enabled: true
extractor:
  kind: greenhouse | lever | ashby | jsonld | selector
  url: "https://boards.greenhouse.io/..."
schedule:
  cadence: hourly | six_hours | twelve_hours | daily | manual
  time: "HH:MM"
  cron: "0 0 9 * * *"
filters:
  role_types: [internship, swe, ...]
  query: "SQL query expression"
  include_keywords: [...]
  exclude_keywords: [...]
  locations: [...]
retention:
  days: 45
  policy: untracked_only | all
```
