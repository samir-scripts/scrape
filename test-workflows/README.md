# Test Workflows

This directory contains standalone test and development workflow profiles:

- `software-engineering-it-us-48h.yaml`: Test workflow targeting US Software Engineering, IT, and related positions from the past 48 hours, with a shared quota of 20 jobs across its configured sources.
- `software-engineering-web-scraper-test.yaml`: Bounded public-HTML crawler test against Lever boards (Palantir and SEP) with custom extractors, rate limits, and eligibility validation.
- `software-engineering-discovery-test.yaml`: Multi-source API search discovery test querying The Muse general feed and category-specific feeds.

Import standalone workflows through the Workflow Editor. Settings replacement loads the 48-hour profile from its bundled canonical YAML, disabled; use the explicit Run action after review. Loading never launches a run. Replacement leaves existing marked profiles and unselected custom workflows intact, validates selected workflow fingerprints, and takes a recovery backup before changing existing data. Adzuna remains disabled on installation until credentials are configured and the source is explicitly enabled. The alternative replacement target installs the three starter workflows.
