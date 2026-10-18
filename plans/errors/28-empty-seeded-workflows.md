# Incident 28 — Seeded workflows produce no Discovered jobs

Status: **Investigated; repair not implemented.** Recorded 2026-10-18 against `8d8c45a49faeb829e2b2d99cd6840ebbe54b243d`, branch `implementation/trustworthy-scraping`. Requested deliverable: diagnosis, research, error log, and an implementation handoff for GPT-6.1 Sol. See [Plan 12](../12-workflow-ingestion-repair.md).

## Scope and confidence

The user's report is that the newly supplied workflows produce no jobs in Discovered. The investigation reproduced several independent defects in current code. It **did not inspect the user's database or reproduce their exact saved run**. In an isolated database, the current Cloudflare and Linear templates did produce and persist some jobs. Consequently, “all workflows always return zero” is not an established universal diagnosis. Existing saved configuration, view filters, tracked/removed/suppressed jobs, network outcomes, and packaged runtime differences remain case-specific checks.

The earlier claim that the original 438/436/2 incident was conclusively repaired is not established by this investigation. Current results also contradict the broad “production-grade workflows” and “fully supported” claims. Historical verification remains valid only for its stated inputs.

## Measured results

Rebuilt the packaged worker, seeded **all six actual `SourcesRepo::sample_workflows_suite()` inputs** into temporary SQLite, and used `workflow_source_inputs` plus `resolve_worker_filters` to obtain worker configuration. Ran each of the eight source entries separately against captured listing data with six variants: unrestricted, full template, keywords only, role only, workplace only, geography only. Every replay retains the production 20-job cap; counts of 20 are capped, not provider totals. Grouped-workflow fairness was not exercised here.

| Source | Unrestricted accepted | Full template accepted | Main finding |
|---|---:|---:|---|
| Cloudflare | 20 | 3 | 435 examined; 432 rejected: 221 keyword, 211 role-type primary rejections. Many ordinary engineering titles normalize to `unknown`, not `full_time`. |
| Bitwarden | 20 | 0 | 39 examined; 16 keyword and 23 role-type primary rejections. Full-time-only rules reject missing employment evidence. |
| Linear | 20 | 2 | 31 examined; 19 geography and 10 keyword primary rejections. Many locations are “Europe” / “North America”, not the template's selected countries. |
| SEP | 8 | 0 | 8 examined; 2 keyword and 6 geography primary rejections. Recorded `Westfield, IN` becomes entirely unknown geography despite provider `country: US`. |
| Google | 20 | 0 | First 20 listings: 12 keyword and 8 role-type primary rejections. Destination URLs also resolve incorrectly. |
| Apple | 20 | 0 | First 20 listings: 17 keyword and 3 role-type primary rejections. Several extracted locations are empty. |
| Netflix | 10 | 0 | First ten listings all fail keywords; no continuation. Script workflow is also quarantined at the host. |
| Amazon | 10 | 0 | First ten listings: 8 keyword and 2 role-type primary rejections; no continuation. Script workflow is also quarantined at the host. |

These are mutually exclusive **primary** rejection counts. Additional failed conditions overlap. Full terminal statistics, per-filter variants, errors, and unknown-field distributions are in [results.json](fixtures/incident-28/results.json).

Cloudflare listing was fetched read-only from its public API at `2026-10-18T07:08:48.421867+00:00`; SHA-256 `b0ed7d17fb960aa5ae6874f2f736ae23b8f0cda1e84d2c4dfc1451224a957030`. Original bytes remain at `/tmp/incident28-cloudflare.json`; they are not committed or claimed as a durable fixture. Other seven sources use existing manifest captures. Replay returns 404 for uncaptured details/assets, so enrichment warnings are expected and these are **listing replays**, not full provider coverage. No live workflow ingestion was performed.

The real worker's accepted events for the first four templates were subsequently passed through `scraped_job_from_event`, `JobsRepo::upsert_scraped`, and the Discovered statuses query in another temporary database: **Cloudflare 3, Bitwarden 0, Linear 2, SEP 0**. Only local source IDs were rebound for that fresh database; provider titles/IDs/URLs were preserved. This is a persistence/query boundary check, not native UI or full `ScrapeRunner` execution.

## Confirmed defects and actionable errors

### 1. Multi-source seeding skips source ownership persistence

