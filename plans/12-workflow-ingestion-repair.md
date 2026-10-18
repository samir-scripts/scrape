# Workflow ingestion repair — implementation handoff for GPT-6.1 Sol

Status: **Sections 1–4 implemented; Section 5 template catalog/defaults is next.** Section 4 commits: evidence/parity `621d495`, generated script protocol-2 contracts `2e06cc8`, explicit country context `2e4e426`, contextual diagnostics/accounting `716fb2d`. See [implementation evidence](12-implementation-evidence.md). Sections 5–8 acceptance remains pending. Investigated 2026-10-18 at `8d8c45a`. The user requested implementation after the six new templates failed to populate Discovered.

This plan reopens plan 11 Sections 5–7 acceptance for seeded workflows, matching compatibility, and real company coverage. Earlier implementation records remain historical evidence; their broad completion/support claims do not establish working seeded workflows. Preserve plans 06–11 identity, provenance, suppression, retention, scheduling, trusted script execution, fair budgeting and protected-user-data requirements. Native acceptance from plans 08/10 remains outstanding.

## Intended result and constraints

A template is saved through the real workflow service, has all persisted children, and can reach the same worker and persistence path as an edited workflow. A valid matching candidate appears in Discovered unless existing status, suppression or visible view filters explain its absence. Zero is valid when no candidate matches; it must be distinguishable from failed discovery, missing evidence, trust/setup requirements, and persistence failure.

Keep one shared 20-accepted-job workflow budget (refreshes count), preview at most ten, round-robin fairness and cancellation. Never manufacture jobs, call unknown employment “full-time”, expand continental eligibility into guessed countries, silently clear legacy filters, auto-trust scripts, or disable foreign keys to make results appear.

Implement **one numbered section per coherent behavior/test commit**. Inspect/checkpoint the tree first. Write a failing desired-behavior regression, implement, verify, update evidence, then commit explicit files. Rebuild the sidecar before host/process/native checks whenever core/worker code changes. Commit generated Rust→TypeScript bindings separately. Do not rerun archived diagnostic assertions as desired behavior: they intentionally document current bugs.

## 1. Establish one end-to-end reproduction and useful run evidence

Checkpoint `141b113` implements shared seed/execution preparation, redacted per-workflow context and preflight/persistence-stage diagnostics, plus seed → packaged worker → SQLite → Discovered controls. Current matching, ownership and extraction defects remain recorded in Incident 28; repair-specific acceptance is still required in Sections 2–7. Full native/UI acceptance and exact saved-user-run reproduction remain open.

Files: `src-tauri/tests/worker_repository.rs`, `src-tauri/src/services/{runner,ingestion}.rs`, `src-tauri/src/commands/{sources,scrape}.rs`, `src/lib/events.ts`, existing native test harness and workflow editor tests.

- Add a test harness that uses the **same service behind Load test workflows**, rather than constructing replacement sources manually. Refactor IPC logic into a callable service as needed; do not copy seeding behavior into the test.
- Record seed origin/version, canonical spec and effective worker filters, persisted parent/child IDs, source enabled/review/trust state, actual worker version, provider totals, examined/invalid/duplicate/rejected/unresolved/blocked counts, accepted/inserted/refreshed counts, and persistence failures. Diagnostic exports must redact credentials/session content and bounded stderr.
- Capture unrestricted, keywords-only and exact-template runs against the **same response**. Separate ingestion rules from the Discovered view filter/status query. Assert completed run events invalidate saved jobs/counts; changing pages or filters must not conceal saved jobs without explanation.
- Add failures for missing child source rows, unknown role excluded by template, missing geographic evidence, legacy semantics changes, and Google URL resolution. Use temporary SQLite with foreign keys enabled and existing local HTTP replay infrastructure.
- For the user's particular all-zero observation, obtain the saved effective configuration/run diagnostics through a safe diagnostic export if needed. Do not test against or modify their live database. Current evidence proves Cloudflare 3 and Linear 2 can reach the repository query in isolation; do not invent a universal single root cause.

**Gate:** tests fail at the real problematic boundaries, and logs identify whether zero occurred before matching, at matching, at host persistence, or only in the view. Captured fixtures/projections must have classification, URL, timestamp and checksum; preserve a durable full Cloudflare capture if exact board counts become test expectations. The existing `/tmp` Cloudflare capture is not a committed fixture.

## 2. Make seed/save/import share workflow ownership and trust persistence

