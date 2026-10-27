import { runNativeDev } from "./tauri-dev-test.mjs";
import { createServer } from "node:http";
import { mkdtemp, readFile, readdir, writeFile } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { parse as parseYaml, stringify as stringifyYaml } from "yaml";

const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-web-scraping-"));
const pages = [];
const pendingProxyPaths = [];
const recentDate = new Date(Date.now() - 3600_000).toISOString();

const rawCrawlerYaml = readFileSync("workflows/software-engineering-web-scraper-test.yaml", "utf8");
const rawDiscoveryYaml = readFileSync("workflows/software-engineering-discovery-test.yaml", "utf8");
let bulkMode = false;
const bulkSpec = parseYaml(rawDiscoveryYaml);
bulkSpec.metadata.name = "Thousand-candidate diagnostic fixture";
bulkSpec.sources[0].extractor.source_config = { follow_detail: false, max_pages: 1 };

const site = createServer((request, response) => {
  response.setHeader("Access-Control-Allow-Origin", "*");
  response.setHeader("Access-Control-Allow-Headers", "*");
  if (request.method === "OPTIONS") {
    response.writeHead(204);
    response.end();
    return;
  }
  let path = new URL(request.url || "/", "http://fixture.invalid").pathname;
  if (path.startsWith("/127.0.0.1/")) {
    path = path.slice("/127.0.0.1".length);
  }
  const throughProxy = pendingProxyPaths[0] === path || pendingProxyPaths[0] === "/127.0.0.1" + path;
  if (throughProxy) pendingProxyPaths.shift();
  pages.push({ path, throughProxy, userAgent: request.headers["user-agent"] || null });
  if (path === "/robots.txt" || path.endsWith("/robots.txt")) {
    response.writeHead(200, { "content-type": "text/plain" });
    response.end("User-agent: *\nAllow: /\n");
    return;
  }
  if (path === "/crawler.yaml") {
    response.writeHead(200, { "content-type": "text/yaml; charset=utf-8" });
    response.end(rawCrawlerYaml);
    return;
  }
  if (path === "/discovery.yaml") {
    response.writeHead(200, { "content-type": "text/yaml; charset=utf-8" });
    response.end(rawDiscoveryYaml);
    return;
  }
  if (path === "/enable-bulk") {
    bulkMode = true;
    response.writeHead(200);
    response.end("ok");
    return;
  }
  // Palantir Lever crawl routes
  if (path === "/jobs.lever.co/palantir") {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end('<div class="posting"><a class="posting-title" href="/palantir/jobs/palantir-native-1"><h5>Software Engineer</h5></a></div>');
    return;
  }
  if (path === "/jobs.lever.co/palantir/jobs/palantir-native-1") {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(`<!DOCTYPE html><html><head><script type="application/ld+json">${JSON.stringify({
      "@context": "https://schema.org",
      "@type": "JobPosting",
      title: "Software Engineer",
      hiringOrganization: { name: "Palantir Technologies" },
      datePosted: recentDate,
      jobLocation: { address: { addressCountry: "US", addressLocality: "Palo Alto" } },
      description: "Build distributed software systems."
    })}</script></head><body><h1>Software Engineer</h1><div class="job-description"><p>Build distributed software systems.</p></div></body></html>`);
    return;
  }

  // SEP Lever crawl routes
  if (path === "/jobs.lever.co/sep") {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end('<div class="posting"><a class="posting-title" href="/sep/jobs/sep-native-1"><h5>Software Engineer</h5></a></div>');
    return;
  }
  if (path === "/jobs.lever.co/sep/jobs/sep-native-1") {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(`<!DOCTYPE html><html><head><script type="application/ld+json">${JSON.stringify({
      "@context": "https://schema.org",
      "@type": "JobPosting",
      title: "Software Engineer",
      hiringOrganization: { name: "SEP" },
      datePosted: recentDate,
      jobLocation: { address: { addressCountry: "US", addressLocality: "Carmel" } },
      description: "Software engineering and architecture."
    })}</script></head><body><h1>Software Engineer</h1><div class="job-description"><p>Software engineering and architecture.</p></div></body></html>`);
    return;
  }

  // The Muse search discovery routes
  if (path.startsWith("/www.themuse.com/api/public/jobs")) {
    response.writeHead(200, { "content-type": "application/json" });
    if (bulkMode) {
      response.end(JSON.stringify({
        page: 0, page_count: 2,
        results: Array.from({ length: 1000 }, (_, index) => ({
          id: 10000 + index,
          name: index === 0 ? "AI/ML Engineer - Mid Level" : "Retail Associate",
          company: { name: "Bulk Fixture" },
          refs: { landing_page: `https://www.themuse.com/jobs/bulk/${index}` },
          contents: "<p>Fictional bulk fixture.</p>",
          publication_date: recentDate,
          locations: [{ name: "Phoenix, AZ" }],
        })),
      }));
      return;
    }
    response.end(JSON.stringify({
      page: 0,
      page_count: 1,
      results: [{
        id: 801,
        name: "Software Engineer",
        company: { name: "Native Heuristic Discovery Inc" },
        refs: { landing_page: "https://www.themuse.com/jobs/discovered/801" },
        contents: "<p>Native heuristic discovery posting.</p>",
        publication_date: recentDate,
        locations: [{ name: "Austin, TX" }],
      }]
    }));
    return;
  }
  if (path === "/www.themuse.com/jobs/discovered/801") {
    response.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    response.end(`<!DOCTYPE html><html><head><script type="application/ld+json">${JSON.stringify({
      "@context": "https://schema.org",
      "@type": "JobPosting",
      title: "Software Engineer",
      hiringOrganization: { name: "Native Heuristic Discovery Inc" },
      datePosted: recentDate,
      jobLocation: { address: { addressCountry: "US", addressLocality: "Austin" } },
      description: "Cross-employer platform architecture."
    })}</script></head><body><h1>Software Engineer</h1></body></html>`);
    return;
  }

  // Original proxy crawl routes
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
    datePosted: recentDate,
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

// Direct worker replay origin routing
await writeFile(join(directory, "replay-origin"), `${origin}/\n`);

const name = "Native public HTML crawl fixture";
const spec = {
  version: 10,
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
  const wait = async (read, label, timeoutMs = 20000) => {
    const end = Date.now() + timeoutMs;
    while (Date.now() < end) {
      const value = await read();
      if (value) return value;
      await pause(50);
    }
    throw new Error("Deadline exceeded waiting for " + label);
  };
  const button = text => [...document.querySelectorAll("button")].find(el => el.textContent.trim().toUpperCase() === text.toUpperCase() && el.getClientRects().length);
  const field = label => document.querySelector('input[aria-label="' + label + '"], textarea[aria-label="' + label + '"], button[aria-label="' + label + '"][aria-haspopup="listbox"]');
  const change = (input, value) => {
    input.focus();
    const prop = input instanceof HTMLTextAreaElement ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prop, "value").set.call(input, value);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  };

  try {
    await invoke("set_scheduler_paused", { paused: true });
    await invoke("update_setting", { key: "app.auto_update", value: false });
    await invoke("update_setting", { key: "notifications.enabled", value: false });
    assert((await invoke("list_jobs", { filter: {} })).total === 0, "Isolated native database must start empty");

    // 1. Authenticated proxy verification
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

    await invoke("clear_http_proxy");
    assert(!(await invoke("get_http_proxy_status")).configured, "Clear proxy did not remove routing configuration");
    await mark("Cleared proxy configuration before workflow tests");

    // 2. Drive Workflow 1 (Company crawler) through actual UI
    (await wait(() => button("WORKFLOWS"), "WORKFLOWS button")).click();
    (await wait(() => [...document.querySelectorAll("button")].find(b => b.textContent.includes("ADD WORKFLOW")), "ADD WORKFLOW button")).click();
    (await wait(() => button("Advanced"), "Advanced button")).click();
    (await wait(() => button("Edit YAML"), "Edit YAML button")).click();
    const yamlInput1 = await wait(() => field("Workflow YAML"), "Workflow YAML textarea");
    const crawlerYamlText = await fetch(${JSON.stringify(origin)} + "/crawler.yaml").then(r => r.text());
    change(yamlInput1, crawlerYamlText);
    await pause(50);
    (await wait(() => button("Apply YAML to form"), "Apply YAML to form button")).click();
    await pause(100);
    for (const s of ["Criteria", "Sources", "Schedule", "Review"]) {
      (await wait(() => [...document.querySelectorAll("button")].find(b => b.textContent.includes("Continue to " + s)), "Continue to " + s)).click();
      await pause(50);
    }
    (await wait(() => button("Preview matches"), "Preview matches button")).click();

    // Wait for preview results in UI
    await wait(() => document.querySelector('[aria-label="Preview results"]'), "Preview results container", 30000);
    const previewResult1 = document.querySelector('[aria-label="Preview results"]');
    assert(previewResult1.textContent.includes("eligible matches"), "Preview matches header missing");
    assert((await invoke("list_jobs", { filter: {} })).total === 0, "Preview must not persist jobs");

    // Save workflow
    const saveBtn1 = await wait(() => {
      const btn = button("Save workflow");
      return btn && !btn.disabled ? btn : null;
    }, "enabled Save workflow button");
    saveBtn1.click();
    const runBtn1 = await wait(() => button("Run now"), "Run now button on confirmation 1");
    runBtn1.click();
    await wait(() => !document.querySelector('dialog[aria-label="Create workflow"][open]'), "closed dialog after run now 1", 30000);
    await mark("Executed explicit run now for company crawler workflow");

    const jobsAfterRun1 = await invoke("list_jobs", { filter: {} });
    assert(jobsAfterRun1.total >= 1, "Jobs must be persisted in SQLite after explicit crawl run");
    assert(jobsAfterRun1.items.some(j => j.company.includes("Palantir") || j.company.includes("SEP")), "Persisted jobs must belong to Palantir or SEP");

    // 3. Drive Workflow 2 (Heuristic discovery) through actual UI
    (await wait(() => [...document.querySelectorAll("button")].find(b => b.textContent.includes("ADD WORKFLOW")), "ADD WORKFLOW button 2")).click();
    (await wait(() => button("Advanced"), "Advanced button 2")).click();
    (await wait(() => button("Edit YAML"), "Edit YAML button 2")).click();
    const yamlInput2 = await wait(() => field("Workflow YAML"), "Workflow YAML textarea 2");
    const discoveryYamlText = await fetch(${JSON.stringify(origin)} + "/discovery.yaml").then(r => r.text());
    change(yamlInput2, discoveryYamlText);
    await pause(50);
    (await wait(() => button("Apply YAML to form"), "Apply YAML to form button 2")).click();
    await pause(100);
    for (const s of ["Criteria", "Sources", "Schedule", "Review"]) {
      (await wait(() => [...document.querySelectorAll("button")].find(b => b.textContent.includes("Continue to " + s)), "Continue to " + s + " 2")).click();
      await pause(50);
    }
    (await wait(() => button("Preview matches"), "Preview matches button 2")).click();

    await wait(() => document.querySelector('[aria-label="Preview results"]'), "Preview results container 2", 30000);
    const saveBtn2 = await wait(() => {
      const btn = button("Save workflow");
      return btn && !btn.disabled ? btn : null;
    }, "enabled Save workflow button 2");
    saveBtn2.click();
    const runBtn2 = await wait(() => button("Run now"), "Run now button on confirmation 2");
    runBtn2.click();
    await wait(() => !document.querySelector('dialog[aria-label="Create workflow"][open]'), "closed dialog after run now 2", 30000);
    await mark("Executed explicit run now for heuristic discovery workflow");
    const jobsAfterRun2 = await invoke("list_jobs", { filter: {} });
    assert(jobsAfterRun2.total > jobsAfterRun1.total, "Discovery run must persist newly discovered jobs");
    assert(jobsAfterRun2.items.some(j => j.company === "Native Heuristic Discovery Inc"), "Persisted job must include Native Heuristic Discovery Inc");
    const consoleButton = await wait(() => document.getElementById("terminal-console-btn"), "Terminal console button");
    consoleButton.click();
    await pause(1000);
    await mark("Clicked the native Terminal console button");
    await fetch(${JSON.stringify(origin)} + "/enable-bulk");
    const bulk = await invoke("save_workflow", { spec: ${JSON.stringify(bulkSpec)}, id: null });
    const bulkRun = await invoke("run_source_now", { id: bulk.id });
    assert(bulkRun.counts.sources[0].examined === 1000 && bulkRun.counts.sources[0].rejected === 999 && bulkRun.counts.inserted === 1 && bulkRun.counts.sources[0].termination === "page_limit", "Bulk fixture accounting was inconsistent: " + JSON.stringify(bulkRun));
    await mark("Examined 1,000 Muse fixtures with 999 title rejections and one Phoenix insertion before the page bound");

    const allRuns = await invoke("list_scrape_runs", { limit: 100 });
    assert(allRuns.length >= 2, "Expected persisted scrape runs for both explicit executions");

    await mark("Verified native Preview -> Save -> Run for both company crawler and heuristic discovery workflows in isolated SQLite");
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
  const log = await readFile(join(directory, "scrape.log"), "utf8");
  const bulkCards = log.split("+-- CANDIDATE ").slice(1).filter(card => card.includes("| Company: Bulk Fixture\n"));
  if (bulkCards.length !== 1000 || bulkCards.filter(card => card.startsWith("1 | INSERTED --")).length !== 1 || !bulkCards.some(card => card.startsWith("1000 | REJECTED --"))) throw new Error(`Bulk fixture did not produce all 1,000 ordered outcome cards: ${bulkCards.length}; artifacts: ${directory}`);
  if (!result.ok) throw new Error(`Native web crawl/proxy smoke failed: ${JSON.stringify(result)}; isolated artifacts: ${directory}`);
  console.log(`Native web scraping smoke passed: ${JSON.stringify({ result, artifacts: directory })}`);
} finally {
  site.closeAllConnections();
  proxy.closeAllConnections();
  site.close();
  proxy.close();
}
