# Built-in workflow templates

Rust owns catalog version 1 in [`template_catalog.rs`](../src-tauri/src/services/template_catalog.rs). It validates every WorkflowSpec v6 through the ordinary workflow service. The picker, these YAML files, and provider smoke-check extractor configurations are generated projections. Do not edit generated files; edit Rust definitions or the bundled script source, then run:

```sh
npm run gen:templates
npm run check:templates
npm run gen:types
```

**Load test workflows adds six examples without replacing existing workflows.** Four ATS templates (Cloudflare, Bitwarden, Linear, SEP) use title keywords only. Employment, workplace, and geography are unconstrained, including unknown evidence. Cloudflare's default is `cloudflare-systems-network.yaml`; it is not the internship example. The 20-job total cap includes refreshed postings; Preview accepts at most 10.

`*-restrictive.yaml` examples retain explicit role/workplace/geography restrictions. `software-engineering-internships.yaml` retains the older internship/keyword/location-text semantics, now in v6 and explicitly labelled restrictive. These five examples appear in the picker but are not automatically seeded. Unknown provider fields stay unknown; zero matching jobs may be correct. Preview and inspect categorized diagnostics before saving.

Google/Apple static extraction and Netflix/Amazon script examples are **disabled** and labelled Needs review / Setup required. They do not establish verified full-site coverage: URL resolution, site selectors and pagination remain Plan 12 Section 6 work. All matching restrictions are shown in Review, with exact YAML available.

## Script setup

Netflix/Amazon scripts are embedded at compile time from `crates/scrape-worker/examples/extractors/`. The generated workflow carries the actual code, under catalog v1 and custom script protocol 2; it needs no repository files or working directory. Each script is reviewed by its executable fingerprint. Editing code, arguments or interpreter invalidates local trust.

1. Open each source's custom extraction settings and enter an **absolute path to an installed Node.js 18+ executable**. The desktop app does not download a runtime or search a developer shell for `node`.
2. Check the environment, then save the workflow disabled for review.
3. Reopen and explicitly review/trust each saved executable source. Trust does not enable anything.
4. Enable the sources/workflow explicitly, Preview, inspect diagnostics, then Save. Run now is a separate action.

Imported file scripts must use an absolute path to a readable regular file (maximum 1 MiB), or carry embedded code. Missing files/runtime produce setup errors. Existing saved configuration is preserved; a legacy relative path or `node` command must be configured locally before execution. Imports never carry trust. Scripts run with local privileges; process isolation is not an OS sandbox.

The existing browser demonstration remains labelled and deliberately restrictive. It is not the new desktop seed and is not live provider evidence. Required verification uses recordings/local fixtures and temporary databases; full native and live provider acceptance remains open.
