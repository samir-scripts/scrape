# Test Workflows

This directory contains standalone test and development workflow profiles:

- `software-engineering-it-us-48h.yaml`: Independent public-web discovery for US Software Engineering, IT, and related positions from the past 48 hours, with a shared cap of 20 newly saved jobs. The public directory and sampled Common Crawl seeds are bounded starting points, not an exhaustive web index.
- `software-engineering-web-scraper-test.yaml`: Bounded public-HTML crawler test against Lever boards (Palantir and SEP) with custom extractors, rate limits, and eligibility validation.
- `software-engineering-discovery-test.yaml`: Multi-source API search discovery test querying The Muse general feed and category-specific feeds.

`common-crawl-2026-43-seeds.txt` records the sampled seed URLs and crawl observations used by the workflow; indexed observations are not publication dates and each destination is fetched again. Import standalone workflows through the Workflow Editor. Settings replacement loads the 48-hour profile from its bundled canonical YAML, disabled; use the explicit Run action after review. Loading never launches a run. Replacement preserves unselected workflows and takes a recovery backup before changing existing data. The alternative replacement target installs the three starter workflows.
