# Independent public-web discovery evidence

## Scope and limits

The requested search is US SWE/IT jobs originally posted within 48 hours, with a shared target of 20 newly inserted jobs. Bounded crawling cannot establish complete web coverage. Static HTML only; JavaScript-only pages, inaccessible or denied sites, unknown dates and unresolved geography remain gaps. Explicit rounded minute/hour/day labels are stored as observed intervals rather than invented exact timestamps. Conflicting structured and visible date text is conservatively excluded, even where the intervals may overlap. Crawl verification used isolated temporary databases; the separately authorized read-only database inspection is described below.

## Deterministic integration checks

`cargo test -p scrape-worker --test web_discovery_process` passed five process
tests: an unrelated directory reaches two fictional employer details; shared
career pages retain separate requisitions; and a career index is not a job
detail. A direct-email detail with unknown date and geography stays unresolved;
conflicting dates, old/unknown dates, foreign locations, robots denial and
private redirects are fixture cases, not live coverage. Rounded relative dates
are checked through the worker; an ingestion and SQLite query regression checks
interval boundaries and retained observation evidence.
`cargo test -p scrape --test worker_repository` passed all 33 tests after the
protocol-version updates. Its new two-source discovery fixture used actual
packaged-worker commit feedback and temporary SQLite: 20 new inserts counted
against the shared target, while an update and a blocked tombstone did not;
notes/favorite and per-run provenance were retained. These are fictional
network fixtures, not live-job evidence.

The full `cargo test --workspace` run passed 739 tests with one ignored.
`npx tsc --noEmit`, a complete serial `npx vitest run --maxWorkers=1` (588 tests),
`node scripts/test-app-startup.mjs`, `npm run build` and
`node scripts/check-style.mjs` passed. The production build retained its
large-chunk advisory. A Chromium fixture-mode session loaded the disabled test
profile and displayed the independent source, seed import and traversal bounds;
the screenshot was reviewed in-session but not saved. Fixture browser behavior
does not prove native file-chooser, physical pointer, or live provider behavior.
A second isolated Chromium fixture session displayed `1 day ago` with its
observation time on a Discovered card and opened the job inspector; its
screenshot was reviewed in-session, not saved. The data was synthetic.
`cargo clippy --workspace --all-targets -- -D warnings` and the recorded
`npm run test:providers` matrix also passed.

## Live discovery runs — 2026-10-31

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

At 2026-10-31T07:33:18Z, 22 career surfaces were selected from employer-domain leads in the CC-MAIN-2026-43 URL-index shard 00060 plus two shared-career pages from the checked-in sample. Selection occurred before the Wikipedia directory run above but after the earlier direct-seed smoke, so this is not a fully blind sample. The corresponding 48-hour cutoff was 2026-10-29T07:33:18Z. The access-check output is `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-web-sample-20261031T073323Z.json` (temporary artifact). Seventeen pages returned accessible HTML after the preliminary robots check, one was robots-denied, and four returned HTTP errors. These were surface-access checks, not an enumeration of open, dated US SWE jobs. The later capped feed/ATS baseline is described below; it cannot establish a sample-relative miss rate.

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

### Follow-up adjudication of the selected surfaces

The selected sample above was revisited by independent public-page reads between
approximately 08:50:48 and 08:51:21 UTC on 2026-10-31; its comparison cutoff
remains **2026-10-29T07:33:18Z**, not a new moving cutoff. These later reads
are snapshots, not proof a role was open at the original selection time. The
following is an assessment of the selected surface and linked examples, not
an exhaustive enumeration of each employer's entire careers system. “No
observed date” means no original job-publication date was seen in the
inspected page text; it does not prove no date exists elsewhere.

