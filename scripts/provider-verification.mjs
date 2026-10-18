// Real worker verification. Default: deterministic HTTP replay. --live: public APIs.
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";

// Adapter configurations are projections of Rust's canonical extractor definitions.
// This remains a direct-worker smoke check; full workflow acceptance is Plan 12 §7.
const templateCatalog = JSON.parse(await readFile(new URL("../src/generated/workflowTemplates.json", import.meta.url), "utf8"));
function extractor(id) {
  const entry = templateCatalog.templates.filter(template => template.seed).flatMap(template => template.extractors).find(source => source.id === id);
  if (!entry) throw new Error(`No canonical extractor for ${id}`);
  return entry;
}
function extractorConfig(id) {
  const config = structuredClone(extractor(id).config);
  if (config.script) config.script.interpreter = process.execPath; // Explicit fixture runtime.
  config.__workflow_spec = { max_jobs_per_run: 100 };
  return config;
}

const live = process.argv.includes("--live");
const refresh = process.argv.includes("--refresh-example");

const providers = [
  {
    kind: "lever",
    name: "SEP",
    identifier: extractor("sep").identifier,
    url: "https://api.lever.co/v0/postings/sep?mode=json",
    file: "fixtures/providers/lever/sep.json",
    classification: "native_ats",
    config: extractorConfig("sep"),
  },
  {
    kind: "ashby",
    name: "Linear",
    identifier: extractor("linear").identifier,
    url: "https://api.ashbyhq.com/posting-api/job-board/linear?includeCompensation=true",
    file: "fixtures/providers/ashby/linear.json",
    classification: "native_ats",
    config: extractorConfig("linear"),
  },
  {
    kind: "greenhouse",
    name: "Bitwarden",
    identifier: extractor("bitwarden").identifier,
    url: "https://boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true",
    file: "fixtures/providers/greenhouse/bitwarden.json",
    classification: "native_ats",
    config: extractorConfig("bitwarden"),
  },
  {
    kind: "selector",
    mode: "static",
    name: "Google",
    identifier: "google",
    url: extractor("google").config.url,
    file: "fixtures/providers/selector/google.html",
    classification: "selector_static",
    config: extractorConfig("google"),
  },
  {
    kind: "selector",
    mode: "static",
    name: "Apple",
    identifier: "apple",
    url: extractor("apple").config.url,
    file: "fixtures/providers/selector/apple.html",
    classification: "selector_static",
    config: extractorConfig("apple"),
  },
  {
    kind: "selector",
    mode: "script",
    name: "Netflix",
    identifier: "netflix",
    url: extractor("netflix").config.url,
    file: "fixtures/providers/script/netflix.json",
    classification: "script_ndjson",
    config: extractorConfig("netflix"),
  },
  {
    kind: "selector",
    mode: "script",
    name: "Amazon",
    identifier: "amazon",
    url: extractor("amazon").config.url,
    file: "fixtures/providers/script/amazon.json",
    classification: "script_ndjson",
    config: extractorConfig("amazon"),
  },
  {
    kind: "selector",
    mode: "browser",
    name: "Meta",
    identifier: "meta",
    url: "https://www.metacareers.com/jobs",
    file: "fixtures/providers/meta/meta.html",
    classification: "dynamic_spa_bot_shielded",
    config: {
      mode: "static",
      item: ".job-item",
      title: ".job-title",
      link: "a",
      __workflow_spec: { max_jobs_per_run: 100 },
    },
    note: "Meta Careers returns an empty React/Relay client container (<div id=\"root\"></div>) in initial HTML. Documented as an open acceptance gap per Plan 11.",
  },
];

const executable = `src-tauri/binaries/scrape-worker-${process.arch === "arm64" ? "aarch64" : "x86_64"}-${process.platform === "darwin" ? "apple-darwin" : "unknown-linux-gnu"}`;
if (!existsSync(executable)) throw new Error("Build the packaged worker with npm run build:sidecar before provider verification.");

const protocolVersion = Number(execFileSync(executable, ["--protocol-version"], { encoding: "utf8", timeout: 5000 }).trim());
if (!Number.isInteger(protocolVersion) || protocolVersion <= 0) throw new Error("Worker returned an invalid protocol version");

const report = {
  mode: live ? "live provider" : "recorded HTTP replay",
  timestamp: new Date().toISOString(),
  commit: execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim(),
  checks: [],
};

const manifest = JSON.parse(await readFile("fixtures/providers/manifest.json", "utf8"));

