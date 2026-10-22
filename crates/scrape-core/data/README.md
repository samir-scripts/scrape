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
## Occupation-title catalog

`occupations/` vendors the original BLS 2018 SOC coding XLSX and O*NET 31.0
job-title JSON gzip, their retrieval/license/checksum manifest, and a deterministic
`catalog.json`. `scripts/import-occupation-catalog.py --check` validates the
derived file offline; run the importer only for an intentional data refresh.
Rust loads and validates the catalog once from `scrape-core/src/occupations.rs`
and exports its generated TypeScript projection. BLS supplies all 867 detailed
occupations across 23 major groups; O*NET contributes alternate titles mapped
to their original SOC/O*NET provenance. A title remains a literal match string,
not a semantic occupation matcher or a complete worldwide job-title registry.
Review the source manifest and attribution before changing the pinned versions.
