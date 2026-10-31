import { runNativeDev } from "./tauri-dev-test.mjs";
import { createServer } from "node:http";
import { mkdtemp, writeFile, readFile, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-targeted-"));
const now = () => new Date(Date.now() - 60_000).toISOString();
const server = createServer((request, response) => {
  response.setHeader("content-type", "application/json");
  const path = request.url || "";
  if (path.startsWith("/www.themuse.com/api/public/jobs")) {
    response.end(JSON.stringify({
      page: 0, page_count: 1, results: [
        { id: 90001, name: "Software Engineer", company: { name: "Native Fixture Engineering" }, refs: { landing_page: "https://fixture.invalid/engineering/90001" }, contents: "Fictional local replay posting.", publication_date: now(), locations: [{ name: "Remote, United States" }] },
        { id: 90002, name: "Crew Member", company: { name: "Native Fixture Fast Food" }, refs: { landing_page: "https://fixture.invalid/fast-food/90002" }, contents: "Fast food restaurant crew role. Fictional local replay posting.", publication_date: now(), locations: [{ name: "Cherry Hill, NJ" }] },
        { id: 90003, name: "Remote Software Engineer", company: { name: "Native Fixture Remote" }, refs: { landing_page: "https://fixture.invalid/remote/90003" }, contents: "Remote. Annual salary USD 70000. Fictional local replay posting.", publication_date: now(), locations: [{ name: "Remote, United States" }] },
      ],
    }));
    return;
  }
  if (path.startsWith("/api.adzuna.com/")) {
    response.end(JSON.stringify({ results: [], count: 0 }));
    return;
  }
  if (path.startsWith("/jobicy.com/api/jobs")) {
    response.end(JSON.stringify({ jobs: [], hasMore: false, nextCursor: null }));
    return;
  }
  if (path === "/www.arbeitnow.com/api/job-board-api") {
    response.end(JSON.stringify({ data: [], links: { next: null } }));
    return;
  }
  response.statusCode = 404;
  response.end(JSON.stringify({ error: "unconfigured fictional replay route" }));
});
await new Promise(resolve => server.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${server.address().port}`;
await writeFile(join(directory, "replay-origin"), `${origin}/\n`);
await writeFile(join(directory, "smoke.js"), `
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const steps = [];
  const progress = async message => {
    steps.push(message);
    await invoke("plugin:event|emit", { event: "native-smoke-log", payload: JSON.stringify({ step: message, at: new Date().toISOString() }) });
  };
  try {
    await invoke("set_scheduler_paused", { paused: true });
    await invoke("update_setting", { key: "app.auto_update", value: false });
    await invoke("update_setting", { key: "notifications.enabled", value: false });
    const initialJobs = await invoke("list_jobs", { filter: {} });
    assert(initialJobs.total === 0, "Native smoke database was not isolated/empty");
    const runBefore = await invoke("list_scrape_runs", { limit: 100 });
    const targetIds = ["software-engineering-last-12-hours", "cherry-hill-fast-food-last-month", "remote-usd-over-60000"];

    let workflowsButton;
    const deadline = Date.now() + 20000;
    while (Date.now() < deadline && !workflowsButton) {
      workflowsButton = [...document.querySelectorAll("button")].find(button => /WORKFLOWS/i.test(button.textContent));
      if (!workflowsButton) await pause(100);
    }
    assert(workflowsButton, "Workflow navigation is unavailable");
    workflowsButton.click();
    const replaceButton = await waitFor(() => [...document.querySelectorAll("button")].find(button => button.getAttribute("aria-label") === "Replace test workflows" || /Replace test workflows/i.test(button.textContent)), "Replace test workflows action");
    replaceButton.click();
    const dialog = await waitFor(() => {
      const current = document.querySelector('dialog[aria-label="Replace test workflows"]');
      return current?.open && current.textContent.includes("Saved jobs and tracking are kept") ? current : null;
    }, "replacement dialog");
    await waitFor(() => dialog.querySelector('[data-control-action="workflow.replace.mode.starter"]:not(:disabled)'), "starter target selector").then(button => button.click());
    await waitFor(() => dialog.querySelector('[data-control-action="workflow.replace.mode.starter"][aria-pressed="true"]') && dialog.textContent.includes("Cherry Hill") && !dialog.querySelector('[data-control-action="workflow.replace.confirm"]').disabled, "successful starter replacement preview");
    assert(dialog.textContent.includes("Saved jobs and tracking are kept") && dialog.textContent.includes("Software Engineering") && dialog.textContent.includes("Cherry Hill") && dialog.textContent.includes("Advertised Annual USD"), "Replacement disclosure/targets missing");
    await waitFor(() => dialog.querySelector('[data-control-action="workflow.replace.confirm"]:not(:disabled)'), "starter replacement confirmation").then(button => button.click());
    const replacementOutcome = await waitFor(() => {
      const report = [...document.querySelectorAll("dialog")].find(element => element.open && element.textContent.includes("Created "));
      if (report) return { report };
      const error = [...dialog.querySelectorAll('[role="alert"]')].find(element => element.textContent.includes("Replacement failed"));
      return error ? { error: error.textContent } : null;
    }, "replacement outcome");
    assert(replacementOutcome.report, "Replacement failed: " + replacementOutcome.error);
    await progress("Selected starter_workflows explicitly through the actual replacement dialog and confirmation");
    const replacementReport = replacementOutcome.report;
    const closeReport = [...replacementReport.querySelectorAll("button")].find(button => /Close/i.test(button.textContent));
    closeReport?.click();
    await waitFor(() => {
      const text = document.body.innerText;
      return text.includes("Software Engineering & Related") && text.includes("Fast Food — Cherry Hill") && text.includes("Remote — Advertised Annual USD");
    }, "three resulting workflow profile names in the UI");
    await progress("Inspected all three replacement profile names in the native workflow UI");

    const profiles = await invoke("list_sources");
    const groups = new Map();
    for (const source of profiles) {
      const spec = source.config?.__workflow_spec;
      const templateId = source.config?.__builtin_workflow?.template_id;
      if (templateId && targetIds.includes(templateId)) groups.set(templateId, [source]);
    }
    assert(profiles.length === 3, "Expected exactly three persisted workflow records in the isolated replacement database; found " + profiles.length);
    assert(groups.size === 3, "Expected exactly three distinct target workflow profiles; found: " + [...groups.keys()].join(","));
    const profileIds = [...groups.keys()].sort();
    assert(JSON.stringify(profileIds) === JSON.stringify([...targetIds].sort()), "Target workflow IDs differ: " + profileIds.join(","));
    await progress("Verified exactly three target profile identifiers from persisted built-in origin markers");

    await invoke("set_adzuna_credentials", { appId: "fictional-native-app-id", appKey: "fictional-native-app-key" });
    const credentialStatus = await invoke("get_adzuna_credential_status");
    assert(credentialStatus.configured, "Fictional Adzuna credentials were not stored in isolated smoke credential store");
    const workflow = groups.get("cherry-hill-fast-food-last-month")[0];
    const spec = workflow.config?.__workflow_spec;
    assert(spec?.sources, "Persisted fast-food workflow specification is missing");
    const adzuna = spec.sources.find(source => source.extractor?.url === "adzuna");
    assert(adzuna, "Fast-food profile has no Adzuna source to enable");
    adzuna.enabled = true;
    await invoke("save_workflow", { spec, id: workflow.id });
    const persistedSources = await invoke("list_sources");
    assert(!JSON.stringify(persistedSources).includes("fictional-native-app-key"), "Credential leaked into saved source configuration");
    const savedWorkflow = persistedSources.find(source => source.id === workflow.id);
    assert(savedWorkflow?.config?.__workflow_spec?.sources?.some(source => source.extractor?.url === "adzuna" && source.enabled), "Explicit Adzuna enablement did not persist");
    groups.set("cherry-hill-fast-food-last-month", [savedWorkflow]);
    await progress("Saved fictional credentials through isolated IPC and explicitly enabled Adzuna in the fast-food workflow specification");

    const jobsBeforePreview = await invoke("list_jobs", { filter: {} });
    const runBeforePreview = await invoke("list_scrape_runs", { limit: 100 });
    for (const [templateId, members] of groups) {
      const workflow = members[0];
      const spec = workflow.config?.__workflow_spec;
      if (spec) {
        const preview = await invoke("preview_workflow", { spec, requestId: "targeted-" + templateId });
        if (templateId === "cherry-hill-fast-food-last-month") {
          await progress("Fast-food replay preview: " + JSON.stringify({
            fetched: preview.fetched, matched: preview.matched, rejected: preview.rejected,
            errors: preview.errors,
            sources: preview.sources?.map(source => ({
              examined: source.examined, matched: source.matched, rejections: source.rejections,
              examples: source.rejection_examples,
            })),
          }));
        }
      }
      assert((await invoke("list_jobs", { filter: {} })).total === jobsBeforePreview.total, "Preview created jobs");
      assert((await invoke("list_scrape_runs", { limit: 100 })).length === runBeforePreview.length, "Preview created a persisted run");
      await progress("Previewed " + templateId + " without persistence side effects");
    }
    const afterSaveJobs = await invoke("list_jobs", { filter: {} });
    assert(afterSaveJobs.total === 0, "Workflow replacement/save created jobs");

    const target = groups.get("cherry-hill-fast-food-last-month")?.find(source => source.enabled) || groups.get("cherry-hill-fast-food-last-month")?.[0];
    assert(target, "Fast-food profile source missing");
    // One explicit single-workflow Run gesture; previews above are read-only.
    await invoke("run_source_now", { id: target.id });
    let resultJobs;
    const runDeadline = Date.now() + 45000;
    while (Date.now() < runDeadline) {
      resultJobs = await invoke("list_jobs", { filter: {} });
      if (resultJobs.total > 0) break;
      await pause(200);
    }
    const runAfter = await invoke("list_scrape_runs", { limit: 100 });
    assert(runAfter.length === runBefore.length + 1, "Explicit Run must persist exactly one run; before=" + runBefore.length + " after=" + runAfter.length);
    assert(resultJobs.total > 0, "Packaged worker replay did not persist any fixture jobs");
    const expectedJob = resultJobs.items.find(job => String(job.external_id) === "90002");
    assert(expectedJob?.url === "https://fixture.invalid/fast-food/90002" && expectedJob.title === "Crew Member" && expectedJob.company === "Native Fixture Fast Food", "Exact expected saved-job identity/fields differ: " + JSON.stringify(expectedJob));
    assert(expectedJob.source_id, "Persisted fixture job has no source provenance");
    await progress("Ran one explicit workflow, verified one persisted run and fictional provider results in list_jobs/SQLite");

    // Separate canonical test-profile journey: custom and all starter workflows survive.
    const customSpec = structuredClone(groups.get("software-engineering-last-12-hours")[0].config.__workflow_spec);
    customSpec.metadata.name = "Native custom workflow — preserve me";
    customSpec.metadata.enabled = false;
    customSpec.sources.forEach(source => { source.id = crypto.randomUUID(); });
    const custom = await invoke("save_workflow", { spec: customSpec, id: null });
    const beforeTestProfiles = await invoke("list_sources");
    const beforeTestRuns = await invoke("list_scrape_runs", { limit: 100 });
    const beforeTestJobs = await invoke("list_jobs", { filter: {} });
    await waitFor(() => document.querySelector('[data-control-action="workflow.replace.request"]:not(:disabled)'), "test-profile replacement action").then(button => button.click());
    const testDialog = await waitFor(() => {
      const current = document.querySelector('dialog[aria-label="Replace test workflows"]');
      return current?.open && current.querySelector('[data-control-action="workflow.replace.mode.test"][aria-pressed="true"]') && current.textContent.includes("test-workflows/software-engineering-it-us-48h.yaml") ? current : null;
    }, "canonical test-profile preview");
    assert(testDialog.textContent.includes("loaded disabled") && testDialog.textContent.includes("choose Run explicitly"), "Test profile disabled/explicit Run disclosure is missing");
    assert(![...testDialog.querySelectorAll('input[type="checkbox"]')].some(input => input.checked), "Existing starter/custom workflows unexpectedly selected for removal");
    await waitFor(() => testDialog.querySelector('[data-control-action="workflow.replace.confirm"]:not(:disabled)'), "test-profile confirmation").then(button => button.click());
    const testOutcome = await waitFor(() => {
      const report = document.querySelector('dialog[aria-label="Workflow replacement complete"][open]');
      const error = testDialog.querySelector('[role="alert"]');
      return report ? { report } : error ? { error: error.textContent } : null;
    }, "test-profile replacement outcome");
    assert(testOutcome.report, "Test-profile replacement failed: " + testOutcome.error);
    testOutcome.report.querySelector('[data-control-action="workflow.replace.report.close"]').click();
    const afterTestProfiles = await invoke("list_sources");
    const testProfile = afterTestProfiles.find(source => source.config?.__builtin_workflow?.template_id === "software-engineering-it-us-48h");
    assert(testProfile && !testProfile.enabled && testProfile.config.__workflow_spec.metadata.enabled === false, "Canonical test profile was not saved disabled");
    assert(testProfile.config.__workflow_spec.max_jobs_per_run === 20, "Canonical test quota differs");
    assert(testProfile.config.__workflow_spec.filters.matching.posting_date.preset === "past_48h", "Canonical test date filter differs");
    assert(testProfile.config.__workflow_spec.sources.find(source => source.extractor.kind === "search" && source.extractor.url === "adzuna")?.enabled === false, "Adzuna was enabled without explicit opt-in");
    assert(afterTestProfiles.length === beforeTestProfiles.length + 1, "Test-profile replacement removed or duplicated workflows");
    for (const original of beforeTestProfiles) {
      assert(JSON.stringify(afterTestProfiles.find(source => source.id === original.id)) === JSON.stringify(original), "Previously saved workflow changed: " + original.id);
    }
    assert(afterTestProfiles.some(source => source.id === custom.id && source.name === customSpec.metadata.name), "Custom workflow did not survive test replacement");
    assert((await invoke("list_scrape_runs", { limit: 100 })).length === beforeTestRuns.length, "Test-profile replacement started a run");
    assert((await invoke("list_jobs", { filter: {} })).total === beforeTestJobs.total, "Test-profile replacement altered saved jobs");
    const testRow = await waitFor(() => [...document.querySelectorAll('[data-control-action="workflow.edit"]')].find(button => button.getAttribute("aria-label") === "Edit workflow " + testProfile.name)?.closest("tr"), "disabled test profile row");
    assert(testRow.querySelector('[data-control-action="workflow.run"]').disabled, "Disabled test profile offers an enabled Run action");
    await progress("Loaded canonical test_workflow separately; verified persisted 48-hour/20 quota, disabled Run, no automatic crawl, and unchanged starters/custom workflow/jobs");
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: true, steps, artifacts: { directory: "${directory}", database: "${directory}/scrape_data.db", replayOrigin: "${directory}/replay-origin" } } });
  } catch (error) {
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: false, steps, error: error?.message || String(error), artifacts: { directory: "${directory}" } } });
  }
  async function waitFor(find, label) {
    const until = Date.now() + 20000;
    while (Date.now() < until) { const value = find(); if (value) return value; await pause(100); }
    throw new Error("Timed out waiting for " + label);
  }
})();
`);
try {
  const { code, signal, output } = await runNativeDev(directory);
  if (code !== 0 || signal) throw new Error(`Native targeted workflow smoke failed (${code}/${signal}); artifacts: ${directory}\n${output}`);
  const result = JSON.parse(await readFile(join(directory, "result.json"), "utf8"));
  if (!result.ok) throw new Error(`Native targeted workflow smoke failed: ${JSON.stringify(result)}; artifacts: ${directory}`);
  const artifacts = await readdir(directory);
  if (!artifacts.includes("scrape_data.db")) throw new Error(`Isolated SQLite database missing from ${directory}`);
  for (const name of ["workflow-progress.jsonl", "dev-startup.log"]) {
    try {
      const log = await readFile(join(directory, name), "utf8");
      if (log.includes("fictional-native-app-key") || log.includes("fictional-native-app-id")) {
        throw new Error(`Fictional Adzuna credential leaked into ${name}; isolated artifacts: ${directory}`);
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  console.log(`Native targeted workflow smoke completed: ${JSON.stringify(result)}; artifacts: ${directory}`);
} finally {
  server.closeAllConnections();
  server.close();
}
