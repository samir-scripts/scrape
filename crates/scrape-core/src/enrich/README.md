# `crates/scrape-core/src/enrich` — Data Enrichment & Information Extraction

## Overview

The `crates/scrape-core/src/enrich` directory contains heuristic and deterministic information extraction engines. Raw job postings scraped from ATS APIs or web pages contain unstructured text, varied formatting, and ambiguous field labels. This module normalizes compensation, geography, legal eligibility, experience level, and workplace arrangements into strongly-typed structures.

---

## File Manifest

| File | Primary Functions & Types | Purpose & Heuristic Details |
|---|---|---|
| [`mod.rs`](mod.rs) | Module root & re-exports | Exposes all enrichment submodules and their public interfaces (`Salary`, `ParsedLocation`, `ClassifiedSponsorship`, `ClassifiedClearance`, `ClassifiedWorkAuth`, `ExperienceClassification`, `classify_workplace`, `to_plain_text`). |
| [`salary.rs`](salary.rs) | `Salary`, `extract_salary()`, `extract_salary_from_text()` | Parses structured ATS salary objects as well as raw strings (e.g. `"$150k - $200k/yr"`, `"$45.00 - $60.00 / hr"`). Detects currencies (`USD`, `EUR`, `GBP`, `CAD`), converts `k/M` multipliers, normalizes intervals (`hourly`, `monthly`, `yearly`), and avoids false positives on job requisition numbers or dates. |
| [`location.rs`](location.rs) | `ParsedLocation`, `parse_locations()`, `normalize_location()` | Deconstructs compound and slash-delimited location strings (e.g. `"San Francisco, CA / New York, NY / Remote"`) into lists of `ParsedLocation` structs with normalized city, state, country, and remote flags. Resolves ambiguous US state abbreviations and international metropolitan areas. |
| [`eligibility.rs`](eligibility.rs) | `ClassifiedSponsorship`, `ClassifiedClearance`, `ClassifiedWorkAuth`, `classify_sponsorship()`, `classify_clearance()`, `classify_work_auth()` | Analyzes compliance and citizenship requirements: detects explicit visa sponsorship statements (`"will sponsor"`, `"no sponsorship"`, `"H1-B"`), U.S. citizenship mandates (`ITAR`, `U.S. Citizen only`, `Green Card`), and national security clearance requirements (`Secret`, `Top Secret`, `TS/SCI`, `Polygraph`). |
| [`experience.rs`](experience.rs) | `ExperienceClassification`, `extract_experience()` | Extracts minimum and maximum required years of experience (`min_years`, `max_years`), identifies seniority levels (Entry, Senior, Staff, Principal), and computes the `is_new_grad` `TriState` flag by matching campus hire patterns while filtering out non-entry-level disqualifiers. |
| [`workplace.rs`](workplace.rs) | `classify_workplace()`, `WorkplaceType` | Classifies workplace policy into `OnSite`, `Hybrid`, `Remote`, or `Unknown`. Resolves conflicts where ATS APIs state `"In-Office"` in `location` while listing cities in `offices`, and inspects description text with negation awareness (e.g., `"not considering remote"`, `"in-person expectations"`, `"in office 3-5 days"`). |
| [`html.rs`](html.rs) | `to_plain_text()` | Strips HTML tags, decodes HTML entities, and normalizes excessive whitespace from raw ATS description fields into clean, searchable plain text for subsequent regex and NLP processing. |

---

## Integration in Scrape Pipeline

Every scraper adapter in `scrape-worker` pipes raw job payloads through this enrichment pipeline prior to packaging a `WorkerEvent::Job`:

```
Raw ATS Payload / HTML Page
          │
          ▼
   html::to_plain_text() ──► Clean Text
          │
          ├──► workplace::classify_workplace() ──► WorkplaceType (OnSite, Hybrid, Remote)
          ├──► salary::extract_salary()        ──► Salary (min, max, currency, interval)
          ├──► location::parse_locations()     ──► Vec<ParsedLocation>
          ├──► eligibility::classify_*()       ──► Sponsorship, Clearance, WorkAuth
          └──► experience::extract_experience() ──► ExperienceClassification (YOE, NewGrad)
          │
          ▼
   WorkerEvent::Job (Normalized Domain Event)
```
