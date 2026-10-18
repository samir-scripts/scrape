# Workflow specification compatibility and current v6 contract

Current canonical exports use **v6**. Rust accepts versions 1–5 through the same
compatibility resolver; the v3/v4/v5 descriptions below record the behavior those
versions introduced. Existing storage is preserved until a normal save.

The desktop validator is authoritative for create, edit, import and preview. Rust contracts generate the TypeScript representations in `src/bindings`. Versions 1 and 2 migrate explicitly to version 3; unknown fields and unsupported versions are errors with paths.

The editor holds one draft. Simple creation follows Source → Matches → Schedule and save. Advanced fields and YAML share that draft. Configuration edits invalidate and cancel the previous preview. Saving never runs a scrape; the success state offers an explicit Run now action.

New editor workflows default to daily 09:00 in the device IANA timezone. Calendar scheduling uses that stored timezone. Manual workflows have no automatic next run. Global pause stops automatic dispatch; manual execution remains available.

Only legacy versions 1–3 resolve omitted keyword/location/role properties from global defaults at import or historical upgrade; explicit empty arrays remove inherited restrictions. Versions 4–6 and every saved execution are independent of global matching defaults. Enriched `filters.matching` supports keywords (`any`, `all`, `exclude`, explicit fields), original compensation ranges, role/workplace/experience/eligibility/geography, and posting dates. Relative ages use one run-start instant. Date-only boundaries use the date filter's timezone, with the final date inclusive through the next midnight. Unknown posting dates are excluded unless requested.

Simple Matching exposes searchable Country and dependent State/province controls plus Remote/Hybrid/In-person workplace choices. Countries use ISO alpha-2 codes; newly selected subdivisions use qualified codes such as `US-CA` or `CA-ON`. Geography and workplace apply together, selected workplaces match with OR, and country/subdivision must match the same location entry. Changing country clears incompatible subdivisions. Imported multiple-country rules and legacy short subdivision codes remain intact until explicitly edited. The generated [catalog](../crates/scrape-core/data/README.md) has all 249 countries but a bounded subdivision list; Advanced retains raw codes for other subdivisions. Additional free-text location restrictions live in Advanced and still apply with AND.

Unknown geography is excluded when a geographic filter is active unless explicitly included. A bare Remote row does not override known contradictory geography elsewhere in the posting. Inclusion permits missing fields only on otherwise compatible locations; a qualified subdivision also provides country evidence for partial legacy rows. Malformed country/subdivision filters and entirely incompatible qualified country/subdivision combinations fail validation. Worker matching and SQL compilation share these rules.

Extraction prioritizes structured address country/subdivision fields over text inference, with bounded aliases and word boundaries. JSON-LD keeps every location-array entry and nested Country values; for remote roles explicit `applicantLocationRequirements` takes precedence over physical office geography, followed by labeled eligibility sections. General company prose does not establish job eligibility. Unsupported or ambiguous geography remains unknown.

`max_jobs_per_run` is an optional top-level whole number from 1 to 10,000, defaulting to **20** for new and existing workflows. It counts unique matching, valid, unblocked postings, including existing records refreshed by a run. Each workflow in Run all has its own budget. This is not a total saved-job capacity limit and does not remove existing jobs. The worker stops candidate processing and further detail/page requests when its budget is reached; downloading a provider's single complete-board response can be unavoidable. Matching follows provider order, so repeated runs may refresh the same first postings. Source summaries explicitly identify the limit and whether processing stopped there; totals describe examined records only.

Preview and Test use the same effective filters, block snapshot and worker acceptance pipeline, capped at the smaller of `max_jobs_per_run` and **10**. Preview has a bounded worker lifetime and cancellation, and writes no jobs, retention changes or scheduling state. A limit-reached result is labeled as potentially incomplete source coverage, rather than a complete-board count. Provider failures remain visible as errors. Browser demonstration previews use fictional fixtures and cannot verify date/eligibility extraction or expert rules.

