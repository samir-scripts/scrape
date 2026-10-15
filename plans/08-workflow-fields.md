# Workflow v3 field ownership

The Rust `WorkflowSpec` and canonical validator remain authoritative. TypeScript
contracts in `src/bindings` are generated from Rust. `useWorkflowDraft` stores the
entire specification, not a lossy projection of visible fields. Source editing
uses `sourceToWorkflowSpec` and the stored workflow snapshot; save/preview use the
same canonical API. YAML exposes the complete supported representation.

| Specification fields | Form owner / presentation |
|---|---|
| `version` | Backend validates/migrates supported versions; YAML preserves the explicit version. Unsupported versions fail. |
| `metadata.name`, `company`, `enabled` | Source name, advanced company hint, Schedule enabled switch. |
| `extractor.kind`, `url` | Source detection / advanced adapter selection. |
| `extractor.job_container`, `title_selector`, `url_selector`, `location_selector`, `salary_selector`, `description_selector`, `date_selector` | Source advanced selector fields. |
| `extractor.title_regex`, `link_regex`, `location_regex`, `salary_regex`, `url_pattern` | Source advanced pattern fields. |
| `extractor.source_config`, `url_patterns`, `exclude_url_patterns` | Expert YAML; preserved when editing other fields. |
| `filters.include_keywords`, `exclude_keywords`, `role_types`, `locations` | Legacy/global matching overrides; locations has a form control, all remain available in YAML. Matching shows inheritance explicitly. |
| `filters.query` | Matching advanced expert query; parsed by the desktop validator. |
| `filters.policy`, `compensation`, `experience_min`, `experience_max` | Legacy/expert YAML; preserved without converting to competing rules. |
| `filters.matching.keywords.any`, `all`, `exclude`, `fields` | Matching title keywords, advanced all/exclude and field checkboxes. |
| `filters.matching.compensation` | Advanced native salary interval, cents, currency and unknown-value controls. |
| `filters.matching.role_types`, `workplaces` | Advanced role/workplace controls. |
| `filters.matching.experience_min`, `experience_max`, `experience_include_unknown`, `new_grad` | Advanced experience/new-graduate controls. |
| `filters.matching.sponsorship`, `work_auth`, `clearance` | Advanced eligibility controls. |
| `filters.matching.countries`, `regions` | Advanced geographic eligibility fields. |
| `filters.matching.location_include_unknown` | Expert YAML; retained through form edits. |
| `filters.matching.posting_date` | Advanced mutually exclusive relative age/date range, timezone and unknown-date inclusion. |
| `schedule.cadence`, `time`, `timezone` | Schedule controls. Manual and Daily are direct choices; inheritance is explicit. |
| `schedule.weekdays`, `cron` | Advanced Schedule / YAML. |
| `retention.days`, `policy` | Collapsed Schedule retention; expert YAML policy. All execution policies protect tracked/annotated records. |

Absent global filter overrides inherit defaults. Explicit empty collections mean
no restriction and must not be replaced with defaults. New workflows default to
09:00 daily in the detected device timezone. Existing inherited schedules retain
inheritance; global values are shown separately and are not silently copied into
per-workflow configuration. Truly global controls remain in Scraper settings.

Each change invalidates preview eligibility and cancels an in-flight preview.
Navigation alone preserves the current configuration. Backend validation guards
Review and preview; only a current successful preview enables Save, including a
successful zero-match preview. Preview and Save do not persist discovered jobs,
apply retention or trigger another scrape. Run now is a separate explicit action.

Browser fixtures are visibly labeled and are not live-provider verification.
Recorded-example mode rejects all execution APIs, including saved/transient Test.
Expert syntax requires desktop validation. No new workflow schema or migration was
introduced for the wizard.
