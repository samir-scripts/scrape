import { runNativeDev } from "./tauri-dev-test.mjs";
import { createServer } from "node:http";
import { mkdtemp, readFile, readdir, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-web-scraping-"));
const pages = [];
const pendingProxyPaths = [];
const site = createServer((request, response) => {
  const path = new URL(request.url || "/", "http://fixture.invalid").pathname;
  const throughProxy = pendingProxyPaths[0] === path;
  if (throughProxy) pendingProxyPaths.shift();
  pages.push({ path, throughProxy, userAgent: request.headers["user-agent"] || null });
  if (path === "/robots.txt") {
    response.writeHead(200, { "content-type": "text/plain" });
    response.end("User-agent: *\nAllow: /");
    return;
  }
  const postings = {
    "/seed": ["html-1", "Native HTML Engineer"],
    "/listing": ["html-2", "Native Crawl Engineer"],
  };
  const posting = postings[path];
  if (!posting) {
    response.writeHead(404);
    response.end("not found");
    return;
  }
  const [id, title] = posting;
  response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
  response.end(`<script type="application/ld+json">${JSON.stringify({
    "@context": "https://schema.org",
    "@type": "JobPosting",
    identifier: id,
    title,
    url: `/jobs/${id}?team=platform`,
    datePosted: "2026-10-15",
    hiringOrganization: { name: "Native Public HTML Fixture" },
    description: "Fictional local public HTML fixture.",
  })}</script>${path === "/seed" ? '<a href="/listing">More public listings</a>' : ""}`);
});
await new Promise(resolve => site.listen(0, "127.0.0.1", resolve));
const origin = `http://127.0.0.1:${site.address().port}`;
const username = "native-proxy-fixture";
const password = "native-proxy-secret";
const expectedAuth = `Basic ${Buffer.from(`${username}:${password}`).toString("base64")}`;
const proxyRequests = [];
const proxy = createServer(async (request, response) => {
  if (request.headers["proxy-authorization"] !== expectedAuth) {
    response.writeHead(407, { "proxy-authenticate": "Basic realm=\"native fixture\"" });
    response.end();
    return;
  }
  let target;
  try { target = new URL(request.url); } catch {
    response.writeHead(400); response.end(); return;
  }
  if (target.origin !== origin || request.method !== "GET") {
    response.writeHead(403); response.end(); return;
  }
  proxyRequests.push(target.pathname);
  pendingProxyPaths.push(target.pathname);
  try {
    const upstream = await fetch(target);
    response.writeHead(upstream.status, {
      "content-type": upstream.headers.get("content-type") || "application/octet-stream",
      "connection": "close",
    });
    response.end(Buffer.from(await upstream.arrayBuffer()));
  } catch {
    response.writeHead(502); response.end();
  }
});
await new Promise(resolve => proxy.listen(0, "127.0.0.1", resolve));
const proxyOrigin = `http://127.0.0.1:${proxy.address().port}`;
const name = "Native public HTML crawl fixture";
const spec = {
  version: 9,
  max_jobs_per_run: 5,
  metadata: { name, enabled: true },
  sources: [{
    id: "public-html",
    label: "Public HTML fixture",
    enabled: true,
    employer_hint: "Native Public HTML Fixture",
    extractor: {
      kind: "selector",
      url: `${origin}/seed`,
      source_config: { crawl: { extraction: "jsonld", max_pages: 3, max_depth: 1 } },
    },
  }],
  schedule: { cadence: "manual", time: "09:00", timezone: "UTC", run_on_startup: false },
  filters: { role_types: [], include_keywords: [], exclude_keywords: [], locations: [] },
};
await writeFile(join(directory, "smoke.js"), `
(async () => {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const assert = (condition, message) => { if (!condition) throw new Error(message); };
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  const steps = [];
  const mark = async message => {
    steps.push(message);
    await invoke("plugin:event|emit", { event: "native-smoke-log", payload: { step: message } });
  };
  try {
    await invoke("set_scheduler_paused", { paused: true });
    await invoke("update_setting", { key: "app.auto_update", value: false });
    await invoke("update_setting", { key: "notifications.enabled", value: false });
    assert((await invoke("list_jobs", { filter: {} })).total === 0, "Isolated native database must start empty");
    const proxyStatus = await invoke("set_http_proxy", { server: ${JSON.stringify(proxyOrigin)}, username: ${JSON.stringify(username)}, password: ${JSON.stringify(password)} });
    assert(proxyStatus.configured && proxyStatus.authenticated && proxyStatus.server === ${JSON.stringify(proxyOrigin)}, "Authenticated proxy status was not saved");
    assert(!JSON.stringify(proxyStatus).includes(${JSON.stringify(password)}), "Proxy status exposed the secret");
    const status = await invoke("get_http_proxy_status");
    assert(status.authenticated && !JSON.stringify(status).includes(${JSON.stringify(username)}), "Proxy status leaked credentials");
    await mark("Saved write-only authenticated proxy configuration in the isolated native store");
    const detected = await invoke("detect_source", { url: ${JSON.stringify(origin)} + "/seed" });
    assert(detected.kind === "jsonld", "Source detection through authenticated proxy failed: " + JSON.stringify(detected));
    await mark("Detected source format through the authenticated proxy");
    const beforeRuns = await invoke("list_scrape_runs", { limit: 100 });
    const preview = await invoke("preview_workflow", { spec: ${JSON.stringify(spec)}, requestId: "native-public-html-preview" });
    assert(preview.fetched === 2 && preview.matched === 2, "Preview did not traverse and extract both local HTML pages: " + JSON.stringify(preview));
    assert((await invoke("list_jobs", { filter: {} })).total === 0, "Read-only preview persisted jobs");
    assert((await invoke("list_scrape_runs", { limit: 100 })).length === beforeRuns.length, "Preview persisted a run");
    await mark("Previewed two JSON-LD postings from bounded same-origin public HTML through the authenticated proxy");
    await invoke("save_workflow", { spec: ${JSON.stringify(spec)}, id: null });
    const saved = (await invoke("list_sources")).find(source => source.name === ${JSON.stringify(name)});
    assert(saved && !JSON.stringify(saved.config).includes(${JSON.stringify(password)}), "Saved workflow missing or contains proxy credentials");
    const run = await invoke("run_source_now", { id: saved.id });
    assert(run.status === "success" && run.counts.inserted === 2, "Explicit native crawl did not persist both posts: " + JSON.stringify(run));
    const jobs = await invoke("list_jobs", { filter: {} });
    assert(jobs.total === 2, "Native SQLite does not contain exactly two crawled postings: " + jobs.total);
    const ids = jobs.items.map(job => job.external_id).sort();
    assert(JSON.stringify(ids) === JSON.stringify(["html-1", "html-2"]), "Persisted posting identities changed: " + JSON.stringify(ids));
    assert(jobs.items.every(job => job.url.startsWith(${JSON.stringify(origin)} + "/jobs/") && job.company === "Native Public HTML Fixture"), "Original destinations/employer identity were not preserved");
    assert((await invoke("list_scrape_runs", { limit: 100 })).length === beforeRuns.length + 1, "Expected exactly one explicit persisted scrape run");
    await mark("Ran one explicit packaged-sidecar crawl and verified public HTML identities in isolated SQLite");
    await invoke("clear_http_proxy");
    assert(!(await invoke("get_http_proxy_status")).configured, "Clear proxy did not remove routing configuration");
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: true, steps } });
  } catch (error) {
    await invoke("plugin:event|emit", { event: "native-smoke-result", payload: { ok: false, steps, error: error?.message || String(error) } });
  }
})();
`);
try {
  const { code, signal, output } = await runNativeDev(directory);
  if (code !== 0 || signal) throw new Error(`Native web crawl/proxy smoke failed (${code}/${signal}); isolated artifacts: ${directory}\n${output}`);
  const result = JSON.parse(await readFile(join(directory, "result.json"), "utf8"));
  if (!result.ok) throw new Error(`Native web crawl/proxy smoke failed: ${JSON.stringify(result)}; isolated artifacts: ${directory}`);
  const expectedPaths = ["/seed", "/robots.txt", "/seed", "/listing", "/robots.txt", "/seed", "/listing"];
  let matchedCrawlRequests = 0;
  for (const path of proxyRequests) if (path === expectedPaths[matchedCrawlRequests]) matchedCrawlRequests++;
  if (matchedCrawlRequests !== expectedPaths.length) throw new Error(`Proxy missed one or more preview/run crawl requests: ${JSON.stringify(proxyRequests)}; artifacts: ${directory}`);
  if (pages.length !== proxyRequests.length || pages.some(request => !request.throughProxy)) throw new Error(`A public HTML request bypassed the authenticated proxy: ${JSON.stringify({ pages, proxyRequests })}; artifacts: ${directory}`);
  const artifacts = await readdir(directory);
  if (!artifacts.includes("scrape_data.db")) throw new Error(`Isolated SQLite database missing from ${directory}`);
  for (const file of ["workflow-progress.jsonl", "dev-startup.log"]) {
    try {
      const text = await readFile(join(directory, file), "utf8");
      if (text.includes(password) || text.includes(username)) throw new Error(`Proxy credential leaked into ${file}; artifacts: ${directory}`);
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
  }
  console.log(`Native public HTML/proxy smoke passed: ${JSON.stringify({ result, pages, proxyRequests, artifacts: directory })}`);
} finally {
  site.closeAllConnections();
  proxy.closeAllConnections();
  site.close();
  proxy.close();
}