| Selected surface | Observation and adjudication |
| --- | --- |
| [Elite Hockey HQ](https://www.elitehockeyhq.com/careers) | Accessible; index says remote roles are open to US applicants and lists seven engineering positions. Employer details for [senior software](https://www.elitehockeyhq.com/careers/senior-software-engineer), [backend](https://www.elitehockeyhq.com/careers/backend-engineer), [frontend](https://www.elitehockeyhq.com/careers/frontend-engineer), [full stack](https://www.elitehockeyhq.com/careers/full-stack-engineer), [mobile](https://www.elitehockeyhq.com/careers/mobile-engineer-ios-android), [AI](https://www.elitehockeyhq.com/careers/ai-engineer), and [QA](https://www.elitehockeyhq.com/careers/qa-engineer) were readable with no visible publication date. Each links to a different LandEarly application; the [senior application](https://www.landearly.com/companies/elite-hockey/jobs/senior-software-engineer-44450487) says posted October 25, before the cutoff. It is therefore not a 48-hour reference match; the remaining dates were not adjudicated and cannot be counted. |
| [Elite Performance Tech](https://eliteperformancetech.com/careers/) | Selected path returned 404 in the recorded access check; other paths not enumerated. |
| [Ellipsis Health](https://ellipsishealth.com/careers/) | Accessible “Current openings” shell, but the public read did not expose role listings. No conclusion about vacancies. |
| [Ellisys](https://www.ellisys.com/company/jobs/) | Recorded access returned 403; unknown. |
| [ElogicSquare](https://elogicsquare.com/jobs/) | Accessible index lists three software roles with 2025 dates, including [Software Developer I](https://elogicsquare.com/jobs/software-developer-i/) in Jacksonville, linked to April 16, 2025 on the index. That index date is outside the window, although open status is not independently established. Other listings likewise carry 2025 index dates; no recent reference role established. |
| [Elva](https://career.elva-group.com/) | Accessible; visible software/Cloud Architect roles name Stockholm, Gothenburg, Malmö and Cebu, with hybrid arrangements. These listed roles are not US-based; no full alternative-site enumeration. |
| [Elvy](https://careers.elvyenergy.com/) | Accessible; listed operations, customer success and finance roles, and identifies its HQ as Stockholm. No US SWE/IT role established from the page. |
| [Emaan Commerce](https://emaancommerce.com/careers/) | Accessible; [middle](https://emaancommerce.com/careers/senior-software-engineer-2/) and [senior](https://emaancommerce.com/careers/senior-software-engineer/) software pages contain apparent placeholder fish text, with page publication metadata from June 10, 2021. Actual vacancies, US work location and original job dates are unconfirmed; page metadata is not sufficient job-date evidence. |
| [Embark Studios](https://careers.embark-studios.com/) | Accessible; visible positions including technical roles identify Stockholm. The linked all-jobs list was not fully enumerated; no eligible US role established. |
| [embeddedTS](https://www.embeddedts.com/about/careers/) | Accessible; page explicitly says “No Current OPenings”; not independent proof about unlisted vacancies. |
| [Embitel](https://www.embitel.com/careers/) | Preliminary robots check denied the selected path. Not fetched as an eligible discovery target; unknown. |
| [EMD Group](https://careers.emdgroup.com/us/en) | US careers shell accessible, but [search results](https://careers.emdgroup.com/us/en/search-results) returned unresolved template placeholders in the public read. Neither no jobs nor a complete list can be inferred. |
| [eMed](https://jobs.emed.com/) | Accessible index lists three distinct Miami SWE roles: [Cloud](https://jobs.emed.com/apply/t9JRN5xoqC/Senior-Software-Engineer-Cloud), [Full Stack](https://jobs.emed.com/apply/OMNx5wWPbt/Senior-Software-Engineer-Full-Stack), and [SRE](https://jobs.emed.com/apply/N46MBdTxRG/Senior-Software-Engineer-SRE). Each detail is readable and invites applications, but none exposed an original posting date in the inspected text. Open-at-cutoff and 48-hour eligibility are indeterminate; URL IDs distinguish them. |
| [Emirates Group](https://external.emiratesgroupcareers.com/) | Recorded access redirected to a login page; not a usable public enumeration. |
| [Empire Vets](https://empirevets.com/careers/) | Selected path returned 404 in the recorded access check; other paths not enumerated. |
| [EnAppSys](https://www.enappsys.com/careers/) | Accessible; points to [Montel careers](https://montelgroup.com/careers), whose visible featured roles are European sales positions. Neither page establishes an eligible US role; the all-jobs list was not fully enumerated. |
| [Enco Consulting](https://enco-consulting.com/careers/) | Selected path returned 404 in the recorded access check; other paths not enumerated. |
| [Encore Automation](https://www.encoreautomation.com/careers/open-positions/) | Accessible shared listing shows Robotics Engineer and Project Engineer in Auburn Hills, Michigan. These titles alone do not establish SWE/IT eligibility, posting dates or an exhaustive listing. |
| [Endla](https://endla.com/careers/) | Accessible shared listing links [backend](https://endla.com/careers/backend-software-engineer-python/) and [frontend](https://endla.com/careers/frontend-software-engineer-react/) roles with direct email application. Both lack observed publication dates and US work eligibility; remote-first and Brisbane headquarters do not prove US eligibility. |
| [Endpoint Associates](https://endpointassociates.com/careers/) | Accessible staffing-company page, not the client employer's career page. [Software](https://endpointassociates.com/jobs/senior-software-engineer-opportunities/) and [cloud](https://endpointassociates.com/jobs/cloud-infrastructure-engineer-opportunities/) pages invite consideration for current *and upcoming* client searches, not a uniquely identified open requisition; visible September 10, 2026 page dates precede cutoff. Neither is a confirmed reference job. |
| [Turning Point](https://www.myturningpt.com/careers/) | Accessible shared page solicits clinical-license holders in North Webster; no SWE/IT opening observed. |
| [Sanoh America](https://www.sanoh-america.com/careers) | Accessible employer page links to [ADP current openings](https://workforcenow.adp.com/mascsr/default/mdf/recruitment/recruitment.html?cid=61043aea-34c2-440f-9a98-72040c4d937a&ccId=19000101_000001&lang=en_US&source=CC4&selectedMenuKey=CurrentOpenings); the public read exposed only a supported-browser notice. Open roles and dates remain unknown. |

An isolated prior-channel native baseline subsequently started at
2026-10-31T08:49:14.069Z with the same US/48-hour filters and 20-insert cap.
Its retained artifact is
`/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-independent-live-gnJ5Sq`:
partial status, eight sources, 1,203 fetched, 1,178 rejected, 20 inserted,
and page-limit/job-limit termination. It was not an uncapped reference
enumeration and its slightly later 48-hour cutoff is not identical to the
sample cutoff. The canonical discovery run above also stopped at page limits.
No individual reference-eligible sampled role has been established with all
required open-status, US, role and original-date evidence, so no
old/new/both/neither classification can honestly be made for these examples.
**A sample-relative miss rate is not measurable**, rather than zero: the
reference denominator remains unresolved, and the capped baselines would not
support a recall estimate even if some qualifying roles were identified. The
alphabetically clustered, partly post-smoke selection is not representative of
all employers or global web coverage.

## Repeat live comparison — 2026-10-31

Two independently selected convenience samples of ten employer career surfaces
each were frozen before either run's results were inspected. The institution
sample was recorded at 09:08:05.686 UTC; the company sample's first clock
reading immediately after selection was 09:08:05.692 UTC. Their approximate
48-hour cutoff was 2026-10-29 09:08:05 UTC. The samples are not representative:
the company set contains large organizations, several surfaces require
JavaScript, and none was exhaustively enumerated. The earlier sample above
includes smaller employers but used a different cutoff.

| Selected surface | Public-page assessment |
| --- | --- |
| [IBM](https://www.ibm.com/careers/search) | Search readable but paginated; a US software role had no observed original date. |
| [Microsoft](https://careers.microsoft.com/v2/global/en/home.html) | Public portal shell; individual current jobs and dates not established. |
| [Apple](https://jobs.apple.com/en-us/search) | US results contain unlabeled dates; the inspected detail returned a shell. Robots response was not usable. |
| [Google](https://www.google.com/about/careers/applications/jobs/results) | US software detail readable without an original date; pagination was not fully examined. |
| [NVIDIA](https://nvidia.wd5.myworkdayjobs.com/NVIDIAExternalCareerSite) | Public page returned a shell; no roles enumerated from it. |
| [Intel](https://intel.wd1.myworkdayjobs.com/External) | Listing and candidate detail returned shells; robots response was not usable. |
| [Boeing](https://jobs.boeing.com/) | Employer detail pages readable; IT category only partially enumerated. Its robots file disallows `/search-jobs/`. |
| [Lockheed Martin](https://www.lockheedmartin.com/en-us/careers/index.html) | Landing page readable; individual current job dates not established. |
| [JPMorgan Chase](https://www.jpmorganchase.com/careers) | Landing page readable; robots restricts linked job-details paths. |
| [Goldman Sachs](https://higher.gs.com/) | Selected site and robots request returned 502; no role verified. |
| [USAJOBS](https://www.usajobs.gov/) | Static software search reported no jobs, which does not prove the rendered search is empty. |
| [Harvard](https://hr.harvard.edu/search-jobs) | Selected page and robots request returned 403. |
| [Mayo Clinic](https://jobs.mayoclinic.org/) | Employer detail for a Rochester software engineer displayed 2026-08-20, outside the window; broader search was not enumerated. |
| [Cleveland Clinic](https://jobs.clevelandclinic.org/) | Search returned a static shell; a linked role returned 404. |
| [CDC](https://jobs.cdc.gov/) | Redirected to USAJOBS; not a distinct enumerated surface. |
| [Red Cross selected hostname](https://careers.redcross.org/) | DNS failed; the separately located employer careers page links to Workday. |
| [Johns Hopkins Medicine](https://jobs.hopkinsmedicine.org/jobs/information-technology/) | IT listing exposed roles; inspected recent clinical data roles did not establish SWE/IT duties. An inspected software engineer was dated 2026-09-28. |
| [Children's Hospital of Philadelphia](https://careers.chop.edu/) | Search returned a shell; one indexed administrator detail returned 410. |
| [Cornell](https://hr.cornell.edu/jobs) | Linked job-search page returned a shell; one indexed ML engineer detail returned 410. |
| [University of Virginia](https://jobs.virginia.edu/) | Search and IT category returned static shells without enumerated jobs. |

The [Boeing Senior Cloud DevOps Developer](https://jobs.boeing.com/job/hazelwood/senior-cloud-devops-developer/185/102655847072)
(`JR2026527973`, Missouri) and
[IT Systems Administrator](https://jobs.boeing.com/job/el-segundo/it-systems-administrator-millennium-space-systems/185/101069888336)
(`JR2026525275`, California) each visibly showed Post Date October 30 and an
Apply link; both also exposed structured `datePosted: 2026-10-30`. Their page
does not establish the source timezone. They are plausible recent US roles,
not confirmed strict-window reference jobs, and both link to Workday applications.
Neither requisition appeared in either bounded run's persisted results.
That is an output observation, not proof that either channel could never find
the jobs. A third [Boeing programmer analyst](https://jobs.boeing.com/job/fairfax/programmer-analyst-subject-matter-expert/185/102227910896)
(`JR2026527462`) showed the same visible date and a US location but was
contingent on program award; its publication timezone remains unknown.

The isolated canonical independent run began at **09:08:25.045 UTC** with the
unchanged 48-hour US filters and 20-new-job cap. It examined 36 candidates,
inserted **0**, and ended `partial`: its directory source attempted 100 pages
across 25 origins, its seed sample 100 pages across 17 origins, and both hit
page limits. Eight candidates had posting date as their primary rejection.
The temporary database contains zero jobs and zero discovery observations.
Artifacts: `/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-independent-live-kuJx74`.

The separate prior-channel run began at **09:09:04.137 UTC**, with the same
filter configuration and cap but a 39-second-later rolling cutoff. It inserted
**20** jobs after 1,203 examined candidates and ended `partial` with page/job
limits. Its result cannot establish an uncapped baseline. Artifacts:
`/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-independent-live-K8fXoP`.
No strictly reference-eligible job was established by the incomplete sample;
the miss-rate denominator and a comparable old/new recall rate remain
**not measurable**. Neither run demonstrates discovery of a qualifying
non-ATS employer from a directory seed.

A further manually seeded native run began at **09:14:38.895 UTC** from
`https://jobs.boeing.com/category/information-technology-jobs/185/2630/1`.
It persisted **4** jobs after examining 39 candidates; its 100-page, 7-origin
run ended `partial` at the page limit. The saved provenance shows the Boeing
seed, then a Workday sitemap and Workday-hosted final job pages—not an
ATS-independent employer result. The saved requisitions were `JR2026527973`,
`JR2026514051`, `JR2026526220`, and `JR2026528420`. Each final page was
independently reread and exposed `JobPosting.datePosted: 2026-10-30`; each
observation retained that date. The workflow interpreted these date-only values
in its configured `America/New_York` timezone; the pages did not independently
declare their publication timezone. `JR2026527973` was one of the sampled
roles above; `JR2026525275` was not persisted in this run. This manually targeted
run is not part of the prior/default channel comparison. Temporary SQLite had
four observations for four jobs, integrity `ok`, and no foreign-key violations.
Artifacts:
`/var/folders/kr/lz4p6cgs18s4583cskq0xjs80000gn/T/scrape-native-smoke-independent-live-HFwNHE`.

Repeat regressions: `cargo test --workspace` passed 739 tests with one ignored;
`npm run gen:types` and the bindings drift check passed; `npx tsc --noEmit`,
`npx vitest run --maxWorkers=1` (92 files, 588 tests), the frontend startup
smoke, `npm run build`, `node scripts/check-style.mjs`, and the recorded
`npm run test:providers` passed. The production bundle retained its large-chunk
advisory. These results do not change the live coverage limitations above.

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

The 20 older jobs were 17 `scraped` and 3 `viewed`, with no favorites or
nonblank notes. The latest startup/migration validator was also run against a
second isolated copy and returned `REPAIR_OK migration_version=35 integrity=ok`.