Implemented and fixture-verified: atomic parent/child saves, canonical IDs, workflow-scoped endpoint uniqueness (migration 27), parent-only scheduling/listing/retention, source-specific script review and disabled Save, and an explicit backed-up ownership repair helper. The helper has not been run against user data. Existing matching, provider extraction and native acceptance gaps remain open.

Files: `commands/sources.rs`, `db/sources.rs`, `services/workflow.rs::persist`, `services/script_trust.rs`, `SourcesSettings.tsx`, `api.ts`.

- Route all six templates through typed Rust validation and canonical workflow persistence. `SourcesRepo::create` alone does not save a multi-source spec.
- Persist every child, allocate collision-safe IDs, and rewrite the saved spec to those IDs in the same atomic transaction. The worker must only emit IDs owned by that workflow. Consolidate seed/editor/save paths without duplicating identity logic.
- Preflight all children before mutation; roll back an entire workflow if one fails. A batch seed may report per-workflow outcomes truthfully. Duplicate detection must remain idempotent and must not overwrite a user's edits. Report created, already present, and needs-review outcomes.
- Script templates stay disabled until **each executable source** is saved, reviewed and trusted. Ensure the UI can save a disabled executable draft and review its exact executable fingerprint before preview; imported executable authority remains local-only. Review is not automatic enablement.
- Repair existing malformed seeded workflows through a deliberate, idempotent forward data repair using the same ownership service. Preserve parent/source IDs already in use, source/job relationships, schedule state, notes/favorites/status/history/suppression. Create only missing owned children; reject cross-workflow ownership conflicts. Do not drop/reseed the database. Back up before a persistent repair and test failure/retry/restart against populated historical data. A new schema migration is not assumed necessary; never rewrite applied SQL.

**Gate:** new Google/Apple and Netflix/Amazon seeds have both owned child rows; Google/Apple accepted candidates persist without SQLite 787; executable children can be reviewed by stored ID; running before trust still fails explicitly. Repeated seeding adds no duplicates and leaves customized filters unchanged. Multi-source save and removal preserve historical jobs.

## 3. Restore matching compatibility; eliminate three competing migrations

Implemented: one non-destructive Rust compatibility resolver preserves independent groups, text fields, sample/arbitrary SQL, policy and compensation. Explicit legacy inheritance is isolated from modern workflows. Standard Matching exposes retained restrictions and Rust descriptions; the unsafe migration action is removed. All three Incident 28 flips are covered through validation and packaged-worker/persistence fixtures. No previously lost restrictions or live data were reconstructed.

Files: `crates/scrape-core/src/{protocol,matching,filter}.rs`, `services/{workflow,runner}.rs`, `src/lib/yamlWorkflow.ts`, `WorkflowSections.tsx`.

- Replace destructive `normalize_into_matching` behavior with one authoritative Rust compatibility operation. Do not union independent Any groups or role constraints, convert legacy OR locations into AND dimensions, change search fields, or remove SQL by recognizing its text.
- Prefer preserving existing legacy predicates as visible compatibility restrictions when the current structured model cannot express an equivalent transformation. Automatic conversion is allowed only when equivalence is established. Keep arbitrary query/policy active and editable.
- If a user wants a behavior-changing replacement, show what restrictions will change and require their explicit edit/confirmation. This is a product action, not permission for the implementer to investigate. A “migrate” click must not claim lossless behavior when it broadens/narrows matches.
- Remove frontend inference/migration logic; expose Rust conversion/diagnostics through generated boundary types. Do not maintain host/core copies of the same rule compiler. Audit global-filter merging and legacy inheritance so a modern independent workflow never acquires hidden global restrictions.
- Round-trip YAML/import/save/reopen/preview/run. Preserve effective semantics for separate Any/All/exclude groups, role intersections, mixed remote/city locations, text fields, exact sample query strings, arbitrary queries, policy and compensation. If wire/spec semantics change incompatibly, version them explicitly.

**Gate:** independent regression expectations show identical before/after acceptance for every lossless conversion. The three Incident 28 semantic flips disappear. Explicit user edits remain possible and invalidate preview. No silent broadening or narrowing on load/validation/run.

## 4. Repair field evidence and geographic predicate parity

Implemented: primary Lever country context/unlisted cities, Ashby addresses and explicit workplace precedence, truthful unknowns, same-tuple worker/SQLite geography, per-field evidence and bounded country diagnostics. Script protocol 2 carries typed evidence through shared validation and host ingestion. Recorded SEP and Netflix controls persist the exact expected postings without fabricated fields. Worker protocol 7/WorkflowSpec 6 remain unchanged; no live data/migration was touched.

