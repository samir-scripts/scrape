// Opt-in live public-web smoke. Never opens the user's application profile.
import { runNativeDev } from "./tauri-dev-test.mjs";
import { mkdtemp, readFile, writeFile, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, basename, dirname } from "node:path";
import { pathToFileURL } from "node:url";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { parse } from "yaml";

// Runs inside the actual WebView before native-smoke-result terminates the host.
export const inspectCandidates = `async function inspectCandidates(jobs) {
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const wait = async (read, label) => {
    const end = Date.now() + 10000;
    while (Date.now() < end) { const value = await read(); if (value) return value; await pause(50); }
    throw new Error("Deadline exceeded waiting for " + label);
  };
  const visible = el => el && el.getClientRects().length;
  const inspected = [];
  if (!jobs.length) return inspected;
  (await wait(() => [...document.querySelectorAll('[data-control-action="navigation.discovered.open"]')].find(visible), "Discovered navigation")).click();
  for (const job of jobs) {
    const saved = await invoke("get_job", { id: job.id });
    if (!saved || saved.url !== job.url) throw new Error("Persisted candidate identity mismatch: " + job.id);
    const label = "View " + saved.title + " at " + saved.company;
    const controls = await wait(() => {
      const matches = [...document.querySelectorAll('[data-control-action="job.inspect"]')].filter(el => visible(el) && (el.getAttribute("aria-label") === label || (el.textContent.trim() === saved.title && el.closest("tr")?.textContent.includes(saved.company))));
      return matches.length ? matches : null;
    }, "candidate " + job.id);
    // Duplicate titles must not accidentally attest a different saved row.
    let matched = false;
    for (const control of controls) {
      control.click();
      await pause(50);
      const pane = await wait(() => [...document.querySelectorAll('[aria-label="Job details"]')].find(el => visible(el) && el.querySelector("[data-dialog-scroll] header h2")?.textContent.trim() === saved.title && el.textContent.includes(saved.company)), "job inspector");
      const heading = pane.querySelector("[data-dialog-scroll] header h2");
      if (heading?.textContent.trim() !== saved.title || !pane.textContent.includes(saved.company)) continue;
      const evidence = pane.querySelector('[data-control-action="job.evidence.expand"]');
      if (evidence && !evidence.parentElement.open) evidence.click();
      const renderedUrl = evidence?.parentElement.querySelector("p")?.textContent.trim();
      if (renderedUrl !== saved.url) continue;
      const details = pane.querySelector('[data-control-action="job.details.expand"]');
      if (details && !details.parentElement.open) details.click();
      const posted = [...pane.querySelectorAll("dt")].find(el => el.textContent.trim() === "Posted")?.nextElementSibling?.textContent.trim();
      if (!posted || posted === "—" || posted === "Unknown") throw new Error("Candidate has no rendered publication date: " + job.id);
      await wait(async () => (await invoke("get_job", { id: job.id })).status !== "scraped", "selected persisted job " + job.id);
      inspected.push({ id: saved.id, title: heading.textContent.trim(), company: saved.company, url: renderedUrl, posted, inspector: pane.innerText });
      matched = true;
      break;
    }
    if (!matched) throw new Error("Could not inspect exact candidate " + job.id);
  }
  return inspected;
}`;

