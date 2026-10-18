# `plans/errors` — Troubleshooting & Solution Playbooks

## Overview

The `plans/errors` directory records diagnostic procedures, root-cause analyses, and architectural solutions for complex technical issues encountered during development.

---

## File Manifest

| File | Scope | Description |
|---|---|---|
| [`solutions.md`](solutions.md) | Technical Playbook | Detailed resolutions for platform-specific challenges, including Tauri v2 sidecar packaging, cross-compilation target triples, SQLite WAL connection locking, and CSS layout edge cases. |
| [`25-native-startup-migration-12.md`](25-native-startup-migration-12.md) | Incident 25 | Records the confirmed migration-12 checksum/schema variant, verification gap, and forward repair. |
| [`26-native-startup-migration-19.md`](26-native-startup-migration-19.md) | Incident 26 | Records the migration-19 structured job description startup incident and forward repair (`0020_finalize_structured_descriptions.sql`). |
| [`27-native-startup-migration-23.md`](27-native-startup-migration-23.md) | Incident 27 | Records the migration-23 employer branding startup incident and forward repair (`0025_finalize_employer_branding.sql`). |
| [`28-empty-seeded-workflows.md`](28-empty-seeded-workflows.md) | Incident 28 — open | Reproduced empty template results, missing multi-source ownership, semantic matching regressions, and extraction/verification gaps; links to Plan 12 and diagnostic evidence. |
