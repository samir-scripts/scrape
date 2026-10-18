# Plan 11 — Section 7 Execution Blueprint: Provider & Company Verification

**Acceptance correction, 2026-10-18:** The “Fully Supported” classifications below are historical and overstate coverage of the seeded workflows. [Incident 28](errors/28-empty-seeded-workflows.md) reproduces filtered zero results, missing source ownership, malformed Google URLs, and first-page-only coverage. [Plan 12](12-workflow-ingestion-repair.md) defines the required repair and full workflow verification. Meta remains an open acceptance gap.

**Target Executor:** Gemini 3.8 Flash / GPT-6.1 Sol  
**Parent Specification:** [Reliable ingestion, multi-source workflows, and UI repair](11-reliable-ingestion-and-ui-repair.md#7-providercompany-verification)  
**Prerequisite State:** Sections 1–6 (Phases 6A–6F) implemented, tested, and verified on branch `implementation/trustworthy-scraping`.

**Implementation Status:** In progress. Section 7 provides manifest-driven recorded HTTP replay and separately labelled live verification for all required companies: Meta, Apple, Amazon, Netflix, Google, Cloudflare, SEP, Linear, Bitwarden, and authored local fixtures.

---

## 1. Executive Summary & Verification Rules

Section 7 validates the extraction pipeline against real-world career sites across the agreed company coverage matrix.

### Verification Guardrails:
1. **Zero Live Network Calls in CI / Default Tests:** Default verification (`npm test`, `cargo test`, `npm run test:providers`) runs entirely offline against recorded deterministic fixtures served by a local loopback HTTP server with `--replay-origin`. Live network calls are strictly opt-in via `--live` and reported in a separate evidence category.
2. **Authentic Provenance & Checksums:** Every recorded provider response is an authentic, unmodified capture stored under `fixtures/providers/` and registered in `fixtures/providers/manifest.json` with its exact capture URL, ISO 8601 UTC timestamp, SHA-256 checksum, extraction version, and adapter classification.
3. **No Fabricated Production Data:** Unknowns stay unknown. When an endpoint or site uses bot-shielded client-side rendering or blocks automated crawls (such as Meta's dynamic Relay SPA), it is tested, proven, and truthfully documented as an open acceptance gap rather than bypassed with invented mock data.
4. **End-to-End Persistence Boundary:** Verification is not complete with regex or parser tests alone. Every supported company format must be proven through the packaged Rust sidecar binary (`scrape-worker`) into a temporary SQLite database (`JobsRepo::upsert_scraped`), asserting canonical identity (`PostingIdentity`), deduplication, and title/URL fidelity.

---

## 2. Company Coverage & Classification Matrix

| Company | Careers URL / Endpoint | Platform / Extractor Kind | Extraction Mode / Config | Status & Acceptance Classification |
|---|---|---|---|---|
| **SEP** | `https://api.lever.co/v0/postings/sep?mode=json` | Lever REST API | Native `lever` adapter | **Fully Supported**: Native ATS. Proven in recorded HTTP replay and SQLite persistence. |
| **Linear** | `https://api.ashbyhq.com/posting-api/job-board/linear?includeCompensation=true` | Ashby REST API | Native `ashby` adapter | **Fully Supported**: Native ATS. Proven in recorded HTTP replay and SQLite persistence. |
| **Bitwarden** | `https://boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true` | Greenhouse REST API | Native `greenhouse` adapter | **Fully Supported**: Native ATS. Captured 39 authentic jobs. Proven in recorded HTTP replay and SQLite persistence. |
| **Cloudflare** | `https://boards-api.greenhouse.io/v1/boards/cloudflare/jobs?content=true` | Greenhouse REST API | Native `greenhouse` adapter | **Fully Supported**: Native ATS. Authoritative captured projection (`regressions/cloudflare-8024849.json`) and live Greenhouse board. |
| **Google** | `https://www.google.com/about/careers/applications/jobs/results/` | Google Careers SSR HTML | Static CSS selector (`mode: "static"`) | **Fully Supported**: Server-rendered HTML DOM (`li.lLd3Je`). Container, title, destination, location, and ID locators verified. |
| **Apple** | `https://jobs.apple.com/en-us/search` | Apple Jobs SSR HTML | Static CSS selector (`mode: "static"`) | **Fully Supported**: Server-rendered HTML DOM (`li.rc-accordion-item`). Container, title, destination, location, and ID locators verified. |
| **Netflix** | `https://explore.jobs.netflix.net/api/apply/v2/jobs?domain=netflix.com&start=0&num=10` | Eightfold AI JSON API | Versioned script (`mode: "script"`, protocol v1) | **Fully Supported**: Dedicated versioned script extractor speaking `SCRIPT_PROTOCOL_VERSION = 1` over stdin/stdout NDJSON. |
| **Amazon** | `https://www.amazon.jobs/en/search.json?offset=0&result_limit=10` | Amazon Jobs Search API | Versioned script (`mode: "script"`, protocol v1) | **Fully Supported**: Dedicated versioned script extractor speaking `SCRIPT_PROTOCOL_VERSION = 1` over stdin/stdout NDJSON. |
| **Meta** | `https://www.metacareers.com/jobs` | React / Relay Dynamic SPA | Dynamic SPA / Bot-shielded | **Documented Acceptance Gap**: Meta Careers returns an empty client-side React shell with no server-rendered job cards in initial HTML. Relay GraphQL endpoint requires interactive bot clearance. Recorded capture proves empty DOM shell; documented truthfully per project rules. |
| **Authored Regression** | `fixtures/providers/regressions/remote-us-availability-authored.json` | Greenhouse-shaped fixture | Native `greenhouse` adapter | **Authoritative Control**: Proves remote workplace policy vs US applicant eligibility and negative prose control. |

---

## 3. Detailed Provider Contracts & Locators

### 3.1 Bitwarden (Greenhouse ATS)
- **Official Board:** `https://job-boards.greenhouse.io/bitwarden`
- **API Endpoint:** `https://boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true`
- **Payload Characteristics:** Clean JSON object containing `jobs` array with 39 postings. Each job provides `id` (integer), `title`, `absolute_url`, `location.name`, `offices`, `content` (HTML description), `first_published`, `updated_at`, and `pay_input_ranges`.
- **Worker Configuration:**
  ```json
  {
    "kind": "greenhouse",
    "name": "Bitwarden",
    "identifier": "bitwarden",
    "config": {}
  }
  ```

### 3.2 Cloudflare (Greenhouse ATS)
- **Official Board:** `https://job-boards.greenhouse.io/cloudflare`
- **API Endpoint:** `https://boards-api.greenhouse.io/v1/boards/cloudflare/jobs?content=true`
- **Single Job API:** `https://boards-api.greenhouse.io/v1/boards/cloudflare/jobs/8024849?questions=false`
- **Payload Characteristics:** Greenhouse JSON schema. Ingestion preserves original job ID, title, `absolute_url`, remote policy, and labeled "Available Locations" eligibility sections.
- **Worker Configuration:**
  ```json
  {
    "kind": "greenhouse",
    "name": "Cloudflare",
    "identifier": "cloudflare",
    "config": {}
  }
  ```

### 3.3 Google Careers (Static Selector)
- **Official Careers Page:** `https://www.google.com/about/careers/applications/jobs/results/` (redirect target from `https://careers.google.com/jobs/results/`)
- **HTML DOM Structure:** Server-side pre-rendered listing containing 20 job cards per page.
- **Locators:**
  - `item_selector`: `li.lLd3Je`
  - `title_selector`: `h3.QJPWVe`
  - `url_selector`: `a[href*="jobs/results/"]` (relative link resolved against origin)
  - `location_selector`: `span.r0wTof`
  - `id_selector`: attribute `ssk` on container or regex `jobs/results/(\d+)` from `url_selector`
- **Worker Configuration:**
  ```json
  {
    "kind": "selector",
    "name": "Google",
    "identifier": "https://www.google.com/about/careers/applications/jobs/results/",
    "config": {
      "mode": "static",
      "item_selector": "li.lLd3Je",
      "title_selector": "h3.QJPWVe",
      "url_selector": "a[href*='jobs/results/']",
      "location_selector": "span.r0wTof",
      "link_regex": "(jobs/results/[^\"?]+)"
    }
  }
  ```

### 3.4 Apple Jobs (Static Selector)
- **Official Careers Page:** `https://jobs.apple.com/en-us/search`
- **HTML DOM Structure:** Server-side pre-rendered accordion items containing 12–20 job cards per search page.
- **Locators:**
  - `item_selector`: `li.rc-accordion-item`
  - `title_selector`: `h3 a.link-inline`
  - `url_selector`: `a.link-inline[href*="/details/"]`
  - `location_selector`: `span.table--advanced-search__location-sub`
  - `date_selector`: `span.job-posted-date`
  - `id_selector`: regex `/details/(\\d+)` from `url_selector`
- **Worker Configuration:**
  ```json
  {
    "kind": "selector",
    "name": "Apple",
    "identifier": "https://jobs.apple.com/en-us/search",
    "config": {
      "mode": "static",
      "item_selector": "li.rc-accordion-item",
      "title_selector": "h3 a.link-inline",
      "url_selector": "a.link-inline[href*='/details/']",
      "location_selector": "span.table--advanced-search__location-sub",
      "date_selector": "span.job-posted-date"
    }
  }
  ```

### 3.5 Netflix (Versioned Script Extractor)
- **Official API:** `https://explore.jobs.netflix.net/api/apply/v2/jobs?domain=netflix.com&start=0&num=10`
- **Protocol:** `SCRIPT_PROTOCOL_VERSION = 1` over stdin/stdout NDJSON.
- **Mapping:**
  - `external_id`: `String(position.id)`
  - `title`: `position.name`
  - `url`: `position.canonicalPositionUrl`
  - `location`: `position.locations.join("; ")`
  - `workplace_type`: derived from `position.work_location_option` ("remote" vs "onsite")
- **Worker Configuration:**
  ```json
  {
    "kind": "selector",
    "name": "Netflix",
    "identifier": "https://explore.jobs.netflix.net/api/apply/v2/jobs?domain=netflix.com&start=0&num=10",
    "config": {
      "mode": "script",
      "script": {
        "runtime": "js",
        "interpreter": "node",
        "path": "crates/scrape-worker/examples/extractors/netflix_extractor.js"
      }
    }
  }
  ```

### 3.6 Amazon (Versioned Script Extractor)
- **Official API:** `https://www.amazon.jobs/en/search.json?offset=0&result_limit=10`
- **Protocol:** `SCRIPT_PROTOCOL_VERSION = 1` over stdin/stdout NDJSON.
- **Mapping:**
  - `external_id`: `String(job.id_icims)`
  - `title`: `job.title`
  - `url`: `https://www.amazon.jobs` + `job.job_path`
  - `location`: `job.normalized_location` || `job.location`
  - `date_posted`: `job.posted_date`
  - `description`: `job.description`
  - `salary`: extracted from `job.preferred_qualifications`
- **Worker Configuration:**
  ```json
  {
    "kind": "selector",
    "name": "Amazon",
    "identifier": "https://www.amazon.jobs/en/search.json?offset=0&result_limit=10",
    "config": {
      "mode": "script",
      "script": {
        "runtime": "js",
        "interpreter": "node",
        "path": "crates/scrape-worker/examples/extractors/amazon_extractor.js"
      }
    }
  }
  ```

### 3.7 Meta Careers (Documented Bot / Dynamic SPA Gap)
- **Official Page:** `https://www.metacareers.com/jobs`
- **Observed Behavior:**
  - Initial HTTP response delivers an empty React container (`<div id="root"></div>`) with bundled Relay bootstrap scripts.
  - No server-side HTML job items are rendered.
  - Background GraphQL queries (`/api/graphql`) require anti-bot tokens and session cookies.
  - Automated headless crawlers without valid sessions receive bot verification or blank responses.
- **Contract & Truthful Handling:**
  - Tested against real capture `fixtures/providers/meta/meta.html`.
  - Zero jobs parsed from static DOM.
  - Documented as an open acceptance gap per Plan 11 rules: "All FAANG entries need explicit evidence; blocked/unavailable/unsupported remain incomplete. Never fabricate production data."

---

## 4. Manifest Schema (`fixtures/providers/manifest.json`)

```json
{
  "sep": {
    "kind": "lever",
    "board": "sep",
    "capture_url": "https://api.lever.co/v0/postings/sep?mode=json",
    "captured_at": "2026-10-14T03:27:51.591154+00:00",
    "sha256": "dbda1f6de673d485bb313f973afca5023f17a55b3ac536ecf8d9d1f1425e3989",
    "extraction_version": "scrape-worker 0.1.0; protocol 7",
    "classification": "native_ats"
  },
  "linear": {
    "kind": "ashby",
    "board": "linear",
    "capture_url": "https://api.ashbyhq.com/posting-api/job-board/linear?includeCompensation=true",
    "captured_at": "2026-10-14T03:29:45.613116+00:00",
    "sha256": "c312aacc520506ff64c227452540daa7036f92bb215bbf0a56ba8fededbc2a38",
    "extraction_version": "scrape-worker 0.1.0; protocol 7",
    "classification": "native_ats"
  },
  "bitwarden": {
    "kind": "greenhouse",
    "board": "bitwarden",
    "capture_url": "https://boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true",
    "captured_at": "2026-10-17T23:09:00.000000+00:00",
    "sha256": "3a003b29a42085238bd10f2d3d5fee226df6f69ae39ca7a0c0555379b4dd71b3",
    "extraction_version": "scrape-worker 0.1.0; protocol 7",
    "classification": "native_ats"
  },
  "google": {
    "kind": "selector",
    "mode": "static",
    "capture_url": "https://www.google.com/about/careers/applications/jobs/results/",
    "captured_at": "2026-10-17T23:09:20.000000+00:00",
    "sha256": "b4d8c19bbafdda8fa6db42c5f89fe5ee5d05bef9c78b381cd1698e2686a9cae0",
    "extraction_version": "scrape-worker 0.1.0; protocol 7",
    "classification": "selector_static"
  },
  "apple": {
    "kind": "selector",
    "mode": "static",
    "capture_url": "https://jobs.apple.com/en-us/search",
    "captured_at": "2026-10-17T23:09:40.000000+00:00",
    "sha256": "0d43425ca5ecb5c36b402c5807ea7b9103ddba618d44d363374c399ea4d269c9",
    "extraction_version": "scrape-worker 0.1.0; protocol 7",
    "classification": "selector_static"
  },
  "netflix": {
    "kind": "selector",
    "mode": "script",
    "capture_url": "https://explore.jobs.netflix.net/api/apply/v2/jobs?domain=netflix.com&start=0&num=10",
    "captured_at": "2026-10-17T23:10:00.000000+00:00",
    "sha256": "43730b244926a29ea382ed47e367c650c09f327b7af558f639bf4caa660112a7",
    "extraction_version": "scrape-worker 0.1.0; protocol 7; script protocol 1",
    "classification": "script_ndjson"
  },
  "amazon": {
    "kind": "selector",
    "mode": "script",
    "capture_url": "https://www.amazon.jobs/en/search.json?offset=0&result_limit=10",
    "captured_at": "2026-10-17T23:10:20.000000+00:00",
    "sha256": "c455ca49307ac32a6c6dccb635913d2e0e4f1499a2ec6b90c442cf9becec3d20",
    "extraction_version": "scrape-worker 0.1.0; protocol 7; script protocol 1",
    "classification": "script_ndjson"
  },
  "meta": {
    "kind": "selector",
    "mode": "browser",
    "capture_url": "https://www.metacareers.com/jobs",
    "captured_at": "2026-10-17T23:10:40.000000+00:00",
    "sha256": "863d7a411ab319a457c38689a162631b86d8c7730cb49ad346e62cfdf78b77f0",
    "extraction_version": "scrape-worker 0.1.0; protocol 7",
    "classification": "dynamic_spa_bot_shielded"
  },
  "regressions": {
    "remote_us_availability_authored": {
      "fixture": "regressions/remote-us-availability-authored.json",
      "kind": "authored_regression_not_raw_provider_capture",
      "source_capture": null
    },
    "cloudflare_8024849": {
      "fixture": "regressions/cloudflare-8024849.json",
      "kind": "captured_provider_field_projection",
      "source_capture": "https://boards-api.greenhouse.io/v1/boards/cloudflare/jobs/8024849?questions=false",
      "sha256": "20e0e48e9e2ca890b690bcf5232ad8ca7a1f5ef5cd05c372104ca6f32eef8bcc"
    }
  }
}
```

---

## 5. Implementation Phases

- **Phase 7A: Fixture Ingestion & Manifest Registration**
  - Store raw captures: Bitwarden, Google, Apple, Netflix, Amazon, Meta under `fixtures/providers/`.
  - Update `manifest.json` with exact URLs, timestamps, SHA-256 hashes, and classifications.
  - Update `fixtures/providers/README.md`.

- **Phase 7B: Authored Extractor Scripts**
  - Create `crates/scrape-worker/examples/extractors/netflix_extractor.js`.
  - Create `crates/scrape-worker/examples/extractors/amazon_extractor.js`.
  - Verify standalone execution of both extractors with mock stdin and output NDJSON frames.

- **Phase 7C: Verification Test Runner Expansion (`scripts/provider-verification.mjs`)**
  - Update runner to verify all recorded providers against the manifest.
  - Set up loopback HTTP mock server for static and REST fixtures.
  - Replay each provider through packaged `scrape-worker` binary with `--replay-origin`.
  - Assert 1-to-1 fidelity on titles, external IDs, and authentic URLs.
  - Support `--live` opt-in verification.

- **Phase 7D: Native Worker-to-SQLite Boundary Tests (`src-tauri/tests/worker_repository.rs`)**
  - Implement integration tests exercising Bitwarden, Google, Apple, Netflix, and Amazon replay into SQLite.
  - Assert upsert into `jobs` table, canonical identity formation, and filter matching.

- **Phase 7E: Quality Assurance & Evidence Ledger Updates**
  - Run full test suite (`npm run test:providers`, `cargo test --workspace`, etc.).
  - Append Section 7 ledger entry to `plans/11-implementation-evidence.md`.
  - Update `AGENTS.md` and repository READMEs.
