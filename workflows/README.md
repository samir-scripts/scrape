# Built-in workflow templates

Rust owns catalog version 3 in [`template_catalog.rs`](../src-tauri/src/services/template_catalog.rs). It validates WorkflowSpec v9 through the ordinary workflow service. The picker and these three YAML files are generated projections. Edit Rust definitions or bundled scripts, not outputs, then run:

```sh
npm run gen:templates
npm run check:templates
npm run gen:types
```

The catalog contains exactly three manual workflows, each capped at 20 accepted postings per run and retaining untracked postings for 30 days:

| Workflow | Authoritative local criteria |
| --- | --- |
| Software Engineering & Related — Past 12 Hours | Explicit engineering/related title phrases and original publication within 12 hours; unknown dates excluded. No employment, geography, workplace or pay restriction. |
| Fast Food — Cherry Hill, NJ — Past 30 Days | Front-line title phrases **and** literal fast-food/chain evidence in title/company/description; Cherry Hill, US-NJ and US in the same location tuple; original publication within rolling 720 hours. Unknown dates/geography excluded. |
| Remote — Advertised Annual USD Pay Over $60,000 | Remote-only and evidenced native annual USD advertised pay above $60,000. A $50k–$80k range qualifies; lower-only $50k and exactly $60k do not. No hourly/monthly annualization, currency conversion or date/geography restriction. |

Engineering and remote profiles include The Muse, Arbeitnow, Jobicy, Adzuna and five direct boards: Cloudflare/Bitwarden Greenhouse, Linear Ashby, SEP Lever and NVIDIA Workday. Fast-food uses only Adzuna and The Muse. **Adzuna is disabled by default in all three profiles**: configure App ID/App key in **Settings > System > Job providers**, then enable it explicitly. Credentials remain local and are not part of YAML or exports.

Coverage is bounded, not exhaustive. The Muse's remaining fast-food coverage does not establish current Cherry Hill vacancies. Arbeitnow focuses on Germany and does not establish original publication dates, so unknown dates cannot satisfy the engineering window. Jobicy establishes remote work but exposes a seven-day feed delayed three hours; public responses are cached for an hour. Adzuna descriptions are truncated, predicted pay is excluded, and bare native salary fields do not prove an annual interval. Local Rust rules, not provider queries, decide acceptance. Zero matches may be correct.

Google/Apple static sources remain **Needs review** and Netflix/Amazon script sources remain **Setup required**, independently available from the company picker but not attached to runnable built-ins. Selecting all sources never grants script trust or claims unavailable coverage. New custom role-first workflows retain their existing The Muse/Arbeitnow defaults.

**Replace test workflows** offers the bundled 48-hour test workflow or the three starter templates. Exact recognized shipped examples are selected by default; custom or modified workflows remain preserved unless selected. Review removals and targets before confirming. The test workflow is installed disabled; loading never launches a scrape or enables Adzuna. Saved jobs, notes, favorites, tracking/history and suppression are retained. Replacement reports the recovery backup path; never restore a full backup automatically over newer user data. Existing startup does not automatically replace old examples.

Legacy recognition specs and recorded provider verification profiles are frozen separately from this production catalog. Recorded route hashes, script fingerprints and expected provider outcomes do not change when user-facing profiles change. See the [workflow contract](../plans/workflow-v3.md) for canonical matching semantics.
The standalone [`software-engineering-discovery-test.yaml`](../test-workflows/software-engineering-discovery-test.yaml) retains the unfiltered The Muse feed and adds Computer and IT, Data and Analytics, and Data Science category-scoped samples. These provider-defined categories are not an exhaustive mapping of engineering titles. All sources share the 40-job target and apply US, original-publication and literal title requirements locally. Page/time bounds or insufficient eligible inventory can leave the target unmet. The Terminal console shows each evaluated candidate and its actual decision.


## Software Engineering crawler test

[`software-engineering-web-scraper-test.yaml`](../test-workflows/software-engineering-web-scraper-test.yaml) is a standalone development/test workflow, not a generated built-in and not automatically seeded. It runs only the bounded public-HTML crawler/scraper against Palantir and SEP career pages; original posting time must be within 24 hours and location must be evidenced as US. A run can add at most 40 accepted jobs, not a guaranteed 40 and not a limit on existing Discovered rows. Import or replace the intended saved workflow explicitly.

Workflow Preview uses a transient six-navigation cap per public-HTML source, within its existing 20-second source/45-second workflow deadline. This is a quick representative preview; it does not change the saved 50-page source configuration or normal runs. Preview breakdown reports the termination reason, and a coverage-limited result is marked sampled.

Both sources set `ignore_robots: true` for testing/development only. This bypass does not change worker defaults or other workflows. Keep it limited to this fixture workflow; normal custom workflows retain their configured robots policy.

## Public HTML sources and proxy

Custom workflows can use **Website crawl** to start from an employer's public HTML URL and follow a bounded, same-origin set of links. Choose JSON-LD or explicit CSS/XPath fields; page/depth limits, per-host pacing and `robots.txt` checks apply. Preview is read-only, and inaccessible/login-gated pages remain unsupported rather than being bypassed.

An optional HTTP(S) proxy is configured under **Settings > System > Network proxy**. Credentials are write-only and stored in the local OS credential store; they are never part of workflow YAML or exports. HTTP and app-launched Chromium extraction use the configured proxy. Basic credentials over an HTTP proxy are not transport-encrypted; prefer HTTPS or a trusted local network. Unset the proxy to restore direct routing. Only use a proxy you control or are authorized to use.

## Script setup

Netflix/Amazon scripts are embedded at compile time from `crates/scrape-worker/examples/extractors/`. Independent company presets carry the actual code using custom script protocol 2; no repository files or working directory are needed. Executable fingerprint review remains local; code, arguments or interpreter edits invalidate trust.

1. Open each source's custom extraction settings and enter an **absolute path to an installed Node.js 18+ executable**. The desktop app does not download a runtime or search a developer shell for `node`.
2. Check the environment, then save the workflow disabled for review.
3. Reopen and explicitly review/trust each saved executable source. Trust does not enable anything.
4. Enable the sources/workflow explicitly, Preview, inspect diagnostics, then Save. Run now is a separate action.

Imported file scripts must use an absolute path to a readable regular file (maximum 1 MiB), or carry embedded code. Missing files/runtime produce setup errors. Existing saved configuration is preserved; a legacy relative path or `node` command must be configured locally before execution. Imports never carry trust. Scripts run with local privileges; process isolation is not an OS sandbox.

Browser demonstrations and recorded provider fixtures are labelled and are not live extraction evidence. Required execution proof uses recordings/local fixtures, temporary databases, isolated credential stores and cache directories. Fixture success does not establish current Cherry Hill availability; live Adzuna requires personal credentials.
