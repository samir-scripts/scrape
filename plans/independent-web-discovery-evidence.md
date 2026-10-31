# Independent public-web discovery evidence

## Scope and limits

The requested search is US SWE/IT jobs originally posted within 48 hours, with a shared target of 20 newly inserted jobs. Bounded crawling cannot establish complete web coverage. Static HTML only; JavaScript-only pages, inaccessible or denied sites, unknown dates and unresolved geography remain gaps. Rounded relative publication dates are not converted into exact timestamps. Crawl verification used isolated temporary databases; the separately authorized read-only database inspection is described below.

## Deterministic integration checks

`cargo test -p scrape-worker --test web_discovery_process` passed four process
tests: an unrelated directory reaches two fictional employer details; shared
career pages retain separate requisitions; and a career index is not a job
detail. A direct-email detail with unknown date and geography stays unresolved;
conflicting dates, old/unknown dates, foreign locations, robots denial and
private redirects are fixture cases, not live coverage.
`cargo test -p scrape --test worker_repository` passed all 33 tests after the
protocol-version updates. Its new two-source discovery fixture used actual
packaged-worker commit feedback and temporary SQLite: 20 new inserts counted
against the shared target, while an update and a blocked tombstone did not;
notes/favorite and per-run provenance were retained. These are fictional
network fixtures, not live-job evidence.

## Live worker smoke — 2026-10-31

A packaged worker was launched directly, without boards or API sources, against `https://en.wikipedia.org/wiki/Category:Software_companies_of_the_United_States` with strict US and 48-hour filters. The observed run ended at 07:47 UTC after attempting 80 pages and 25 origins. It emitted no qualifying jobs, examined two candidates and reported a page limit and partial coverage. Robots redirects, DNS failures, 403/404 responses, unsupported sitemap content and oversized sitemaps were reported. The run reached employer-domain leads, but this worker-only smoke did not prove native host/SQLite ingestion, a current 48-hour match, or a non-ATS direct-application case. The earlier direct Elite Hockey HQ smoke found an employer-hosted software engineer page dated 2026-09-06 with unknown location; it was correctly rejected, and its application link led to a board. Neither result measures global coverage.

At 07:56 UTC a second worker-only run started from
`https://gainam.com/careers-gainam`, discovered six matching software/IT
listings and reported 25 pages, one origin, 22 examined and 16 role rejections.
Each accepted detail exposed `2026-10-30` as both visible Date posted and
JSON-LD `datePosted`; all six detail pages were independently reread. Gain
America describes itself as a staffing partner, not the ultimate employer;
open status and absence from boards are not proven. This was a manually
researched seed, not a company-directory proof or a persisted run. A later
native-host attempt failed before fetching because host-only `__cron_timezone`
metadata was rejected by the worker adapter. Its profile was isolated.

A separate debug-worker run from `https://endla.com/careers/` followed the
employer's public career links and examined Backend and Frontend Software
Engineer detail pages offering application by email. It left company unknown and
reported date and US geography as unresolved, so neither role was accepted.
Both slash and non-slash URL variants were examined, producing four unresolved
events for two titles. These are not 48-hour US matches.

