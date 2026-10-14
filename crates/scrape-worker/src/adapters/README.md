# `crates/scrape-worker/src/adapters` — ATS & Website Ingestion Adapters

## Overview

The `crates/scrape-worker/src/adapters` directory contains specialized parsers that ingest job listings from diverse web formats and Applicant Tracking Systems (ATS). Each adapter fetches raw data (via REST API endpoints or HTML documents), parses the platform's specific schema, applies the enrichment heuristics from `scrape-core::enrich`, and produces standardized `WorkerEvent::Job` events.

---

## File Manifest

| File | Adapter Target | Data Source & Mechanism | Key Implementation Details |
|---|---|---|---|
| [`mod.rs`](mod.rs) | Adapter Dispatcher | Router | Matches `source.kind` (`Greenhouse`, `Lever`, `Ashby`, `JsonLd`, `Selector`) to its respective asynchronous scraper function. |
| [`greenhouse.rs`](greenhouse.rs) | Greenhouse | JSON REST API | Fetches `https://boards-api.greenhouse.io/v1/boards/{board}/jobs?content=true`. Keeps Remote workplace policy separate from eligible geography; uses office location fields for an explicit hybrid/on-site arrangement and labeled eligible-location sections from normalized descriptions. Fetches provider detail fields and preserves posting provenance. |
| [`lever.rs`](lever.rs) | Lever | JSON REST API | Fetches `https://api.lever.co/v0/postings/{company}`. Parses commitment categories, workplace types, salary ranges, and plain-text descriptions. |
| [`ashby.rs`](ashby.rs) | Ashby | JSON REST API | Fetches `https://api.ashbyhq.com/posting-api/job-board/{board}`. Normalizes compensation tiers, remote/hybrid status, and multi-location lists. |
| [`jsonld.rs`](jsonld.rs) | Schema.org Semantic Web | Embedded HTML `<script>` | Scrapes HTML pages containing `<script type="application/ld+json">` tags. Recursively parses Schema.org `JobPosting` objects, supporting both direct objects and `@graph` array collections. |
| [`selector.rs`](selector.rs) | Custom Web Pages | CSS Selectors + Regex | Configurable engine for arbitrary career websites. Extracts job items, titles, URLs, locations, and salaries using configurable CSS selectors, regex captures, and URL glob filters (`url_pattern`, `exclude_url_patterns`). |

---

## Adapter Data Flow

```
                      WorkerSource Config
                               │
                               ▼
                        [adapters::mod.rs]
                               │
       ┌───────────────┬───────┴───────┬───────────────┬───────────────┐
       ▼               ▼               ▼               ▼               ▼
[greenhouse.rs]   [lever.rs]      [ashby.rs]      [jsonld.rs]     [selector.rs]
 (REST JSON)     (REST JSON)     (REST JSON)     (HTML Script)    (CSS & Regex)
       │               │               │               │               │
       └───────────────┴───────┬───────┴───────────────┴───────────────┘
                               │
                               ▼
               Enrichment Engine (scrape-core::enrich)
               (Salary, Location, Workplace, Eligibility)
                               │
                               ▼
                      WorkerEvent::Job
```
