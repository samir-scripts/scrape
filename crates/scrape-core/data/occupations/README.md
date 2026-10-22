# BLS and O*NET occupation catalog

This directory vendors the unmodified BLS 2018 SOC coding XLSX, the original O*NET 31.0 Job Titles JSON compressed as gzip, source/license/checksum metadata in `manifest.json`, and the deterministic derived `catalog.json` consumed by Rust.

Run `python3 scripts/import-occupation-catalog.py` only for an explicit catalog refresh. `--check` validates the checked-in derived JSON offline and never writes. Importer tests use in-memory/temporary fixtures. The application loads the pinned catalog locally and performs no title lookup request.

BLS detailed occupations are complete for the 2018 SOC structure; BLS public-domain material is attributed to the Bureau per its reuse guidance. O*NET alternatives retain their original SOC and source-code references, including ambiguous title mappings. Catalog selection persists literal title strings and does not imply semantic occupation-wide matching. O*NET 31.0 material is used under CC BY 4.0; see `manifest.json` for per-dataset license, attribution and download provenance.
