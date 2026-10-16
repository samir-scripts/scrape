# `crates/scrape-worker/tests/fixtures` — Offline Test Fixtures

## Overview

The `crates/scrape-worker/tests/fixtures` directory stores static mock payloads representing real-world responses from supported job board platforms and HTML pages. These fixtures allow unit and integration tests to run deterministically and offline without network access or third-party rate limits.

---

## File Manifest

| File | Target Format / Platform | Description & Key Scenarios Covered |
|---|---|---|
| [`ashby.json`](ashby.json) | Ashby REST API | Sample Ashby posting board JSON containing job listings with salary ranges, employment types, and location metadata. |
| [`detect_greenhouse.html`](detect_greenhouse.html) | HTML Web Page | HTML document containing Greenhouse iframe embeds and script links, used to verify ATS auto-detection. |
| [`detect_lever.html`](detect_lever.html) | HTML Web Page | HTML document containing links to Lever career pages, used to verify ATS auto-detection. |
| [`greenhouse.json`](greenhouse.json) | Greenhouse REST API | Realistic Greenhouse board payload (`/v1/boards/{board}/jobs?content=true`) testing job parsing, HTML description cleanup, departments, and office locations. |
| [`greenhouse_branding.html`](greenhouse_branding.html) | Greenhouse Board HTML | HTML fixture containing employer-labelled branding markup for testing Greenhouse board logo extraction. |
| [`cloudflare-logo.json`](cloudflare-logo.json) | Branding Provenance | Provenance metadata (capture URL, timestamp, original JPEG SHA-256, and canonical PNG SHA-256) for the recorded Cloudflare logo fixture. |
| [`cloudflare-logo.jpg`](cloudflare-logo.jpg) | Raw Logo Bytes | Recorded original JPEG-encoded bytes served by the Cloudflare logo URL used to verify content-based image decoding. |
| [`cloudflare-logo-cache.png`](cloudflare-logo-cache.png) | Normalized Logo PNG | Canonical PNG produced by `scrape-core::branding` validation (reproducible via `cargo run -p scrape-core --example cache-logo-fixture`). |
| [`jsonld_single.html`](jsonld_single.html) | Schema.org JSON-LD | HTML page with an individual `<script type="application/ld+json">` declaring a `JobPosting` entity. |
| [`jsonld_graph.html`](jsonld_graph.html) | Schema.org JSON-LD | HTML page containing a Schema.org `@graph` array with multiple nested `JobPosting` entities. |
| [`lever.json`](lever.json) | Lever REST API | Realistic Lever `/v0/postings/{company}` payload testing location categories, workplace types, and salary descriptions. |
| [`selector.html`](selector.html) | Generic HTML Page | Multi-job listing HTML document with nested elements, testing CSS selector extraction, regex pattern captures, and glob URL filtering. |
