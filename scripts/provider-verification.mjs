// Real worker verification. Default: deterministic HTTP replay. --live: public APIs.
import { createServer } from "node:http";
import { spawn } from "node:child_process";
import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { existsSync } from "node:fs";
import { execFileSync } from "node:child_process";
const live = process.argv.includes("--live"),
  refresh = process.argv.includes("--refresh-example");
const providers = [
  {
    kind: "lever",
    name: "SEP",
    identifier: "sep",
    url: "https://api.lever.co/v0/postings/sep?mode=json",
    file: "fixtures/providers/lever/sep.json",
  },
  {
    kind: "ashby",
    name: "Linear",
    identifier: "linear",
    url: "https://api.ashbyhq.com/posting-api/job-board/linear?includeCompensation=true",
    file: "fixtures/providers/ashby/linear.json",
  },
];
if (live)
  providers.push({
    kind: "greenhouse",
    name: "Bitwarden",
    identifier: "bitwarden",
    url: "https://boards-api.greenhouse.io/v1/boards/bitwarden/jobs?content=true",
  });
const report = {
  mode: live ? "live provider" : "recorded HTTP replay",
  timestamp: new Date().toISOString(),
  commit: execFileSync("git", ["rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim(),
  checks: [],
};
const manifest = JSON.parse(
  await readFile("fixtures/providers/manifest.json", "utf8"),
);
for (const provider of providers) {
  const bytes = live
    ? Buffer.from(
        await (
          await fetch(provider.url, { signal: AbortSignal.timeout(30000) })
        ).text(),
      )
    : await readFile(provider.file);
  const checksum = createHash("sha256").update(bytes).digest("hex");
  if (!live && manifest[provider.identifier].sha256 !== checksum)
    throw new Error(
      "Recorded response checksum changed: " + provider.identifier,
    );
  const raw = JSON.parse(bytes);
  const postings = Array.isArray(raw) ? raw : raw.jobs;
  if (!postings?.length)
    throw new Error(
      "No source postings; cannot verify associations: " + provider.url,
    );
  let server, origin;
  if (!live) {
    server = createServer((req, res) => {
      if (
        req.url.startsWith(
          "/" + new URL(provider.url).host + new URL(provider.url).pathname,
        )
      ) {
        res.setHeader("Content-Type", "application/json");
        res.end(bytes);
      } else if (req.url.endsWith("/robots.txt")) {
        res.end("User-agent: *\nAllow: /");
      } else {
        res.statusCode = 404;
        res.end("Unrecorded request");
      }
    });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    origin = `http://127.0.0.1:${server.address().port}`;
  }
  const config = {
    version: 3,
    sources: [
      {
        id: "recorded-" + provider.identifier,
        kind: provider.kind,
        name: provider.name,
        identifier: provider.identifier,
        // Verification intentionally examines the complete captured response.
        config: { __workflow_spec: { max_jobs_per_run: 10000 } },
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
    limits: { concurrency: 1, per_host_delay_ms: 100, request_timeout_s: 10 },
  };
  try {
    const output = await new Promise((resolve, reject) => {
      const executable = `src-tauri/binaries/scrape-worker-${process.arch === "arm64" ? "aarch64" : "x86_64"}-${process.platform === "darwin" ? "apple-darwin" : "unknown-linux-gnu"}`;
      if (!existsSync(executable)) throw new Error("Build the packaged worker with npm run build:sidecar before provider verification.");
      const child = spawn(
        executable,
        ["--stdin", ...(origin ? ["--replay-origin", origin] : [])],
        { stdio: ["pipe", "pipe", "pipe"] },
      );
      let stdout = "",
        stderr = "";
      const timer = setTimeout(() => {
        child.kill("SIGKILL");
        reject(new Error("Worker timed out"));
      }, 70000);
      child.stdout.on("data", (b) => (stdout += b));
      child.stderr.on("data", (b) => (stderr += b));
      child.on("error", reject);
      child.on("close", (code) => {
        clearTimeout(timer);
        code === 0 ? resolve(stdout) : reject(new Error(stderr));
      });
      child.stdin.end(JSON.stringify(config) + "\n");
    });
    const events = output
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    const jobs = events.filter((e) => e.type === "job");
    const errors = events.filter((e) => e.type === "error");
    const done = events.at(-1);
    if (done?.type !== "done" || !jobs.length)
      throw new Error("Missing jobs/completion: " + provider.identifier);
    for (const job of jobs) {
      const source = postings.find((p) => String(p.id) === job.external_id);
      if (
        !source ||
        job.title !== (source.text || source.title) ||
        job.url !== (source.hostedUrl || source.jobUrl || source.absolute_url)
      )
        throw new Error(
          "Provider identity/title/destination changed: " + job.external_id,
        );
    }
    if (errors.length)
      throw new Error("Incomplete extraction: " + JSON.stringify(errors));
    if (jobs.length !== postings.length)
      throw new Error("Missing source postings");
    report.checks.push({
      source: provider.url,
      kind: provider.kind,
      captured_at: live
        ? report.timestamp
        : manifest[provider.identifier].captured_at,
      sha256: checksum,
      fetched: postings.length,
      verified: jobs.length,
      errors: 0,
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
  } finally {
    if (server) await new Promise((resolve) => server.close(resolve));
  }
}
console.log(JSON.stringify(report, null, 2));