`src-tauri/src/commands/sources.rs::seed_all_sample_workflows` calls `SourcesRepo::create` once per template. That repository method inserts one parent workflow and one source row. The embedded spec has two sources for Google/Apple and Netflix/Amazon, but their child IDs never become source rows.

`services/runner.rs` reconstructs both children from that embedded spec and uses literal child IDs (`google`, `apple`, `netflix`, `amazon`) when the spec has multiple sources. Direct ingestion of a valid authored candidate with each such ID fails:

```text
Database error: error returned from database: (code: 787) FOREIGN KEY constraint failed
```

This failure is latent behind matching rejection for Google/Apple. Netflix/Amazon fail earlier at persisted trust lookup:

```text
Validation error: Custom script needs local review and explicit trust before preview or execution. Save it disabled first.
```

Quarantine is correct. The defect is seeding a script workflow without all reviewable source records. The ordinary workflow save path, `workflow::persist`, already materializes child rows and must be the foundation for the fix. Existing seed commands skip existing matching identifiers, so reloading examples does not update old saved Cloudflare filters.

### 2. Templates impose evidence requirements their extractors cannot meet

`greenhouse.rs` calls `RoleType::infer(&job.title, None)`. `role.rs` deliberately returns `unknown` for an ordinary title such as “Senior Software Engineer”. Greenhouse's documented standard job-board payload does not guarantee a full-time field. The new full-time-only Bitwarden and full-time/internship Cloudflare rules therefore exclude genuine engineering postings with unknown employment type. Static/script candidates likewise lack explicit employment hints.

Do **not** repair this by defaulting unknown to full-time. New templates need understandable evidence policy; existing user filters must remain unchanged unless explicitly edited.

### 3. Geography discards available evidence and has worker/SQLite drift

Lever's recorded SEP payload supplies `country: US`, but `LeverPosting` does not deserialize it. The location parser leaves `Westfield, IN` with no country, subdivision, or city. Listing it in template cities cannot compensate for the missing parsed country/subdivision. Ashby address fields are also not represented in the adapter; continental labels must not be silently equated with individual countries. Apple selectors frequently extract no location in the current capture.

The city SQL additionally checks the aggregate `jobs.location` inside the predicate for each individual `job_locations` row. Authored two-location case:

- Locations: Toronto/CA-ON/CA and Cupertino/US-CA/US.
- Rule: country CA **and** city Cupertino.
- Worker rejects; SQLite returns the posting because the Toronto row borrows “Cupertino” from the aggregate location string.

Both results were reproduced. Conditions must apply to the same location evidence. Unknown-city and raw substring semantics also need independent parity cases.

### 4. “Normalization” changes rule meaning

Current `WorkerFilters::normalize_into_matching` was exercised against the same candidate before/after conversion:

| Case | Before | After |
|---|---|---|
| Legacy locations `[Remote, Austin]`, onsite Austin posting | accepts | rejects |
| Legacy include `[Software]` AND structured Any `[Engineer]`, title “Software Designer” | rejects | accepts |
| Exact recognized default SQL query, structured rules present, title “Designer” | rejects | accepts |

Locations are split into dimensions combined with AND; independent Any groups are unioned; recognized SQL strings are dropped. These are semantic changes, not safe normalization. Role-type union and legacy text search field changes require the same scrutiny. Logic is duplicated in core `protocol.rs`, host `workflow.rs`, and frontend `yamlWorkflow.ts`; frontend location conversion is also different from Rust. Some changes occur automatically during validation or execution, before any explicit UI migration click.

### 5. Coverage, evidence and identity bugs survive parser checks

- Google/Apple templates configure no next-page or detail extraction. They terminate after the first listing page.
- Netflix/Amazon scripts read one API page, never consume `input.continuation`, emit no continuation frame, and declare exhaustion based on that one array. Netflix's capture advertises 405 positions but supplies ten. Amazon's seeded endpoint also differs from its YAML endpoint.
- Netflix workplace-only replay emits **zero**, `unresolved: 10`, `outcome: partial`, even though unrestricted events contain concrete workplace values. `MatchingEvidence::knows(Workplace)` requires both Location and Description; absent description keeps evidence unresolved despite `opportunity_exhausted`. This requires a per-field evidence fix, not universal “complete” evidence.
- Google capture has `<base href="https://www.google.com/about/careers/applications/">` and a relative `jobs/results/...` href. Static extraction joins it to the listing URL instead, producing `.../jobs/results/jobs/results/...`. That is both a destination and canonical identity defect. The existing verifier only checks that a selector URL begins with `http`.
- Both example scripts have fallback invented titles/URLs. Validate required source fields instead; inspect provider-specific identity/date semantics before adopting them.

