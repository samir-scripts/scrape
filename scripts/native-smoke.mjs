import { runNativeDev } from "./tauri-dev-test.mjs";
import { createServer } from "node:http";
import { mkdtemp, writeFile, readFile, chmod } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-"));
let opened = 0;
const server = createServer((request, response) => {
  if (request.url.startsWith("/slow")) { request.on("close", () => response.destroy()); return; }
  if (request.url.startsWith("/detail")) { opened++; response.end("Native opener fixture reached"); return; }
  if (request.url === "/robots.txt") { response.end("User-agent: *\nAllow: /"); return; }
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
  const done = { type: "done", stats: { sources_ok: 1, sources_failed: 0, duration_ms: 1, jobs_emitted: 1 } };
  const line = value => JSON.stringify(value) + "\\n";
  let output = Buffer.from(line(job) + line(progress));
  if (mode === "malformed") output = Buffer.concat([output, Buffer.from('{"type":"job"}\\n')]);
  if (mode === "wrong-source") output = Buffer.concat([output, Buffer.from(line({ ...job, source_id: "unknown-workflow" }))]);
  if (mode === "invalid-utf8") {
    const invalid = line({ ...job, external_id: "invalid-extra", title: "TOKEN" }).split("TOKEN");
    output = Buffer.concat([output, Buffer.from(invalid[0]), Buffer.from([255]), Buffer.from(invalid[1])]);
  }
  if (!["missing-completion", "timeout"].includes(mode)) output = Buffer.concat([output, Buffer.from(line(done))]);
  process.stdout.write(output, () => {
    if (mode === "timeout") setInterval(() => {}, 1000);
    else process.exit(mode === "nonzero" ? 7 : 0);
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
    for (const source of await invoke("list_sources")) await invoke("update_source", { input: { id: source.id, enabled: false } });
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
    picker.value = "jetbrains"; picker.dispatchEvent(new Event("change", { bubbles: true }));
    await pause(500);
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
    assert(cancelled.status === "failed" && cancelled.error?.includes("cancelled"), "Cancelled worker was reported as success");
    const pendingPreview = invoke("preview_workflow", { spec: { ...spec, extractor: { kind: "jsonld", url: "${origin}/slow" } }, requestId: "cancelled-preview" }).then(() => false, error => (error?.message || String(error)).includes("cancelled"));
    await pause(500); await invoke("cancel_workflow_preview", { requestId: "cancelled-preview" });
    assert(await pendingPreview, "Preview cancellation failed");
    await invoke("update_source", { input: { id: slow.id, enabled: false } });
    assert((await invoke("run_scrape_now")).status === "success", "Cancellation left the execution guard locked");
    steps.push("run/preview cancellation, reaping and recovery");
    for (const mode of ["malformed", "wrong-source", "invalid-utf8", "nonzero", "missing-completion", "timeout"]) {
      await invoke("plugin:event|emit", { event: "native-smoke-control", payload: { mode } });
      // Wait for the script driver to prepare a scoped process fixture.
      await pause(300);
      const count = (await invoke("list_jobs", { filter: {} })).total;
      const failed = await invoke("run_source_now", { id: recreated.id });
      const reason = { malformed: "Malformed worker event", "wrong-source": "unknown workflow ID", "invalid-utf8": "UTF-8", nonzero: "Some(7)", "missing-completion": "completion", timeout: "timed out" }[mode];
      assert(failed.error?.includes(reason), "Wrong failure reason for " + mode + ": " + failed.error);
      assert(failed.status !== "success" && failed.counts.inserted === 1 && (await invoke("list_jobs", { filter: {} })).total === count + 1, "Protocol failure lost partial results or succeeded: " + mode + " " + JSON.stringify(failed));
    }
    await invoke("plugin:event|emit", { event: "native-smoke-control", payload: { mode: "restore-real-worker" } }); await pause(300);
    assert((await invoke("run_source_now", { id: recreated.id })).status === "success", "Protocol failure left the guard locked");
    steps.push("malformed/unknown-source/UTF-8 output, nonzero exit, missing completion, timeout and partial-result recovery");
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
  if (!opened) throw new Error("Native opener returned without reaching the local destination");
  console.log(`Native smoke passed; external destination reached. Isolated artifacts: ${directory}`);
} finally { server.closeAllConnections(); server.close(); }
