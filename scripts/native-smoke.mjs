import { runNativeDev } from "./tauri-dev-test.mjs";
import { createServer } from "node:http";
import { mkdtemp, writeFile, readFile, chmod } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { execFileSync } from "node:child_process";
const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-"));
let opened = 0;
const server = createServer((request, response) => {
  response.setHeader("Access-Control-Allow-Origin", "*");
  if (request.url.startsWith("/slow")) { request.on("close", () => response.destroy()); return; }
  if (request.url.startsWith("/detail")) { opened++; response.end("Native opener fixture reached"); return; }
  if (request.url === "/inject-count-failure") {
    execFileSync("sqlite3", [join(directory, "scrape_data.db"), "CREATE TRIGGER native_fail_counts BEFORE UPDATE OF counts_json ON scrape_runs WHEN EXISTS (SELECT 1 FROM jobs WHERE external_id='diagnostic-write-failure') BEGIN SELECT RAISE(FAIL, 'native injected count failure'); END"]);
    response.end("ok"); return;
  }
  if (request.url === "/clear-count-failure") {
    execFileSync("sqlite3", [join(directory, "scrape_data.db"), "DROP TRIGGER native_fail_counts"]);
    response.end("ok"); return;
  }
  if (request.url === "/persist-failure") {
    response.end('<script type="application/ld+json">{"@type":"JobPosting","identifier":"diagnostic-write-failure","title":"Failure Fixture Engineer","url":"/failure-job","hiringOrganization":{"name":"Failure Fixture"}}</script>');
    return;
  }
  if (request.url === "/robots.txt") { response.end("User-agent: *\nAllow: /"); return; }
  if (request.url === "/quota") {
    response.end('<script type="application/ld+json">' + JSON.stringify([0, 1, 2].map(n => ({
      "@type": "JobPosting", identifier: `quota-${n}`, title: `Quota Engineer ${n}`,
      url: `/quota-job/${n}`, hiringOrganization: { name: "Quota Fixture" },
    }))) + "</script>");
    return;
  }
  response.end(`<script type="application/ld+json">[
    {"@type":"JobPosting","identifier":"42","title":"Native Fixture Engineer","url":"/detail?gh_jid=42","hiringOrganization":{"name":"Fixture Co"},"baseSalary":{"currency":"USD","value":{"minValue":45.25,"maxValue":55.75,"unitText":"HOUR"}}},
    {"@type":"JobPosting","identifier":"43","title":"Annual Fixture Engineer","url":"/detail?gh_jid=43","baseSalary":{"currency":"USD","value":{"minValue":100000,"unitText":"YEAR"}}}
  ]</script>`);
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
const spec = { version: 3, metadata: { name: "Native fixture" }, extractor: { kind: "jsonld", url: `${origin}/careers` }, schedule: { cadence: "manual", timezone: "UTC" }, filters: { compensation: { min_minor: 5000, max_minor: 6000, currency: "USD", interval: "hour", include_unknown: false } } };
const failureWorker = `#!/usr/bin/env node
const fs = require("node:fs");
const path = require("node:path");
const mode = fs.readFileSync(path.join(__dirname, "failure-mode"), "utf8").trim();
let input = "";
process.stdin.on("data", bytes => {
  input += bytes; if (!input.includes("\\n")) return;
  process.stdin.removeAllListeners("data"); process.stdin.pause(); process.stdin.unref?.();
  const config = JSON.parse(input.split("\\n")[0]);
  const source = config.sources[0];
  const job = { type: "job", source_id: source.id, external_id: "failure-" + mode, title: "Partial protocol fixture " + mode, company: "Fixture Co", role_type: "full_time", salary: { min: 104520, max: 126360, currency: "USD", interval: "hour", raw: "USD 50.25–60.75 HOUR", source: "structured", ranges: [{ min_minor: 5025, max_minor: 6075, currency: "USD", interval: "hour", raw: "USD 50.25–60.75 HOUR", source: "structured", derived: false, location: null }] }, url: "${origin}/detail?fixture=" + mode };
  const progress = { type: "progress", source_id: source.id, fetched: 1, matched: 1 };
  const summary = { source_id: source.id, limit: config.max_jobs_per_run || 20, commit_feedback: true, newly_committed: 1, examined: 1, matched: 1, accepted: 1, rejected: 0, blocked: 0, invalid: 0, duplicates: 0, unresolved: 0, enrichment_warnings: 0, limit_reached: false, rejections: [], rejection_examples: [], termination: "exhausted", outcome: "success" };
  const done = { type: "done", stats: { sources_ok: 1, sources_failed: 0, duration_ms: 1, jobs_emitted: 1, sources: [summary] } };
  const facts = { external_id: job.external_id, title: job.title, company: job.company, url: job.url, location: "", date_posted: null, provider_updated_at: null, description_excerpt: null, description_truncated: false, role_type: job.role_type, locations: [], location_data_complete: false, workplace_type: "unknown", salary: job.salary, experience: null, is_new_grad: "unknown", sponsorship: null, work_auth: null, clearance: null };
  const evaluation = { type: "candidate_evaluated", source_id: source.id, evaluation: { sequence: 1, disposition: "eligible", candidate: facts, rejected: [], unresolved: [], diagnostic: null } };
  const line = value => JSON.stringify(value) + "\\n";
  let output = Buffer.from(line(evaluation) + line(job) + line(progress));
  if (["optional-warning", "required-evidence"].includes(mode)) {
    const required = mode === "required-evidence";
    const warning = { type: "warning", source_id: source.id, message: required ? "Required publication date unavailable" : "Optional logo download unavailable", eligibility_incomplete: required, termination: null };
    const unresolved = { ...evaluation, evaluation: { ...evaluation.evaluation, disposition: "unresolved", unresolved: ["posting_date"] } };
    output = Buffer.from(line(warning) + line(required ? unresolved : evaluation) + (required ? "" : line(job)) + line({ ...progress, matched: required ? 0 : 1 }));
    done.stats.sources = [{ source_id: source.id, limit: config.max_jobs_per_run || 20, commit_feedback: true, newly_committed: required ? 0 : 1, examined: 1, matched: required ? 0 : 1, accepted: required ? 0 : 1, rejected: 0, blocked: 0, invalid: 0, duplicates: 0, limit_reached: false, rejections: [], rejection_examples: [], termination: required ? "failed" : "exhausted", provider_total: null, unresolved: required ? 1 : 0, enrichment_warnings: 1, outcome: required ? "partial" : "success_with_warnings" }];
    done.stats.jobs_emitted = required ? 0 : 1;
    done.stats.sources_ok = required ? 0 : 1; done.stats.sources_failed = required ? 1 : 0;
  }
  if (mode === "missing-summary") done.stats.sources = [];
  if (mode === "duplicate-summary") done.stats.sources = [summary, summary];
  if (mode === "invalid-branding") output = Buffer.concat([output,Buffer.from(line({type:"branding",source_id:source.id,company:"Fixture Co",logo:{status:"verified",checked_at:"2026-10-16T00:00:00Z",asset:"data:image/png;base64,broken",evidence:{url:"https://fixture.test/logo.png",page_url:"https://fixture.test/jobs",method:"employer_label"},diagnostic:null}}))]);
  if (mode === "malformed") output = Buffer.concat([output, Buffer.from('{"type":"job"}\\n')]);
  if (mode === "wrong-source") output = Buffer.concat([output, Buffer.from(line({ ...job, source_id: "unknown-workflow" }))]);
  if (mode === "invalid-utf8") {
    const invalid = line({ ...job, external_id: "invalid-extra", title: "TOKEN" }).split("TOKEN");
    output = Buffer.concat([output, Buffer.from(invalid[0]), Buffer.from([255]), Buffer.from(invalid[1])]);
  }
  const nextEval = { ...evaluation, evaluation: { ...evaluation.evaluation, sequence: 2, candidate: { ...facts, external_id: "pending-" + mode } } };
  if (mode === "bad-sequence") output = Buffer.concat([output, Buffer.from(line({ ...nextEval, evaluation: { ...nextEval.evaluation, sequence: 3 } }))]);
  if (mode === "wrong-evaluation-source") output = Buffer.concat([output, Buffer.from(line({ ...nextEval, source_id: "unknown-workflow" }))]);
  if (mode === "missing-evaluation") output = Buffer.concat([output, Buffer.from(line({ ...job, external_id: "unsolicited" }))]);
  if (mode === "wrong-job-correlation") output = Buffer.concat([output, Buffer.from(line(nextEval) + line({ ...job, external_id: "not-pending" }))]);
  if (mode === "pending-eof") output = Buffer.concat([output, Buffer.from(line(nextEval))]);
  if (!["missing-completion", "timeout", "pending-eof"].includes(mode)) output = Buffer.concat([output, Buffer.from(line(done))]);
  const finish = remaining => process.stdout.write(remaining, () => {
    if (mode === "timeout") setInterval(() => {}, 1000);
    else process.exit(mode === "nonzero" ? 7 : 0);
  });
  const jobLine = Buffer.from(line(job));
  const jobStart = output.indexOf(jobLine);
  if (jobStart < 0) { finish(output); return; }
  const afterJob = jobStart + jobLine.length;
  process.stdout.write(output.subarray(0, afterJob), () => {
    process.stdin.ref?.(); process.stdin.resume();
    process.stdin.once("data", bytes => {
      const ack = JSON.parse(bytes.toString().split("\\n")[0]);
      if (ack.source_id !== source.id || ack.external_id !== job.external_id || ack.outcome !== "inserted") process.exit(8);
      process.stdin.pause(); process.stdin.unref?.();
      finish(output.subarray(afterJob));
    });
  });
});
`;
await writeFile(join(directory, "smoke.js"), `
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const steps = [];
  try {
    await invoke("set_scheduler_paused", { paused: true });
    await invoke("update_setting", { key: "app.auto_update", value: false });
    await invoke("update_setting", { key: "notifications.enabled", value: false });
    const startupSettings = await invoke("get_settings");
    const startupSources = await invoke("list_sources");
    assert(!startupSettings.scraper_enabled && startupSources.every(source => !source.enabled), "Isolated startup must begin paused with workflows disabled");
    let retentionStatus;
    const retentionDeadline = Date.now() + 10000;
    while (Date.now() < retentionDeadline) {
      retentionStatus = await invoke("get_retention_status");
      if (retentionStatus.last_attempt_at) break;
      await pause(100);
    }
    assert(retentionStatus?.last_attempt_at && retentionStatus.consecutive_failures === 0, "Startup retention did not complete while paused with disabled workflows");
    steps.push("isolated startup retention while automatic scraping is paused and all workflows are disabled");
    const spec = ${JSON.stringify(spec)};
    const before = await invoke("list_jobs", { filter: {} });
    const preview = await invoke("preview_workflow", { spec, requestId: "native-preview" });
    assert(preview.matched === 1 && preview.fetched === 2, "Preview must use strict hourly matching");
    assert((await invoke("list_jobs", { filter: {} })).total === before.total, "Preview persisted jobs");
    const source = await invoke("save_workflow", { spec, id: null });
    assert((await invoke("list_jobs", { filter: {} })).total === before.total, "Saving scraped implicitly");
    steps.push("startup, canonical IPC and read-only preview/save");
    const uiDeadline = Date.now() + 15000;
    let settingsButton;
    while (Date.now() < uiDeadline && !settingsButton) {
      settingsButton = [...document.querySelectorAll("button")].find(button => button.textContent.trim() === "SETTINGS");
      if (!settingsButton) await pause(100);
    }
    assert(settingsButton, "Native settings navigation unavailable"); settingsButton.click();
    let picker;
    while (Date.now() < uiDeadline && !picker) {
      picker = document.querySelector('[aria-label="Interface font"]');
      if (!picker) await pause(100);
    }
    assert(picker, "Native font picker unavailable");
    for (const family of ["Inter Variable", "JetBrains Mono Variable", "Geist Mono Variable", "Fira Code Variable", "IBM Plex Mono"]) {
      const faces = await document.fonts.load('14px "' + family + '"', "Software Engineer 45.25");
      assert(faces.length > 0 && faces.every(face => face.status === "loaded"), "Native bundled font did not load: " + family);
    }
    picker.click();
    let jetbrainsOption;
    const optionDeadline = Date.now() + 5000;
    while (Date.now() < optionDeadline && !jetbrainsOption) {
      jetbrainsOption = [...document.querySelectorAll('[role="option"]')].find(option => option.textContent.toLocaleLowerCase().includes("jetbrains mono"));
      if (!jetbrainsOption) await pause(50);
    }
    assert(jetbrainsOption, "Native themed font options did not open"); jetbrainsOption.click();
    await pause(750);
    assert(document.documentElement.getAttribute("data-font") === "jetbrains", "Native font selection was not applied");
    assert(getComputedStyle(settingsButton).fontFamily.includes("JetBrains Mono Variable"), "Native navigation retained another family");
    steps.push("bundled native WebView fonts and global interface selection");

    const run = await invoke("run_source_now", { id: source.id });
    assert(!(await invoke("get_scheduler_status")).running, "Completed run still reported active");
    assert(run.status === "success" && run.counts.inserted === 1 && run.counts.rejected === 1, "Native sidecar run/audit failed: " + JSON.stringify(run));
    let job = (await invoke("list_jobs", { filter: {} })).items[0];
    assert(job.url === "${origin}/detail?gh_jid=42" && job.compensation[0].min_minor === 4525 && !job.compensation[0].derived, "Provider destination/native cents changed");
    await invoke("update_job_notes", { id: job.id, notes: "Native smoke protected note" });
    await invoke("set_job_favorite", { id: job.id, favorited: true });
    await invoke("update_job_status", { id: job.id, status: "applied" });
    steps.push("packaged sidecar, native compensation and persistence");
    const removal = await invoke("remove_job", { id: job.id, block: true });
    await invoke("delete_source", { id: source.id });
    const recreated = await invoke("save_workflow", { spec, id: null });
    const blockedPreview = await invoke("preview_workflow", { spec, requestId: "blocked-preview" });
    assert(blockedPreview.blocked === 1 && blockedPreview.jobs.length === 0, "Block preview crossed workflows incorrectly");
    const blockedRun = await invoke("run_source_now", { id: recreated.id });
    assert(blockedRun.counts.blocked === 1 && (await invoke("list_jobs", { filter: {} })).total === 0, "Blocked ingestion resurrected a posting");
    await invoke("restore_posting", { identity: removal.identity });
    job = (await invoke("list_jobs", { filter: {} })).items[0];
    assert(job.notes === "Native smoke protected note" && job.favorited && job.status === "applied", "Restore lost saved user state");
    steps.push("blocking across recreation and stateful restoration");
    const slow = await invoke("save_workflow", { spec: { ...spec, metadata: { name: "Cancellation fixture" }, extractor: { kind: "jsonld", url: "${origin}/slow" } }, id: null });
    const pending = invoke("run_source_now", { id: slow.id }); await pause(500);
    const activeStatus = await invoke("get_scheduler_status");
    assert(activeStatus.running && !activeStatus.enabled, "Active manual run lost paused automatic state");
    assert(await invoke("cancel_scrape"), "Run cancellation was not accepted");
    const cancelled = await pending;
    assert(!(await invoke("get_scheduler_status")).running, "Cancellation left scheduler status running");
    assert(cancelled.status === "cancelled" && cancelled.counts.failed === 0 && cancelled.error?.includes("cancelled"), "Cancellation did not retain its explicit outcome without failed postings");
    const pendingPreview = invoke("preview_workflow", { spec: { ...spec, extractor: { kind: "jsonld", url: "${origin}/slow" } }, requestId: "cancelled-preview" }).then(() => false, error => (error?.message || String(error)).includes("cancelled"));
    await pause(500); await invoke("cancel_workflow_preview", { requestId: "cancelled-preview" });
    assert(await pendingPreview, "Preview cancellation failed");
    await invoke("update_source", { input: { id: slow.id, enabled: false } });
    assert((await invoke("run_scrape_now")).status === "success", "Cancellation left the execution guard locked");
    steps.push("run/preview cancellation, reaping and recovery");
    for (const mode of ["optional-warning", "required-evidence"]) {
      await invoke("plugin:event|emit", { event: "native-smoke-control", payload: { mode } }); await pause(300);
      const count = (await invoke("list_jobs", { filter: {} })).total;
      const result = await invoke("run_source_now", { id: recreated.id });
      const expected = mode === "optional-warning" ? "success_with_warnings" : "partial";
      assert(result.status === expected && result.counts.failed === 0 && result.counts.enrichment_warnings === 1, "Incorrect enrichment classification: " + JSON.stringify(result));
      assert((await invoke("list_jobs", { filter: {} })).total === count + (mode === "optional-warning" ? 1 : 0), "Required evidence was persisted or optional candidate lost");
      const source = (await invoke("list_sources")).find(source => source.id === recreated.id);
      assert(source.last_outcome === expected, "Source outcome did not persist");
      const saved = (await invoke("list_scrape_runs", { limit: 100 })).find(run => run.id === result.id);
      assert(saved.status === expected && saved.counts.enrichment_warnings === 1, "Saved run outcome changed");
    }
    steps.push("optional enrichment warnings, missing required evidence, saved/source outcome classification and counts (authored protocol fixtures)");
    for (const mode of ["malformed", "wrong-source", "invalid-utf8", "invalid-branding", "nonzero", "missing-completion", "missing-summary", "duplicate-summary", "timeout", "bad-sequence", "missing-evaluation", "wrong-evaluation-source", "wrong-job-correlation", "pending-eof"]) {
      await invoke("plugin:event|emit", { event: "native-smoke-control", payload: { mode } });
      // Wait for the script driver to prepare a scoped process fixture.
      await pause(300);
      const count = (await invoke("list_jobs", { filter: {} })).total;
      const failed = await invoke("run_source_now", { id: recreated.id });
      const reason = { malformed: "Malformed worker event", "wrong-source": "unknown workflow ID", "invalid-utf8": "UTF-8", "invalid-branding": "Invalid employer branding report", nonzero: "Some(7)", "missing-completion": "completion", "missing-summary": "one summary", "duplicate-summary": "one summary", timeout: "timed out", "bad-sequence": "sequence is not contiguous", "missing-evaluation": "without an eligible candidate diagnostic", "wrong-evaluation-source": "unknown workflow ID", "wrong-job-correlation": "does not match", "pending-eof": "completion" }[mode];
      assert(failed.error?.includes(reason), "Wrong failure reason for " + mode + ": " + failed.error);
      assert(failed.status !== "success" && failed.counts.inserted === 1 && (await invoke("list_jobs", { filter: {} })).total === count + 1, "Protocol failure lost partial results or succeeded: " + mode + " " + JSON.stringify(failed));
    }
    await invoke("plugin:event|emit", { event: "native-smoke-control", payload: { mode: "restore-real-worker" } }); await pause(300);
    assert((await invoke("run_source_now", { id: recreated.id })).status === "success", "Protocol failure left the guard locked");
    steps.push("malformed/unknown-source/UTF-8 output, nonzero exit, missing completion, missing/duplicate summaries, timeout and partial-result recovery");
    const quotaSpec = { version: 3, max_jobs_per_run: 1, metadata: { name: "Quota fixture" }, extractor: { kind: "jsonld", url: "${origin}/quota" }, schedule: { cadence: "manual", timezone: "UTC" }, filters: {} };
    const quota = await invoke("save_workflow", { spec: quotaSpec, id: null });
    const first = await invoke("run_source_now", { id: quota.id });
    const second = await invoke("run_source_now", { id: quota.id });
    assert(first.counts.inserted === 1 && first.counts.updated === 0, "First quota run did not stop at one insertion: " + JSON.stringify(first));
    assert(second.counts.inserted === 1 && second.counts.updated === 1 && second.counts.sources[0].accepted === 2, "Refresh consumed the next discovery target: " + JSON.stringify(second));
    const quotaJobs = (await invoke("list_jobs", { filter: {} })).items.filter(job => job.company === "Quota Fixture");
    assert(quotaJobs.length === 2 && !quotaJobs.some(job => job.external_id === "quota-2"), "Run scheduled jobs beyond target");
    const third = await invoke("run_source_now", { id: quota.id });
    const exhausted = await invoke("run_source_now", { id: quota.id });
    assert(third.counts.inserted === 1 && third.counts.updated === 2, "Third discovery did not follow refreshes");
    assert(exhausted.counts.inserted === 0 && exhausted.counts.updated === 3 && exhausted.counts.sources[0].termination === "exhausted" && !exhausted.counts.sources[0].limit_reached, "Exhausted quota shortfall was misreported: " + JSON.stringify(exhausted));
    const failSpec = { version: 3, max_jobs_per_run: 1, metadata: { name: "Diagnostic failure fixture" }, extractor: { kind: "jsonld", url: "${origin}/persist-failure" }, schedule: { cadence: "manual", timezone: "UTC" }, filters: {} };
    const failWorkflow = await invoke("save_workflow", { spec: failSpec, id: null });
    await fetch("${origin}/inject-count-failure");
    const failedCommit = await invoke("run_source_now", { id: failWorkflow.id }).catch(error => ({ error: String(error) }));
    await fetch("${origin}/clear-count-failure");
    const retained = await invoke("list_jobs", { filter: {} });
    assert(retained.items.some(job => job.external_id === "diagnostic-write-failure"), "Post-commit failure lost the inserted row");
    assert(failedCommit.status !== "success", "Diagnostic write failure was reported as successful");
    steps.push("post-commit diagnostic failure retains the inserted row and reports failure");
    steps.push("real host/sidecar quota: refresh followed by one committed discovery, stopping before the third");
    await invoke("plugin:opener|open_url", { url: "${origin}/detail?gh_jid=42", with: null });
    await pause(1500); steps.push("native external opener command");
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: true, steps } });
  } catch (error) {
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: false, steps, error: error?.message || String(error) } });
  }
})();
`);
await writeFile(join(directory, "failure-worker.js"), failureWorker);
await chmod(join(directory, "failure-worker.js"), 0o755);
try {
  const { code, signal, output } = await runNativeDev(directory);
  if (code !== 0 || signal) throw new Error(`Native smoke failed (${code}/${signal}); isolated artifacts: ${directory}\n${output}`);
  const completion = JSON.parse(await readFile(join(directory, "result.json"), "utf8"));
  if (!completion.ok) throw new Error("Native smoke did not complete: " + JSON.stringify(completion));
  const consoleLog = await readFile(join(directory, "scrape.log"), "utf8");
  for (const expected of ["CANDIDATE 1 | INSERTED", "CANDIDATE 1 | UPDATED", "CANDIDATE 1 | BLOCKED AT SAVE", "CANDIDATE 2 | REJECTED", "CANDIDATE 1 | UNRESOLVED", "CANDIDATE 2 | NOT SAVED", "PERSISTENCE ERROR", "commit state may already be durable", "Quota Engineer 2", "coverage Exhausted", "SOURCE REQUIREMENTS"]) {
    if (!consoleLog.includes(expected)) throw new Error(`Native console missing ${expected}; isolated artifacts: ${directory}`);
  }
  if (!opened) throw new Error("Native opener returned without reaching the local destination");
  console.log(`Native smoke passed; external destination reached. Isolated artifacts: ${directory}`);
} finally { server.closeAllConnections(); server.close(); }
