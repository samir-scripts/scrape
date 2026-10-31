// Opt-in live public-web smoke. Uses a temporary native profile and real network.
import { runNativeDev } from "./tauri-dev-test.mjs";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parse } from "yaml";

const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-independent-live-"));
let spec = parse(await readFile("test-workflows/software-engineering-it-us-48h.yaml", "utf8"));
spec.metadata.enabled = true;
if (process.argv[2] === "--baseline" && process.argv.length === 3) {
  const matching = spec.filters.matching;
  spec = parse(await readFile("workflows/software-engineering-last-12-hours.yaml", "utf8"));
  spec.metadata.name = "Live baseline comparison — US · Past 48 Hours";
  spec.filters = { role_types: [], include_keywords: [], exclude_keywords: [], locations: [], matching };
  spec.max_jobs_per_run = 20;
} else if (process.argv[2] === "--seed" && process.argv[3] && process.argv.length === 4) {
  const seed = new URL(process.argv[3]);
  if (seed.protocol !== "https:" || seed.username || seed.password) throw new Error("Seed must be a credential-free HTTPS URL");
  spec.sources = [{ id: "manual-live-seed", label: "Manually supplied public seed", enabled: true,
    extractor: { kind: "web_discovery", url: seed.href, source_config: { max_domains: 25, max_depth: 4, max_pages: 100 } } }];
} else if (process.argv.length !== 2) {
  throw new Error("Usage: node scripts/native-independent-live.mjs [--seed https://example.com/careers | --baseline]");
}
await writeFile(join(directory, "smoke.js"), `
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  try {
    await invoke("set_scheduler_paused", { paused: true });
    await invoke("update_setting", { key: "app.auto_update", value: false });
    await invoke("update_setting", { key: "notifications.enabled", value: false });
    const before = await invoke("list_jobs", { filter: {} });
    if (before.total !== 0) throw new Error("Expected an empty isolated database");
    const spec = ${JSON.stringify(spec)};
    const start = new Date().toISOString();
    const source = await invoke("save_workflow", { spec, id: null });
    const run = await invoke("run_source_now", { id: source.id });
    const jobs = await invoke("list_jobs", { filter: {} });
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: true, start, spec, run, jobs } });
  } catch (error) {
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: false, error: String(error) } });
  }
})();
`);
const { code, signal, output } = await runNativeDev(directory);
if (code !== 0 || signal) throw new Error(`Live independent smoke failed (${code}/${signal}); artifacts: ${directory}\n${output}`);
const result = JSON.parse(await readFile(join(directory, "result.json"), "utf8"));
if (!result.ok) throw new Error(`Live independent smoke failed: ${JSON.stringify(result)}; artifacts: ${directory}`);
console.log(`Live independent smoke completed; artifacts: ${directory}\n${JSON.stringify(result)}`);
if (result.run.status === "failed") process.exitCode = 1;
