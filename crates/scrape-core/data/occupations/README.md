# O*NET role-title seed

`seed_role_titles.tsv` is the sole normalized role-title asset. It is generated
from the three official O*NET 31.0 CSV datasets listed with their SHA-256 digests,
URLs, CC BY 4.0 terms and attribution in `manifest.json`. Source CSV files are
build-time inputs and are not checked into the repository; the TSV is embedded in
the native application through `include_str!` and is not a spreadsheet.

Recreate or check the deterministic TSV with
`python3 scripts/prepare-role-title-seed.py`, passing `--occupation-data`,
`--reported-titles`, and `--alternate-titles` paths to the matching downloaded
CSV files. `python3 -m unittest scripts.test_role_title_seed` exercises trimming,
SOC validation, tier precedence, case-insensitive de-duplication, catch-all
filtering and ordering.

The seed contains 45,716 unique titles. It combines canonical occupation titles,
reported market titles and alternate job titles; generic “All Other” records are
excluded. Tier 1 outranks tier 2, which outranks tier 3. SOC codes remain attached
for source traceability. UI selections remain literal text filters, not semantic
occupation matching. Scrape's derivative includes information from the O*NET 31.0
Database by USDOL/ETA under CC BY 4.0; USDOL/ETA has not approved, endorsed, or
tested the derivative.