export async function verifyTemporaryDatabase(directory, result) {
  const root = await realpath(directory);
  const temporary = await realpath(tmpdir());
  if (!root.startsWith(temporary + "/") || !basename(root).startsWith("scrape-native-smoke-")) throw new Error("Refusing a non-smoke database");
  const database = await realpath(join(root, "scrape_data.db"));
  if (dirname(database) !== root) throw new Error("Database escaped isolated profile");
  console.log(`Isolated verification SQLite: ${database}`);
  const literal = value => "'" + String(value).replaceAll("'", "''") + "'";
  const query = `SELECT j.id,j.url,j.title,j.company,j.source_id,j.workflow_id,j.date_posted,o.run_id,o.source_id AS observation_source_id,o.source_key,o.observed_at,o.provenance,s.workflow_id AS source_workflow_id FROM jobs j JOIN discovery_observations o ON o.job_id=j.id JOIN sources s ON s.id=o.source_id WHERE o.run_id=${literal(result.run.id)} ORDER BY j.id,o.source_key`;
  let backend = "sqlite3";
  let rows, integrity, foreignKeys;
  try {
    rows = JSON.parse(execFileSync("sqlite3", ["-readonly", "-json", database, query], { encoding: "utf8" }) || "[]");
    integrity = execFileSync("sqlite3", ["-readonly", database, "PRAGMA integrity_check"], { encoding: "utf8" }).trim();
    foreignKeys = execFileSync("sqlite3", ["-readonly", database, "PRAGMA foreign_key_check"], { encoding: "utf8" }).trim();
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    backend = "python3 sqlite3 mode=ro";
    const verifier = join(root, "verify-sqlite.py");
    await writeFile(verifier, `import json, sqlite3, sys\nfrom pathlib import Path\nc = sqlite3.connect(Path(sys.argv[1]).as_uri() + '?mode=ro', uri=True)\nc.row_factory = sqlite3.Row\nprint(json.dumps({'rows': [dict(r) for r in c.execute(sys.argv[2])], 'integrity': c.execute('PRAGMA integrity_check').fetchone()[0], 'foreignKeys': [dict(r) for r in c.execute('PRAGMA foreign_key_check')]}))\n`);
    ({ rows, integrity, foreignKeys } = JSON.parse(execFileSync("python3", [verifier, database, query], { encoding: "utf8" })));
  }
  if (integrity !== "ok" || (Array.isArray(foreignKeys) ? foreignKeys.length : foreignKeys)) throw new Error("Temporary database integrity/foreign-key failure");
  if (result.run.counts.inserted > 20 || result.run.counts.inserted < 0) throw new Error("Shared insertion quota violated");
  for (const job of result.inspected) {
    const observations = rows.filter(row => row.id === job.id);
    if (!observations.length) throw new Error("UI-inspected job has no observation in this run: " + job.id);
    for (const row of observations) {
      if (row.url !== job.url || row.source_id !== row.observation_source_id || row.workflow_id !== result.source.id || row.source_workflow_id !== result.source.id) throw new Error("Candidate source/run ownership mismatch: " + job.id);
      row.provenance = JSON.parse(row.provenance);
    }
  }
  const ids = new Set(rows.map(row => row.id));
  if (ids.size !== result.run.counts.inserted || result.inspected.length !== ids.size) throw new Error("Inserted/UI/observation coverage mismatch");
  return { database, backend, integrity, foreignKeys, rows, observationCoverage: true, sourceOwnership: true };
}

