#!/usr/bin/env node
// Netflix Eightfold API extractor speaking SCRIPT_PROTOCOL_VERSION = 2

async function main() {
  // Works as a packaged temporary .js file and as an imported ES module.
  const { readFileSync } = await import("node:fs");
  const inputChunks = [];
  for await (const chunk of process.stdin) {
    inputChunks.push(chunk);
  }
  const inputStr = Buffer.concat(inputChunks).toString("utf8").trim();
  if (!inputStr) {
    process.stderr.write("No input received on stdin\n");
    process.exit(1);
  }

  const input = JSON.parse(inputStr);
  if (input.version !== 2) {
    process.stderr.write(`Unsupported protocol version: ${input.version}\n`);
    process.exit(1);
  }

  let data;
  const fixturePath = input.config?.fixture_path || input.config?.script?.args?.[0] || process.argv[2];
  if (fixturePath) {
    const raw = readFileSync(fixturePath, "utf8");
    data = JSON.parse(raw);
  } else {
    const res = await fetch(input.url, {
      headers: { "User-Agent": "Scrape/1.0 (Job Aggregator; +https://github.com/scrape)" },
      signal: AbortSignal.timeout(input.timeout_ms || 30000),
    });
    if (!res.ok) {
      process.stderr.write(`HTTP error: ${res.status} ${res.statusText}\n`);
      process.exit(1);
    }
    data = await res.json();
  }

  const positions = data.positions || [];
  const limit = Math.min(positions.length, input.max_candidates || 100);

  for (let i = 0; i < limit; i++) {
    const pos = positions[i];
    const candidate = {
      type: "candidate",
      version: 2,
      external_id: String(pos.id || pos.ats_job_id || ""),
      title: pos.name || pos.posting_name || "Untitled Role",
      url: pos.canonicalPositionUrl || `https://explore.jobs.netflix.net/careers/job/${pos.id}`,
      location: Array.isArray(pos.locations) && pos.locations.length > 0
        ? pos.locations.join("; ")
        : (pos.location || null),
      company: "Netflix",
      // Observed in the recorded Netflix contract; no arrangement is inferred
      // from a physical office address or a continental location label.
      workplace_type: pos.work_location_option === "remote_local" ? "remote"
        : ["onsite", "remote", "hybrid"].includes(pos.work_location_option)
          ? pos.work_location_option : "unknown",
    };
    if (pos.job_description) {
      candidate.description = pos.job_description;
    }
    if (pos.t_create) {
      candidate.date_posted = new Date(pos.t_create * 1000).toISOString();
    }
    process.stdout.write(JSON.stringify(candidate) + "\n");
  }

  const complete = {
    type: "complete",
    version: 2,
    exhausted: limit >= positions.length,
  };
  process.stdout.write(JSON.stringify(complete) + "\n");
}

main().catch((err) => {
  process.stderr.write(String(err?.stack || err) + "\n");
  process.exit(1);
});