The worker now ignores host-only metadata when constructing adapter options.
At 08:22:55 UTC, `node scripts/native-independent-live.mjs --seed
https://gainam.com/careers-gainam` executed the unchanged US/48-hour filters
and 20-new-job quota through the actual native host, packaged worker, commit
feedback and temporary SQLite. It inserted 9 distinct jobs, rejected 13 of
22 examined candidates, attempted 25 pages on one origin and exhausted the
source. All nine retained `2026-10-30` original-date observations; each visible
date was independently checked. The three added matches versus the earlier
worker smoke are DevOps roles covered by the full workflow's role list.
The run ended `success_with_warnings` because the employer's GIF logo is not
supported. SQLite integrity was `ok`; nine observations refer to nine jobs.
Artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-independent-live-njG2L4`.
These staffing-site roles do not establish the ultimate employers or whether
they also appear on boards.

`npm run test:native-web-scraping` also passed with an isolated directory →
fictional employer fixture, real host/worker and SQLite. The preceding native
targeted-workflow journey passed its starter/test replacement controls.

The unmodified canonical profile was separately run through the isolated
native host at 08:24:53 UTC. It inserted zero jobs and ended `partial`: the
directory source reached 100 attempted pages / 25 origins and the Common
Crawl sample reached 100 attempted pages / 17 origins. Together they examined
35 candidates and rejected all 35. The directory's advertised sitemap returned
403; both sources terminated at their page limits. The profile did not find the
manually seeded Gain America roles. Artifacts:
`/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-independent-live-tApx5d`.

## Reference-site access sample — incomplete gap assessment

At 2026-10-31T07:33:18Z, 22 career surfaces were selected from employer-domain leads in the CC-MAIN-2026-43 URL-index shard 00060 plus two shared-career pages from the checked-in sample. Selection occurred before the Wikipedia directory run above but after the earlier direct-seed smoke, so this is not a fully blind sample. The corresponding 48-hour cutoff was 2026-10-29T07:33:18Z. The access-check output is `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-web-sample-20261031T073323Z.json` (temporary artifact). Seventeen pages returned accessible HTML after the preliminary robots check, one was robots-denied, and four returned HTTP errors. These were surface-access checks, not an enumeration of open, dated US SWE jobs; the existing feed/ATS baseline was not measured. A sample-relative miss rate cannot be calculated.

Selected surfaces: elitehockeyhq.com, eliteperformancetech.com, ellipsishealth.com, ellisys.com, elogicsquare.com, career.elva-group.com, careers.elvyenergy.com, emaancommerce.com, careers.embark-studios.com, embeddedts.com, embitel.com, careers.emdgroup.com, jobs.emed.com, external.emiratesgroupcareers.com, empirevets.com, enappsys.com, enco-consulting.com, encoreautomation.com, endla.com, endpointassociates.com, myturningpt.com and sanoh-america.com. Employer ownership, direct application, and ATS absence were not established for every surface. Some selected career paths returned 404, and robots denied the Embitel path; neither is evidence of an absence of jobs.

Later manual checks were incomplete and cannot support a miss rate. For example,
[eMed's Miami SRE opening](https://jobs.emed.com/apply/N46MBdTxRG/Senior-Software-Engineer-SRE)
has no observed original publication date, while
[Endla's backend role](https://endla.com/careers/backend-software-engineer-python/)
offers direct application by email but does not establish US eligibility or a
posting date. Neither can be counted as eligible under the strict filter.
The selected Empire Vets and Enco paths returned 404; selected Ellisys and SAIC
reads returned 403; and some sites require rendering or login. These failures
are not evidence that the employers have no jobs.

## Database migration incident

A local development version of migration 34 changed while Scrape was running.
The application reported a checksum mismatch and refused startup. Commit
`0e871c4` restores a recognized immutable history and adds migration 35
without rewriting checksums. The populated upgrade module passed three tests,
including recognized v34 variants, rollback, restart and state preservation.
An independently created temporary v34 database was run through the
`repair_database` example; it returned `REPAIR_OK migration_version=35
integrity=ok`, created a backup, and retained its sample note, favorite,
interviewing status and observation with no foreign-key violations.

After Scrape and its watcher were stopped, an authorized read-only inspection
of a private copy of the database and WAL showed version 34's early-index
checksum and the canonical version 35 checksum; integrity was `ok` and the
foreign-key check was empty. The existing pre-migration-35 backup and current
copy both had 0 jobs, 4 workflows, 22 sources and 3 runs. Seventeen other table
contents were compared: only `app_settings` differed, specifically
`app.last_update_check` and `data.retention_status`. A version-32 backup had
20 jobs; counts alone do not establish why those jobs were absent by version 34.
The watcher is the likely source of the version-35 retry, but the migration was
not directly observed. No manual write or restoration was performed on the live
database.