for (const provider of providers) {
  let bytes;
  if (live) {
    // In live mode, fetch from live URL
    const res = await fetch(provider.url, {
      headers: { "User-Agent": "Scrape/1.0 (Job Aggregator; +https://github.com/scrape)" },
      signal: AbortSignal.timeout(30000),
    });
    bytes = Buffer.from(await res.text());
  } else {
    bytes = await readFile(provider.file);
  }

  const checksum = createHash("sha256").update(bytes).digest("hex");
  if (!live && manifest[provider.identifier]?.sha256 !== checksum) {
    throw new Error(`Recorded response checksum changed: ${provider.identifier} (expected ${manifest[provider.identifier]?.sha256}, got ${checksum})`);
  }

  let server, origin;
  if (!live && provider.classification !== "script_ndjson") {
    server = createServer((req, res) => {
      const u = new URL(provider.url);
      if (req.url.startsWith("/" + u.host + u.pathname) || req.url.startsWith(u.pathname)) {
        res.setHeader("Content-Type", provider.file.endsWith(".html") ? "text/html; charset=utf-8" : "application/json");
        res.end(bytes);
      } else if (req.url.endsWith("/robots.txt")) {
        res.setHeader("Content-Type", "text/plain");
        res.end("User-agent: *\nAllow: /");
      } else {
        // Fallback match for query-stripped paths
        res.setHeader("Content-Type", provider.file.endsWith(".html") ? "text/html; charset=utf-8" : "application/json");
        res.end(bytes);
      }
    });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    origin = `http://127.0.0.1:${server.address().port}`;
  }

  const sourceConfig = {
    ...(provider.config || { __workflow_spec: { max_jobs_per_run: 10000 } }),
  };
  if (!live && provider.classification === "script_ndjson") {
    sourceConfig.fixture_path = provider.file;
  }

  const config = {
    version: protocolVersion,
    sources: [
      {
        id: "recorded-" + provider.identifier,
        kind: provider.kind,
        name: provider.name,
        identifier: provider.kind === "selector" ? provider.url : provider.identifier,
        config: sourceConfig,
      },
    ],
    filters: {
      include_keywords: [],
      exclude_keywords: [],
      locations: [],
      role_types: [],
      query: null,
      policy: null,
      compensation: null,
      matching: null,
    },
    filters_by_source: {},
    run_started_at: report.timestamp,
    limits: { concurrency: 1, per_host_delay_ms: 0, request_timeout_s: 10 },
  };

  try {
    const output = await new Promise((resolve, reject) => {
      const child = spawn(
        executable,
        ["--stdin", ...(origin ? ["--replay-origin", origin] : [])],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      let stdout = "", stderr = "";
      const timer = setTimeout(() => {
        child.kill("SIGKILL");
        reject(new Error("Worker timed out: " + provider.identifier));
      }, 70000);
      child.stdout.on("data", (b) => (stdout += b));
      child.stderr.on("data", (b) => (stderr += b));
      child.on("error", reject);
      child.on("close", (code) => {
        clearTimeout(timer);
        code === 0 ? resolve(stdout) : reject(new Error(`Worker exited ${code}: ${stderr}`));
      });
      child.stdin.end(JSON.stringify(config) + "\n");
    });

    const events = output
      .trim()
      .split("\n")
      .filter(Boolean)
      .map((line) => JSON.parse(line));
    const jobs = events.filter((e) => e.type === "job");
    const errors = events.filter((e) => e.type === "error");
    const done = events.at(-1);

    if (done?.type !== "done") {
      throw new Error("Missing terminal completion event for: " + provider.identifier);
    }

    // Provider specific validation
    if (provider.identifier === "meta") {
      // Meta is expected to produce 0 jobs from static HTML shell
      if (jobs.length !== 0) {
        throw new Error("Expected 0 jobs from Meta client-side SPA shell, got: " + jobs.length);
      }
      report.checks.push({
        source: provider.url,
        kind: provider.kind,
        classification: provider.classification,
        captured_at: live ? report.timestamp : manifest[provider.identifier].captured_at,
        sha256: checksum,
        fetched: 0,
        verified: 0,
        errors: 0,
        note: provider.note,
      });
    } else {
      if (!jobs.length) {
        throw new Error("Missing jobs: " + provider.identifier);
      }

      // Check identity, titles, URLs
      for (const job of jobs) {
        if (!job.title || typeof job.title !== "string" || !job.title.trim()) {
          throw new Error(`Blank title in job: ${JSON.stringify(job)}`);
        }
        if (!job.url || !job.url.startsWith("http")) {
          throw new Error(`Invalid destination URL: ${job.url}`);
        }
      }

      // 1-to-1 matching against raw payload where applicable
      let fetchedCount = jobs.length;
      if (provider.classification === "native_ats") {
        const raw = JSON.parse(bytes);
        const postings = Array.isArray(raw) ? raw : (raw.jobs || []);
        fetchedCount = postings.length;
        for (const job of jobs) {
          const source = postings.find((p) => String(p.id) === job.external_id);
          if (
            !source ||
            job.title !== (source.text || source.title) ||
            job.url !== (source.hostedUrl || source.jobUrl || source.absolute_url)
          ) {
            throw new Error(`Provider identity/title/destination changed for ${job.external_id}`);
          }
        }
      } else if (provider.classification === "script_ndjson") {
        const raw = JSON.parse(bytes);
        const postings = raw.positions || raw.jobs || [];
        fetchedCount = postings.length;
        for (const job of jobs) {
          const source = postings.find((p) =>
            String(p.id_icims || p.ats_job_id || p.id) === job.external_id ||
            String(p.id) === job.external_id
          );
          if (!source) {
            throw new Error(`Could not find source posting for external_id ${job.external_id}`);
          }
        }
      }

      report.checks.push({
        source: provider.url,
        kind: provider.kind,
        classification: provider.classification,
        captured_at: live ? report.timestamp : manifest[provider.identifier].captured_at,
        sha256: checksum,
        fetched: fetchedCount,
        verified: jobs.length,
        errors: errors.length,
      });

      if (refresh && !live && provider.identifier === "sep") {
        await writeFile(
          "src/lib/recordedExample.json",
          JSON.stringify(
            {
              provenance: manifest.sep,
              source: { ...provider, id: config.sources[0].id },
              events: jobs,
            },
            null,
            2,
          ) + "\n",
        );
      }
    }
  } catch (err) {
    if (live) {
      report.checks.push({
        source: provider.url,
        kind: provider.kind,
        classification: provider.classification,
        captured_at: report.timestamp,
        sha256: checksum,
        fetched: 0,
        verified: 0,
        errors: 1,
        note: `Live provider verification blocked or failed: ${err.message}`,
      });
    } else {
      throw err;
    }
  } finally {
    if (server) await new Promise((resolve) => server.close(resolve));
  }
}

console.log(JSON.stringify(report, null, 2));
