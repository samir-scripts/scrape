# Workflow specification v3

The desktop validator is authoritative for create, edit, import and preview. Rust contracts generate the TypeScript representations in `src/bindings`. Versions 1 and 2 migrate explicitly to version 3; unknown fields and unsupported versions are errors with paths.

The editor holds one draft. Simple creation follows Source → Matches → Schedule and save. Advanced fields and YAML share that draft. Configuration edits invalidate and cancel the previous preview. Saving never runs a scrape; the success state offers an explicit Run now action.

New editor workflows default to daily 09:00 in the device IANA timezone. Calendar scheduling uses that stored timezone. Manual workflows have no automatic next run. Global pause stops automatic dispatch; manual execution remains available.

Absent matching properties inherit global defaults, and explicit empty arrays remove inherited restrictions. Enriched `filters.matching` supports keywords (`any`, `all`, `exclude`, explicit fields), original compensation ranges, role/workplace/experience/eligibility/geography, and posting dates. Relative ages use one run-start instant. Date-only boundaries use the date filter's timezone, with the final date inclusive through the next midnight. Unknown posting dates are excluded unless requested.

Simple Matching exposes searchable Country and dependent State/province controls plus Remote/Hybrid/In-person workplace choices. Countries use ISO alpha-2 codes; newly selected subdivisions use qualified codes such as `US-CA` or `CA-ON`. Geography and workplace apply together, selected workplaces match with OR, and country/subdivision must match the same location entry. Changing country clears incompatible subdivisions. Imported multiple-country rules and legacy short subdivision codes remain intact until explicitly edited. The generated [catalog](../crates/scrape-core/data/README.md) has all 249 countries but a bounded subdivision list; Advanced retains raw codes for other subdivisions. Additional free-text location restrictions live in Advanced and still apply with AND.

Unknown geography is excluded when a geographic filter is active unless explicitly included. A bare Remote row does not override known contradictory geography elsewhere in the posting. Inclusion permits missing fields only on otherwise compatible locations; a qualified subdivision also provides country evidence for partial legacy rows. Malformed country/subdivision filters and entirely incompatible qualified country/subdivision combinations fail validation. Worker matching and SQL compilation share these rules.

Extraction prioritizes structured address country/subdivision fields over text inference, with bounded aliases and word boundaries. JSON-LD keeps every location-array entry and nested Country values; for remote roles explicit `applicantLocationRequirements` takes precedence over physical office geography, followed by labeled eligibility sections. General company prose does not establish job eligibility. Unsupported or ambiguous geography remains unknown.

`max_jobs_per_run` is an optional top-level whole number from 1 to 10,000, defaulting to **20** for new and existing workflows. It counts unique matching, valid, unblocked postings, including existing records refreshed by a run. Each workflow in Run all has its own budget. This is not a total saved-job capacity limit and does not remove existing jobs. The worker stops candidate processing and further detail/page requests when its budget is reached; downloading a provider's single complete-board response can be unavoidable. Matching follows provider order, so repeated runs may refresh the same first postings. Source summaries explicitly identify the limit and whether processing stopped there; totals describe examined records only.

Preview and Test use the same effective filters, block snapshot and worker acceptance pipeline, capped at the smaller of `max_jobs_per_run` and **10**. Preview has a bounded worker lifetime and cancellation, and writes no jobs, retention changes or scheduling state. A limit-reached result is labeled as potentially incomplete source coverage, rather than a complete-board count. Provider failures remain visible as errors. Browser demonstration previews use fictional fixtures and cannot verify date/eligibility extraction or expert rules.

YAML is decoded with the existing YAML library and sent to the backend validator. Form/YAML serialization preserves supported hidden configuration and source metadata. The source configuration object accepts only implemented adapter fields. Retention always protects tracked jobs, favorites and nonblank notes.

Validation: populated repository/worker matching parity tests, canonical validation tests, DOM editor tests (zero results, no implicit run, YAML/native pay preservation, stale preview cancellation), and the frontend suite. Browser and native verification are recorded separately in the implementation plan.

Greenhouse detail enrichment follows the [official Job Board API](https://docs.greenhouse.io/job-board.html): it retains `absolute_url` and `first_published`, stores `updated_at` separately, and preserves each `pay_input_ranges` currency/location range. The API does not promise a pay interval; an interval remains unknown unless explicitly evidenced. Detail requests have a 100-posting/45-second bound, a per-worker five-minute cache capped at 256 entries, and visible errors when enrichment is incomplete. Tests serve local list/detail payloads and verify custom destinations, provider company/title, separate dates, cents, multiple currencies and cache reuse.

Each persisted run records its effective worker configuration (including per-workflow specs and resolved filters) and separate fetched, rejected, blocked, inserted, updated, failed and retention-removal counts. Cancelling a desktop run kills its sidecar, consumes termination and flushes buffered valid postings while marking the run incomplete. A process guard also kills children when a persistence error or dropped execution future exits early. The shell plugin owns process reaping.

The native boundary regression in `src-tauri/tests/worker_repository.rs` invokes the packaged sidecar against a local HTTP fixture, parses real NDJSON with the production ingestion mapper and persists to a temporary SQLite database. It compares preview/single/global posting identities, excludes annual pay from hourly rules, preserves native compensation provenance and proves a blocked posting cannot be persisted. Run `npm run build:sidecar` before Rust integration tests.

Legacy versions decode through generated strict contracts; malformed roots,
non-integer versions, unknown selectors and corrupt stored filters fail explicitly.
Recognized nested legacy selectors normalize to the supported adapter fields.
Source identity and stored cron timezone remain authoritative during edit/export.
Simple mode summarizes active Advanced constraints instead of hiding them.

Clock schedules use named `MON-FRI` and `SAT,SUN` expressions. The Rust cron
library numbers Sunday as 1; historical expert numeric expressions retain that
meaning and remain in Advanced mode. Global cron validation and execution share
calendar scheduling's DST policy: first valid instant after a gap, first occurrence
of a repeated time, and no second execution during the repeated local hour.

`npm run test:native` additionally drives actual Tauri commands in an isolated
app, proving preview does not save/run, hourly provenance survives IPC, blocks
survive workflow recreation, restoration retains annotations, cancellation
releases execution guards, and malformed/unknown-source/invalid-UTF-8 output,
nonzero exit, missing completion and timeout retain valid partial results while
reporting failure. Native external opening must reach the stored local destination.
Browser demonstration extraction remains fictional; unsupported date/expert rules
require the desktop rather than silently broadening the fixture match.