Compatibility retains legacy keyword, location, role, expert query/policy and
compensation predicates alongside structured rules. Every active group applies
with AND. Legacy includes use OR across terms and title/location; excludes reject
matches in either field; location terms use OR on original location text. SQL LIKE
wildcards retain their meaning. Independent legacy and structured role/keyword
sets are never unioned, locations are never inferred into workplace/geography,
and queries from old samples are never removed by matching their text. Simple
Matching shows the retained restrictions, with Rust-owned explanations from
`describe_workflow_filters`. Users may edit/remove them explicitly (individual
legacy roles/compensation remain editable in YAML); these edits invalidate preview.
There is no automatic “migrate to structured rules” action. This restores the
existing v6 predicate contract without changing wire/spec versions or stored data.
It cannot reconstruct restrictions already discarded by an earlier unsafe save;
those require review against a known original/backup, never guessed restoration.

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

## Static custom locators (Section 6A)

A v5 source with `extractor.kind: selector` accepts these settings inside
`extractor.source_config` (the top-level extractor contract is unchanged):

```yaml
item: "//article[contains(@class, 'job-card')]"
title: "xpath:.//h2/text()"
title_regex: "Role: (?P<value>.+)"
link: "a.apply@href"
id: "."
id_attribute: "data-job-id"
location: "xpath:.//h2/following-sibling::span[1]"
date: "time"
date_attribute: "datetime"
description: "p.description"
description_attribute: "text"
```

`item` selects elements. Each posting field (`title`, `link`, `id`, `location`,
`salary`, `date`, `description`) accepts CSS or `xpath:` location paths; `/`, `//`,
`./`, `.//`, `(/` and `(//` also identify XPath. `.` selects the current container.
CSS shorthand `a@href`, explicit `<field>_attribute`, or terminal XPath `/@href`
extract attributes. `text` extracts text and `html` extracts serialized markup.
Defaults are text except link (`href`), description (`html`), and date
(`datetime` if present, otherwise text). Original IDs default to the destination
URL when no ID locator is configured. A configured ID is required to produce a
nonempty value.

Every posting field supports `<field>_regex`. A named `value` capture takes
priority, followed by legacy field-named captures, the first numbered capture, then
the whole match. Regex mismatches on title/link/configured ID emit an invalid
candidate; optional mismatches stay unknown. Regex-only optional fields retain
full-container text extraction. Plain-text descriptions retain literal tags.

