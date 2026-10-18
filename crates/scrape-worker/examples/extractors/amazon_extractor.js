#!/usr/bin/env node
// Amazon Jobs API extractor speaking SCRIPT_PROTOCOL_VERSION = 2

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

  const jobs = data.jobs || [];
  const limit = Math.min(jobs.length, input.max_candidates || 100);

  for (let i = 0; i < limit; i++) {
    const job = jobs[i];
    const candidate = {
      type: "candidate",
      version: 2,
      external_id: String(job.id_icims || job.id || ""),
      title: job.title || "Untitled Role",
      url: job.job_path
        ? (job.job_path.startsWith("http") ? job.job_path : `https://www.amazon.jobs${job.job_path}`)
        : `https://www.amazon.jobs/en/jobs/${job.id_icims || job.id}`,
      location: job.normalized_location || job.location || null,
      company: job.company_name || "Amazon",
    };
    if (job.description) {
      candidate.description = job.description;
    }
    if (job.posted_date) {
      candidate.date_posted = job.posted_date;
    }
    // Extract base salary from preferred qualifications or description if present
    if (job.preferred_qualifications) {
      const match = job.preferred_qualifications.match(/(\$[\d,]+(?:\s*-\s*\$[\d,]+)?(?:\s*(?:USD|per year|annually))?|[\d,]+\.\d{2}\s*-\s*[\d,]+\.\d{2}\s*USD\s*annually)/i);
      if (match) {
        candidate.salary = match[1];
      }
    }
    process.stdout.write(JSON.stringify(candidate) + "\n");
  }

  const complete = {
    type: "complete",
    version: 2,
    exhausted: limit >= jobs.length,
  };
  process.stdout.write(JSON.stringify(complete) + "\n");
}

main().catch((err) => {
  process.stderr.write(String(err?.stack || err) + "\n");
  process.exit(1);
});
