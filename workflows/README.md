# Built-in workflow templates

Rust owns catalog version 2 in [`template_catalog.rs`](../src-tauri/src/services/template_catalog.rs). It validates WorkflowSpec v8 through the ordinary workflow service. The picker, these YAML files and verification extractors are generated projections. Edit Rust definitions or bundled scripts, not outputs, then run:

```sh
npm run gen:templates
npm run check:templates
npm run gen:types
```

**Load test workflows adds six examples without replacing existing workflows.** Four ATS templates (Cloudflare, Bitwarden, Linear, SEP) use title keywords only. Employment, workplace, and geography are unconstrained, including unknown evidence. Cloudflare's default is `cloudflare-systems-network.yaml`; it is not the internship example. The 20-job total cap includes refreshed postings; Preview accepts at most 10.

`*-restrictive.yaml` examples retain explicit role/workplace/geography restrictions; `software-engineering-internships.yaml` retains older internship/keyword/location-text semantics. New v8 projections add empty role/company lists without changing these predicates. These five restrictive examples appear in the picker but are not automatically seeded. Unknown provider fields stay unknown; zero matches may be correct. Inspect preview diagnostics before saving.

Google/Apple static extraction and Netflix/Amazon script examples are **disabled** and labelled Needs review / Setup required. They do not establish verified full-site coverage: URL resolution, site selectors and pagination remain Plan 12 Section 6 work. All matching restrictions are shown in Review, with exact YAML available.

New role-first workflows use public Search (The Muse by default), independently
of these company-board examples. Roles/optional Companies narrow observed feed
postings locally; direct boards may be attached without replacing matching.
`past_week` is the new-draft default; Today/rolling presets resolve at each run.
Arbeitnow is explicitly Germany-focused and does not establish publication dates.
Neither public feed guarantees every employer. See the
[v8 contract](../plans/workflow-v3.md) for provider schemas and bounds.

## Script setup

Netflix/Amazon scripts are embedded at compile time from `crates/scrape-worker/examples/extractors/`. Generated workflows carry the actual code under catalog v2 and custom script protocol 2; no repository files or working directory are needed. Executable fingerprint review remains local; code, arguments or interpreter edits invalidate trust.

1. Open each source's custom extraction settings and enter an **absolute path to an installed Node.js 18+ executable**. The desktop app does not download a runtime or search a developer shell for `node`.
2. Check the environment, then save the workflow disabled for review.
3. Reopen and explicitly review/trust each saved executable source. Trust does not enable anything.
4. Enable the sources/workflow explicitly, Preview, inspect diagnostics, then Save. Run now is a separate action.

Imported file scripts must use an absolute path to a readable regular file (maximum 1 MiB), or carry embedded code. Missing files/runtime produce setup errors. Existing saved configuration is preserved; a legacy relative path or `node` command must be configured locally before execution. Imports never carry trust. Scripts run with local privileges; process isolation is not an OS sandbox.

The existing browser demonstration remains labelled and deliberately restrictive. It is not the new desktop seed and is not live provider evidence. Required verification uses recordings/local fixtures and temporary databases; full native and live provider acceptance remains open.
