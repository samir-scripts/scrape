# Geography catalog

Rust owns the catalog and exports `CountryOption`, `RegionOption`, and
`src/bindings/geographyCatalog.ts` through the normal binding-export tests.
Country codes/names come from the pinned [isocountry 0.3.2 API](https://docs.rs/isocountry/0.3.2/isocountry/enum.CountryCode.html).
Common display names and source aliases live in `geography.rs`. There is no
runtime geography service or separate handwritten frontend catalog.

`regions.json` consolidates the prior frontend subdivision choices and replaces
the separate US/Canada parser tables. It contains all 50 US states plus DC,
Canada's 13 provinces/territories, the UK's four constituent countries, and all
eight Australian states/territories. Existing partial choices for Germany (7),
France (3), and India (6) are retained. Greater London's old nonstandard `GB-LDN`
choice was removed; imported codes remain visible/editable through Advanced.
This is not a complete worldwide subdivision registry. Where a country has no
listed subdivisions, the searchable state control is disabled; country matching
still works and Advanced retains explicit subdivision codes.

Normalization is bounded, case/punctuation insensitive, and folds common Latin
accents. Explicit qualified codes and safe abbreviations precede full region
names, then city hints. Ambiguous country/state abbreviations and bare Georgia
remain unknown without supporting context. Remote never implies a country.
Structured remote applicant eligibility takes precedence over a physical office
address; extraction does not use general company prose as eligible geography.
Unsupported names remain unknown; regexes are not a universal geocoder.
## Runtime role-title seed

`occupations/seed_role_titles.tsv` is the single normalized O*NET 31.0 title
resource. `manifest.json` records official CSV source URLs, SHA-256 digests,
license and attribution. The source CSVs are build-time inputs and are not
checked in; the derived TSV is embedded in `src-tauri` and seeded into the
indexed SQLite title-search tables on first launch. `scripts/prepare-role-title-seed.py`
recreates or checks it from the three downloaded source files.

The seed contains canonical occupation titles, reported market titles and
alternate job titles, deduplicated case-insensitively and ranked by source tier.
Generic “All Other” titles are omitted. Suggestions persist as literal strings;
they do not change matching into semantic occupation matching. No spreadsheet
or legacy BLS/O*NET catalog is bundled.