Files: `adapters/{greenhouse,lever,ashby,selector}.rs`, `script_runner.rs`, `scrape-core/src/{matching,geography,role}.rs`, `enrich/location.rs`, repository query tests.

- Keep unknown role values unknown. Prefer explicit ATS employment/workplace evidence, including documented Lever commitment/country and Ashby employmentType/workplaceType/structured primary and secondary addresses. Greenhouse custom metadata must be mapped from actual captured field semantics; no universal full-time default.
- Resolve SEP's `Westfield, IN` with authoritative provider country context and the region registry; avoid a growing hard-coded city allowlist as the only parser. Test ambiguous IN/CA, multiple locations, unlisted cities and raw-only text. Store the original evidence alongside normalized values.
- Keep continental labels such as Europe/North America distinct from country eligibility. Improve extraction of supported evidence; show when a selected country rule cannot be decided from it. Do not guess residence/work authorization.
- Make all country/region/city constraints operate on the **same** location tuple. Remove cross-location matching through `jobs.location` inside per-row predicates. Specify and share city exact/substring and unknown behavior, including whitespace/Unicode considerations. Add worker↔SQLite parity with independently authored positive/negative expectations, not expected results generated by the same matcher.
- Replace coarse Location+Description “known” tests with appropriate per-field eligibility evidence. The Netflix workplace-only replay must not remain unresolved merely because description is absent when authoritative workplace evidence is available. If information really is unavailable after enrichment, retain unknown and follow the user's unknown policy; do not mark all fields complete to force acceptance.
- If script candidate frames need employment/workplace or structured location evidence, extend the Rust-owned protocol with an explicit version bump, validate it in the host, and update examples. Do not let scripts claim they have accepted a job.

**Gate:** SEP structured geography can match its actual US/Indiana evidence; Toronto+Cupertino cannot satisfy Canada+Cupertino; unknown policies agree in worker and repository; completed evidence distinguishes concrete workplace from absent description. Missing required evidence produces truthful partial/rejection diagnostics.

## 5. Make built-in templates usable, consistent and honestly labelled

Files: `db/sources.rs`, `src/lib/yamlWorkflow.ts`, `workflows/`, template picker and Matching UI.

- Establish one canonical versioned template catalog owned/validated by Rust. Generate or load frontend options/YAML from that catalog rather than maintaining separate JSON, TypeScript and YAML configurations. Keep provider verification configurations linked to the same extractor definitions.
- For **new** runnable examples, default to title keywords with role/workplace/geography unconstrained unless those restrictions are part of an explicitly labelled example. Unknown values remain unchanged. Expose Unknown employment/workplace choices or equivalent explicit controls for users who want selected known types plus unknowns. Show every active restriction in the summary.
- Keep restrictive role/geography examples as such, with preview diagnostics; a zero-match exact fixture may be legitimate. Never remove restrictions from existing workflows automatically to make a demo count rise.
- Reconcile Cloudflare naming/spec/YAML (the old v2 internships file remains present), SEP geography, Amazon URL differences, and duplicate legacy role arrays in default seed input. Separate executable examples needing setup from ready-to-run ATS templates.
- Make script resources portable: package versioned built-in extractors or resolve explicit imported file paths reliably. Repository-relative paths and finding `node` through a developer shell are not a packaged-desktop installation contract. Missing runtime/file gets setup guidance; no implicit download or trust.

**Gate:** frontend picker, YAML import and seed catalog yield equivalent validated specs. New broad examples produce known expected fixture jobs without invented attributes. Existing saved filters are not altered; script examples remain clearly disabled/Needs review until configured.

## 6. Repair actual site extraction, pagination and URL identity

Files: `adapters/selector.rs`, `adapters/selector_session.rs`, `script_runner.rs`, `examples/extractors/{netflix,amazon}_extractor.js`, provider manifests/captures.

