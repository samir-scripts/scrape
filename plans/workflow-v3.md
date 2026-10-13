# Workflow specification v3

The desktop validator is authoritative for create, edit, import and preview. Rust contracts generate the TypeScript representations in `src/bindings`. Versions 1 and 2 migrate explicitly to version 3; unknown fields and unsupported versions are errors with paths.

The editor holds one draft. Simple creation follows Source → Matches → Schedule and save. Advanced fields and YAML share that draft. Configuration edits invalidate and cancel the previous preview. Saving never runs a scrape; the success state offers an explicit Run now action.

New editor workflows default to daily 09:00 in the device IANA timezone. Calendar scheduling uses that stored timezone. Manual workflows have no automatic next run. Global pause stops automatic dispatch; manual execution remains available.

Absent matching properties inherit global defaults, and explicit empty arrays remove inherited restrictions. Enriched `filters.matching` supports keywords (`any`, `all`, `exclude`, explicit fields), original compensation ranges, role/workplace/experience/eligibility/geography, and posting dates. Relative ages use one run-start instant. Date-only boundaries use the date filter's timezone, with the final date inclusive through the next midnight. Unknown posting dates are excluded unless requested.

Preview uses the same effective filters and worker as execution, checks posting blocks and returns the first ten eligible samples with stage counts. It has a bounded worker lifetime and cancellation, and writes no jobs, retention changes or scheduling state. Provider failures remain visible as errors; reported counts describe the data actually fetched, not an assumed complete board. Browser demonstration previews use fictional fixtures and cannot verify date/eligibility extraction or expert rules.

YAML is decoded with the existing YAML library and sent to the backend validator. Form/YAML serialization preserves supported hidden configuration and source metadata. The source configuration object accepts only implemented adapter fields. Retention always protects tracked jobs, favorites and nonblank notes.

Validation: populated repository/worker matching parity tests, canonical validation tests, DOM editor tests (zero results, no implicit run, YAML/native pay preservation, stale preview cancellation), and the frontend suite. Browser and native verification are recorded separately in the implementation plan.