## Why existing tests pass

`npm run test:providers` **passed again during this investigation**. It processed eight entries: SEP 8, Linear 31, Bitwarden 39, Google 20, Apple 20, Netflix 10, Amazon 10, Meta 0 (documented gap). Cloudflare is not in its hard-coded provider list. It runs with empty filters, hand-built source configuration, script fixture paths, no local trust checks, and no database persistence. Its loopback fallback serves listing bytes for unexpected paths. The manifest checks bytes/provenance but does not select the provider list. “8/8 verified” is not eight successful template ingestions, and Meta remains a gap.

The repository integration suite may prove separate manually constructed configurations; it does not prove the new **seed → persisted child ownership/trust → effective template rules → runner → Discovered** path. Do not loosen those checks; add the missing boundary tests.

## Research

Checked primary references on 2026-10-18:

- [Greenhouse Job Board API](https://docs.greenhouse.io/job-board.html): use listing `content=true` for description/offices; exposed custom metadata is provider-controlled. Detail publication evidence is distinct from `updated_at`. No blanket full-time inference is justified.
- [Lever Postings API](https://github.com/lever/postings-api): consume explicit `country`, categories/commitment and location evidence; list pagination uses `skip`/`limit`.
- [Ashby Job Postings API](https://developers.ashbyhq.com/docs/public-job-posting-api): consume employment/workplace and structured primary/secondary addresses; absent data stays absent.
- [HTML standard: base element](https://html.spec.whatwg.org/multipage/semantics.html#the-base-element): document base governs relative URL resolution. Preserve existing URL/domain protections after resolution.
- Official [Google](https://www.google.com/about/careers/applications/jobs/results/), [Apple](https://jobs.apple.com/en-us/search), and [Netflix](https://explore.jobs.netflix.net/careers) career surfaces were opened as research. This does not verify the application's live extraction. The web tool's Amazon JSON request failed; no live Amazon support conclusion follows.

Netflix/Amazon endpoints are observed site contracts, not established public API guarantees. Capture and version actual pagination, identity, details and termination behavior before claiming coverage.

## Verification and reproduction

Actually run: sidecar build; 48 isolated source/filter replays; three temporary diagnostic host tests (all three passed after correcting probe setup); existing provider replay command (passed); DOM inspection of saved Google/Apple captures. The diagnostic tests assert observed defects and must **not** become permanent desired-behavior tests. Initial probe errors (test-only `init_memory_db` unavailable to integration tests; missing `location_data_complete` in an authored parity event) were corrected without changing application code. First loopback attempt was sandbox-denied; rerun with local-server permission succeeded.

Evidence artifacts:

- [Archived temporary host probes](fixtures/incident-28/host-probe.rs.txt)
- [Worker replay probe](fixtures/incident-28/replay.mjs)
- [Captured result summaries](fixtures/incident-28/results.json)
- Raw logs/events: `/tmp/incident28-host.log`, `/tmp/incident28-replay.log`, `/tmp/incident28-*-*.ndjson`, `/tmp/incident28-providers.log`, `/tmp/incident28-build.log`.

To reproduce on the tested Apple Silicon/macOS setup: copy the archived host probe to the previously absent `src-tauri/tests/incident28_probe.rs`; rebuild the sidecar; run only `incident28_seed_and_persistence_probe` to produce `/tmp/incident28-host.json`; supply the captured Cloudflare file or deliberately acquire a new, separately labelled capture; run `node plans/errors/fixtures/incident-28/replay.mjs` from repo root with loopback binding allowed; then run all `cargo test -p scrape --test incident28_probe -- --nocapture`. Archive results and remove only the temporary probe file. Adapt the diagnostic executable triple on other platforms. A fresh Cloudflare capture changes counts; the old raw bytes are not in Git.

Not run: full workspace/frontend/Clippy suite, native UI/IPC, full native runner, scheduled execution, live multi-provider ingestion, user-database inspection, or repairs. No migrations, production source edits, or user-data changes were made.
