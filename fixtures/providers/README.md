# Provider fixtures

`manifest.json` identifies raw, timestamped public provider captures by their capture URL, SHA-256 checksum, extraction version, and adapter classification. Files under `regressions/` may instead be authored deterministic edge cases. Each authored case must declare `fixture_kind`, keep `source_capture` null (or link to captured projection metadata), and must never be described or tested as a live provider response.

## Covered Providers and Platforms

| Identifier | Platform / Platform Kind | Extraction Mode | Fixture Path | Notes |
|---|---|---|---|---|
| `sep` | Lever API | Native ATS | `lever/sep.json` | 10 authentic postings. Unpaginated REST API response. |
| `linear` | Ashby API | Native ATS | `ashby/linear.json` | 35 authentic postings with compensation tiers. |
| `bitwarden` | Greenhouse API | Native ATS | `greenhouse/bitwarden.json` | 39 authentic postings with full descriptions and pay ranges. |
| `google` | Google Careers SSR HTML | Static CSS/XPath selector | `selector/google.html` | 20 pre-rendered job cards (`li.lLd3Je`) with relative links and locations. |
| `apple` | Apple Jobs SSR HTML | Static CSS/XPath selector | `selector/apple.html` | 12 pre-rendered accordion items (`li.rc-accordion-item`) with details links and dates. |
| `netflix` | Eightfold AI JSON API | Versioned script (`mode: "script"`) | `script/netflix.json` | 10 authentic postings extracted via script runner speaking protocol v1. |
| `amazon` | Amazon Jobs Search API | Versioned script (`mode: "script"`) | `script/amazon.json` | 10 authentic postings extracted via script runner speaking protocol v1. |
| `meta` | Meta Careers (React/Relay) | Dynamic SPA / Bot-shielded | `meta/meta.html` | Verified empty client-side DOM shell (`<div id="root"></div>`). Documented open acceptance gap. |

## Regressions and Projections

`regressions/remote-us-availability-authored.json` is an authored Greenhouse-shaped case paired with a negative company-prose control. `regressions/cloudflare-8024849.json` is a minimal projection of the official Cloudflare Greenhouse API response captured on 2026-10-15. Its SHA-256 is recorded in the fixture; the complete 18,972-byte response was retained locally for verification. The captured posting ID, title, `absolute_url`, Remote label, `first_published`, update timestamp, and labeled “Available Locations: US” excerpt match the official site.

`regressions/cloudflare-board-authored.json` is an authored three-record Greenhouse-shaped control used to isolate template matching from full live board counts.
`regressions/google-page1-authored.html` and `regressions/google-page2-authored.html` are authored multi-page HTML controls exercising document-base resolution and pagination across early non-matching to later matching postings.
`regressions/apple-page1-authored.html` and `regressions/apple-page2-authored.html` are authored multi-page HTML controls exercising `link[rel="next"]` navigation and `div.job-title-location span:not(.a11y)` location extraction.
`regressions/netflix-missing-total-authored.json` and `regressions/amazon-missing-total-authored.json` are authored script controls with omitted pagination totals exercising truthful bounded/partial termination rather than fabricated board exhaustion.

## Verification profiles

Recorded workflow cases select frozen descriptors from `src-tauri/src/services/provider_verification_profiles.rs` and persist only that profile through ordinary workflow storage. They do not seed the production catalog. `extractors.verification_profile_revision` records this control contract independently of the three user-facing catalog-v3 workflows; worker protocol 13 and script protocol 2 remain separately checked. Provider verification uses non-feedback candidate sampling. Raw capture bytes/checksums, provider cases, embedded-script fingerprints and expected outcomes are unchanged. Provider verification is extraction evidence for these recorded routes, not evidence of current targeted-search vacancies.