- Implement shared document-base resolution using final response URL plus the applicable HTML `<base href>`; use it for posting/detail/next-page/asset references. Apply existing URL/domain/private-network protections to resolved targets. Preserve true provider ID/title/destination and evidence page. A base tag is not permission to fetch arbitrary hosts.
- Add Google regression asserting the exact correct destination, not merely an `http` prefix. Test redirects, missing/relative/multiple/unsafe base tags, and detail/page links. Investigate safe forward handling of previously saved malformed Google URL identities without losing annotations or suppression; do not blindly merge or rewrite them.
- Inspect and capture actual Google/Apple pagination and detail/location/employment evidence. Configure supported static extraction where it works; use browser mode only where the captured contract needs it. Ensure Apple field selectors match real cards/detail data, not a guessed CSS name.
- Implement resumable Netflix/Amazon scripts using versioned continuation and captured provider pagination metadata. The next turn must preserve source/candidate position, stop at limits/cancellation, detect loops and report bounded vs exhausted accurately. Do not infer board exhaustion from the length of one page. Script output budgets are backpressure, not the provider's total count.
- Remove fabricated title/URL fallbacks. Validate identifiers and required fields and report malformed candidates. Verify publication date semantics from evidence; an arbitrary creation/edit timestamp is not automatically a publication date. Source-specific API failures, access challenges and schema changes remain explicit.
- Test matching jobs on later pages, all early postings rejected, no duplicates across turns, shared workflow cap/fairness, one failed/slow source, and cancellation/process cleanup. Never put live providers in required CI.

**Gate:** correct Google URLs and source identity survive worker→database; eligible later-page fixtures reach the matching cap; no source declares full coverage after one page when more remain; parser errors and partial evidence do not masquerade as a successful exhausted scan.

## 7. Replace misleading provider acceptance with full workflow verification

Files: `scripts/provider-verification.mjs`, `fixtures/providers/manifest.json`, `src-tauri/tests/worker_repository.rs`, frontend/native harnesses.

- Make the manifest select companies, routes, extractor versions, fixture classes, expected coverage and acceptance status. Include Cloudflare explicitly. Distinguish passed, partial, blocked, unsupported and failed; Meta's zero-job shell is an open gap, not a passed ingestion.
- Serve exact recorded listing/detail/page/asset routes; reject unexpected requests instead of serving the same board for every path. Assert request coverage, next-page behavior, correct identity/destination and field provenance. Assert terminal status and errors, not just process exit/nonempty jobs.
- Run the exact seed/import/save service, trust/setup steps where needed, host workflow preparation/launch, actual packaged worker, budget validation, SQLite ingestion and Discovered query. Use the real six-template configuration, with clearly labelled alternate unrestricted/strict controls.
- Exercise the UI path Load test workflows → Run → run diagnostics → Discovered, including persisted filters/status, refreshes, suppressed jobs, no-match scans and persistence failures. Explicitly label native QA versus browser/demo tests. Test configured scripts from a non-repository working directory.
- Maintain deterministic exact expected fixture results, and opt-in separately reported live workflow verification. The same live search URL or an unfiltered API fetch is not successful workflow ingestion. All FAANG entries require explicit results; blocked sites remain incomplete.

**Gate:** the previous green verifier would fail on missing child rows, restrictive template mismatches presented as runnable broad examples, duplicated Google URL paths, missing pagination and unresolved required fields. Fixture acceptance is reproducible and live results are separately labelled.

## 8. Integration, recovery and handoff

- Run targeted checks after each section; at integration run Rust formatting, Clippy, workspace tests, regenerated-binding drift, serial frontend tests, TypeScript/build, style checks, recorded provider matrix, browser journeys and isolated native startup/IPC/workflow tests. Rebuild the sidecar first. Investigate failures rather than skipping checks or raising limits to obtain green results.
- If a schema/data repair is required, test populated historical upgrades/repair, backup, injected rollback, retry, restart, retained schedules and protected job state. No code revert is a database rollback strategy.
- Review compact user-facing diagnostics: fetched/examined is distinct from eligible/accepted, refreshed jobs are not necessarily new discoveries, and tracked/removed/suppressed jobs are not reintroduced to Discovered.
- Update Incident 28, plan status/index, AGENTS onboarding, template/setup docs and plan 11 evidence with exact commands, commit IDs and actual outcomes. Separate deterministic, browser, native, live, and unperformed checks. Reopen earlier completion claims only for the affected scope.

**Completion:** the six-template path has verified ownership, explicit trust/setup, faithful effective rules, complete bounded extraction, correct persistence and truthful display; the user's specific zero-result report is either reproduced/repaired or clearly isolated as awaiting saved-run evidence. No supported-provider or native acceptance claim is made from a parser-only pass.

## Implementation research baseline

Use Incident 28's linked primary references for [Greenhouse](https://docs.greenhouse.io/job-board.html), [Lever](https://github.com/lever/postings-api), [Ashby](https://developers.ashbyhq.com/docs/public-job-posting-api), and [HTML base resolution](https://html.spec.whatwg.org/multipage/semantics.html#the-base-element). For undocumented career-site APIs, capture observed current behavior and version it; do not treat it as a stable public API contract. Reuse existing Rust CDP/script architecture instead of introducing another browser framework for this repair.