async function main() {
  const args = process.argv.slice(2);
  const verify = args.length === 1 && args[0] === "--verify-non-ats";
  if (!(args.length === 0 || verify || (args.length === 1 && args[0] === "--baseline") || (args.length === 2 && args[0] === "--seed"))) throw new Error("Usage: node scripts/native-independent-live.mjs [--verify-non-ats | --seed https://example.com/careers | --baseline]");
  const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-independent-live-"));
  console.log(`Live independent artifacts: ${directory}`);
  const canonical = await readFile("test-workflows/software-engineering-it-us-48h.yaml", "utf8");
  const specHash = createHash("sha256").update(canonical).digest("hex");
  let spec = parse(canonical);
  spec.metadata.enabled = true;
  if (args[0] === "--baseline") {
    const matching = spec.filters.matching;
    spec = parse(await readFile("workflows/software-engineering-last-12-hours.yaml", "utf8"));
    spec.metadata.name = "Live baseline comparison — US · Past 48 Hours";
    spec.filters = { role_types: [], include_keywords: [], exclude_keywords: [], locations: [], matching };
    spec.max_jobs_per_run = 20;
  } else if (args[0] === "--seed") {
    const seed = new URL(args[1]);
    if (seed.protocol !== "https:" || seed.username || seed.password) throw new Error("Seed must be a credential-free HTTPS URL");
    spec.sources = [{ id: "manual-live-seed", label: "Manually supplied public seed", enabled: true,
      extractor: { kind: "web_discovery", url: seed.href, source_config: { max_domains: 25, max_depth: 4, max_pages: 100 } } }];
  }
  await writeFile(join(directory, "canonical-workflow.yaml"), canonical);
  await writeFile(join(directory, "executed-spec.json"), JSON.stringify(spec, null, 2));
  await writeFile(join(directory, "smoke.js"), `
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  ${inspectCandidates}
  try {
    await invoke("set_scheduler_paused", { paused: true });
    await invoke("update_setting", { key: "app.auto_update", value: false });
    await invoke("update_setting", { key: "notifications.enabled", value: false });
    if ((await invoke("list_jobs", { filter: {} })).total !== 0) throw new Error("Expected an empty isolated database");
    const spec = ${JSON.stringify(spec)};
    const start = new Date().toISOString();
    const source = await invoke("save_workflow", { spec, id: null });
    const run = await invoke("run_source_now", { id: source.id });
    const jobs = await invoke("list_jobs", { filter: {} });
    const inspected = ${verify ? "await inspectCandidates(jobs.items)" : "[]"};
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: true, start, spec, source, run, jobs, inspected, runId: run.id, inspectedJobIds: inspected.map(job => job.id) } });
  } catch (error) {
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: false, error: String(error) } });
  }
})();
`);
  const previous = { RUST_LOG: process.env.RUST_LOG, SCRAPE_NATIVE_DISCOVERY_TRACE: process.env.SCRAPE_NATIVE_DISCOVERY_TRACE };
  let execution;
  try {
    process.env.RUST_LOG = "scrape_worker::adapters::web_discovery=debug";
    process.env.SCRAPE_NATIVE_DISCOVERY_TRACE = "1";
    execution = await runNativeDev(directory);
  } finally {
    for (const [key, value] of Object.entries(previous)) { if (value === undefined) delete process.env[key]; else process.env[key] = value; }
  }
  const { code, signal, output } = execution;
  if (code !== 0 || signal) throw new Error(`Live independent smoke failed (${code}/${signal}); artifacts: ${directory}\n${output}`);
  const result = JSON.parse(await readFile(join(directory, "result.json"), "utf8"));
  if (!result.ok || result.run.status === "failed") throw new Error(`Live independent execution failed: ${JSON.stringify(result)}; artifacts: ${directory}`);
  if (verify) {
    const sqlite = await verifyTemporaryDatabase(directory, result);
    const trace = output.split("\n").filter(line => line.includes(`[native-discovery run=${result.run.id}]`));
    await writeFile(join(directory, "discovery-trace.log"), trace.join("\n") + "\n");
    const traceComplete = trace.length > 0 && !trace.some(line => line.includes("TRACE_TRUNCATED"));
    // This exclusion only rejects known ATS destinations; passing it is not proof
    // of employer control or of a direct application route.
    const knownAts = ["myworkdayjobs.com", "myworkdaysite.com", "greenhouse.io", "lever.co", "ashbyhq.com", "smartrecruiters.com", "icims.com", "taleo.net", "successfactors.com", "jobvite.com", "bamboohr.com", "applytojob.com"];
    const candidates = result.inspected.filter(job => {
      const host = new URL(job.url).hostname.toLowerCase();
      const evidence = sqlite.rows.filter(row => row.id === job.id).map(row => row.provenance.date_evidence);
      return !knownAts.some(ats => host === ats || host.endsWith("." + ats))
        && evidence.length > 0 && evidence.every(date => date?.raw_value && date?.page_url && date?.locator);
    });
    const candidate = candidates.length > 0 && traceComplete;
    const report = { acceptance: candidate ? "candidate" : "unproven", runId: result.run.id, canonicalSpecSha256: specHash,
      cutoff: new Date(Date.parse(result.run.started_at || result.start) - 48 * 3600_000).toISOString(),
      execution: result.run.status, boundedPartial: result.run.status === "partial", jobs: result.inspected,
      native: { inspectedJobIds: result.inspectedJobIds }, sqlite, trace: { path: join(directory, "discovery-trace.log"), untruncated: traceComplete },
      candidateJobIds: candidates.map(job => job.id),
      reviewRequired: "Candidate collection is not verified non-ATS acceptance. Independently inspect ordered chain, final posting, application route, US duties and publisher publication interval; record acceptance-review.json. Missing/truncated trace cannot prove the full chain." };
    await writeFile(join(directory, "candidate-report.json"), JSON.stringify(report, null, 2));
    console.log(`Candidate collection ${report.acceptance}; not verified acceptance; artifacts: ${directory}`);
    process.exitCode = candidate ? 0 : 2;
  } else console.log(`Live independent diagnostic completed; artifacts: ${directory}\n${JSON.stringify(result)}`);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await main().catch(error => { console.error(error); process.exitCode = 1; });
}