Supported XPath subset: child `/`, descendant `//`, self `.`, parent `..` or
`parent::tag`, `following-sibling::tag`, tag/wildcard/text node tests,
`[@attr]`, `[@attr='value']`, `contains(@attr,'value')`,
`starts-with(@attr,'value')`, `contains(text(),'value')`,
`[normalize-space()='value']`, `[1]`/other nonnegative integers and `[last()]`.
Grouped positional paths are supported: `//a[1]` selects the first matching
child per parent; `(//a)[1]` selects the first match overall. Attribute tests are
case-sensitive. Paths use the parsed HTML5 tree, including implied `html`, `body`
and `tbody` elements. Unsupported expressions, namespaces, unions and functions
fail explicitly; this is a subset of [XPath 1.0](https://www.w3.org/TR/1999/REC-xpath-19991116/).
The shared parser enforces 4096 bytes, 64 steps and 32 predicates per step.

Static locators, hydration and resumable pagination/detail enrichment are implemented.
Browser sessions and executable scripts remain pending in [the Section 6 blueprint](11-section-6-browser-and-script-extraction.md).

### Embedded hydration JSON

Selector sources can read a JSON array embedded in a script without launching a
browser. Configure `hydration_pointer` (an explicit RFC 6901 pointer to that array)
and `hydration_fields` (JSON pointers relative to each record):

```yaml
hydration_script: "#__NEXT_DATA__"
hydration_pointer: "/props/pageProps/jobs"
hydration_fields:
  title: "/name"
  link: "/applyUrl"
  id: "/requisitionId"
  description: "/description"
  salary: "/compensationText"
  date: "/published"
title_regex: "Role: (?P<value>.+)"
description_attribute: text
```

Omitting `hydration_script` selects `#__NEXT_DATA__, #__NUXT_DATA__` in document
order. A custom CSS/XPath locator must select a script element. JSON pointers use
`~0` for `~` and `~1` for `/`, as specified by [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901).
Title and link mappings are required. Other supported mappings are ID, location,
salary text, publication date and description. Numeric IDs are preserved as
strings; other fields require strings. Every field supports the existing Rust
`<field>_regex`. Descriptions default to HTML; `description_attribute: text`
preserves literal tags. DOM posting locators and JSON field mappings cannot be
mixed. Destination URL patterns and authoritative matching still apply.

Missing scripts, malformed JSON and pointers that do not select an array fail
explicitly. Invalid candidates are isolated. No script code is executed, and
flattened/reference-encoded hydration formats are not decoded automatically.
Unknown compensation/date values remain unknown. Hydration uses the same page/detail coordinator when explicitly configured.

## Static pagination and details

Add static navigation and detail extraction inside `extractor.source_config`:

```yaml
next_page: "xpath:(//a[@rel='next'])[1]/@href"
max_pages: 20
max_source_duration_s: 60
wait_for: "main.jobs"
detail: "a.details@href"
detail_container: "main"
detail_fields:
  description: ".description"
  description_attribute: text
  salary: ".pay"
  salary_regex: "Pay: (?P<value>.+)"
  date: "time"
  date_attribute: datetime
```

`next_page` resolves relative destinations against the actual fetched page and
prevents URL/redirect cycles (fragments are ignored). `wait_for` verifies static
HTML presence; it does not execute JavaScript or wait for DOM changes. Load-more
clicking requires the browser mode planned in 6C and fails explicitly in static
mode. Missing/unsafe next-page destinations and bounded or failed coverage retain
already processed jobs with an actionable partial outcome.

`detail` selects a per-candidate detail URL. Alternatively use
`follow_detail: true` to fetch the original candidate destination (also supported
by hydration sources). Details require explicit `detail_fields` for location,
salary, description and/or publication date; field attributes and Rust regexes
work identically to listing extraction. `detail_container` optionally scopes those
locators. Title, ID, company and destination cannot be changed by detail fields.
Only missing evidence is filled; valid listing evidence remains authoritative.
Details for known rejects and duplicate destinations are skipped. Missing required
evidence produces partial coverage; optional enrichment failures are warnings.

Default/hard page bound is 100 (source range 1–100). Source duration defaults to
180 seconds (range 10–600); workflow duration defaults to 600 (range 30–1800).
Worker limits cap source overrides. Navigation defaults to 30 seconds (range
5–120), with HTTP client timeout, robots/pacing and the existing 100-request /
45-second detail allowance retained. No page/detail work is scheduled after the
shared matching cap. Exhausted sources, page/time limits, loops and job limits
have distinct diagnostics. Browser/session/script execution is still pending.

## Typed extraction modes and local trust (Section 6B)

A v6 `selector` extractor accepts `mode: static | browser | script`, defaulting to
static. YAML aliases `kind: browser` and `kind: script` normalize to selector plus
mode; SQLite source kinds stay unchanged. Existing `source_config` vocabulary is
preserved. Typed field selectors, attributes and regexes map to those same Rust
contracts, including `url_selector`/`url_attribute` → `link`/`link_attribute`, and
navigation selectors → `detail`, `next_page`, `load_more`, `wait_for`. Bounds are
validated by shared `WorkerLimits`; source overrides cannot raise worker limits.

Browser and script settings may hold a `session_id` reference. Raw cookies,
headers, authorization, storage_state and tokens are rejected, including inside
`source_config`. Sessions and executable trust never travel as YAML authority.
Browser/session and script execution are unavailable until 6C/6D and fail
explicitly; static HTTP extraction continues to run normally.

A script requires `runtime: js | python | custom`, exactly one of `code` or `path`,
and an explicit interpreter for Python/custom. Arguments remain an array. Code
and file contents are bounded to 1 MiB; there are at most 128 arguments and the
script timeout range is 1–600 seconds (still subject to source/workflow limits).
The stable SHA-256 covers runtime, interpreter, code, path and arguments, plus
file contents for path scripts. Missing local files may be imported/saved for
review but cannot be trusted or executed.

`import_workflow` returns scripts disabled and disables automatic workflow
execution; an explicitly disabled workflow may contain no enabled sources so it
can be saved for review. Execution requires an enabled source. Saves preserve
local trust only for an unchanged fingerprint and reset it on import or edit.
`trust_workflow_script` requires local confirmation and the exact reviewed hash;
custom code runs with local user privileges, and process isolation is not an OS
sandbox. Trust alone never enables a source or starts a run. Preview, transient
Test, manual and scheduled runs pass the same trust/content check before any
subprocess launch. Browser demonstration mode cannot verify these extractors.


## Field evidence and script protocol 2 (Plan 12 Section 4)

The host/worker wire protocol remains 7 and WorkflowSpec remains 6. Custom scripts
now use **SCRIPT_PROTOCOL_VERSION = 2** for every input/output frame. Version 1
output is rejected with the expected version; it is not silently upgraded. Scripts
must emit only documented frame fields (`salary`, not `salary_text`); unknown
fields, including claimed acceptance, fail explicitly.

Candidates may supply `employment_type` (an ATS employment hint), `workplace_type`
(`remote`, `hybrid`, `onsite`, `unknown`), and `locations` (up to 64 `ParsedLocation`
tuples with raw evidence, optional city/ISO country/qualified subdivision, and
`is_remote`). Rust classifies hints, validates tuples, performs matching and owns
acceptance. The host revalidates the same bounds/country-subdivision consistency
before persistence. `raw` must be nonblank and at most 8192 bytes; city is bounded
to 1024 bytes. A physical address alone no longer implies onsite attendance.
Concrete workplace/eligibility evidence does not require an unrelated description;
missing descriptions remain missing for description keyword/SQL rules. Unresolved
required fields retain partial diagnostics, and explicit unknown policies remain
authoritative after enrichment is exhausted.

Lever's `country` disambiguates only its associated primary location. For example,
`Westfield, IN` plus `US` yields city Westfield and US-IN while retaining the raw
label; secondary locations do not inherit another location's country. Ashby primary
postal addresses and both documented/captured secondary address forms are used
before free text. Explicit workplaceType takes precedence over the older isRemote
flag. Continents and unrecognized country labels remain unknown; Greenhouse custom
metadata is not interpreted as employment without captured field semantics.

Geography checks every requested dimension on one tuple. Parsed city names use
exact ASCII-case-insensitive comparison after trimming; original raw text on that
same tuple also permits a literal substring. `%`/`_` are literal city characters.
Non-ASCII spelling is preserved, with no transliteration or Unicode case folding;
query trimming and blank-value handling include Unicode whitespace consistently
in Rust and SQLite. Raw-only city evidence can match when no structured geographic
tuple is present. Unknown tuples cannot bypass contradictory known tuples, and
missing-city inclusion follows `location_include_unknown`. Neither another tuple's
text nor the aggregate job location string can satisfy a tuple's city predicate.

The bundled Netflix/Amazon examples speak version 2. Netflix maps only observed
work-location options (`onsite`, `remote`, `remote_local`, `hybrid`); this establishes
arrangement, not residence or authorization. Editing scripts/files invalidates
local trust through the existing content hash. Users must update/review their own
version-1 scripts and explicitly trust changed content; saved workflows are never
rewritten or enabled automatically. Pagination/date/portable-runtime repairs remain
Plan 12 Sections 5–7 acceptance work.

Required unresolved matching evidence emits at most five contextual examples per
source, including requested country codes and original location text. The existing
eligibility_incomplete warning flag distinguishes these partial-coverage diagnostics
from optional enrichment warnings in worker summaries and host run accounting.
All unresolved candidates remain counted even after the example bound is reached.
