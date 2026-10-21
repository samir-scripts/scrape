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

  let pageOffset = 0;
  let itemIndex = 0;
  if (input.continuation) {
    try {
      const state = JSON.parse(input.continuation);
      if (typeof state.page_offset === "number") pageOffset = state.page_offset;
      if (typeof state.item_index === "number") itemIndex = state.item_index;
    } catch {
      process.stderr.write(`Invalid continuation cursor: ${input.continuation}\n`);
      process.exit(1);
    }
  }

  const fixturePath = input.config?.fixture_path || input.config?.script?.args?.[0] || process.argv[2];
  let data;
  let totalCount = null;

  if (fixturePath) {
    const raw = readFileSync(fixturePath, "utf8");
    data = JSON.parse(raw);
    totalCount = Number.isSafeInteger(data.hits) && data.hits >= 0 ? data.hits : null;
  } else {
    const parsedUrl = new URL(input.url);
    parsedUrl.searchParams.set("offset", String(pageOffset));
    if (!parsedUrl.searchParams.has("result_limit")) {
      parsedUrl.searchParams.set("result_limit", "10");
    }
    // Explicit loopback replay is verification transport, never a live fallback.
    let fetchUrl = parsedUrl.toString();
    if (process.env.SCRAPE_REPLAY_ORIGIN) {
      const replay = new URL(process.env.SCRAPE_REPLAY_ORIGIN);
      if (replay.protocol !== "http:" || !["127.0.0.1", "localhost", "[::1]"].includes(replay.hostname)) {
        throw new Error("SCRAPE_REPLAY_ORIGIN must be a loopback HTTP origin");
      }
      replay.pathname = `/${parsedUrl.host}${parsedUrl.pathname}`;
      replay.search = parsedUrl.search;
      fetchUrl = replay.toString();
    }

    const res = await fetch(fetchUrl, {
      headers: { "User-Agent": "Scrape/1.0 (Job Aggregator; +https://github.com/scrape)" },
      signal: AbortSignal.timeout(input.timeout_ms || 30000),
    });
    if (!res.ok) {
      process.stderr.write(`Provider API HTTP error: ${res.status} ${res.statusText}\n`);
      process.exit(1);
    }
    data = await res.json();
    totalCount = Number.isSafeInteger(data.hits) && data.hits >= 0 ? data.hits : null;
  }

  const jobs = data.jobs;
  if (!Array.isArray(jobs)) {
    process.stderr.write("Provider API response missing jobs array\n");
    process.exit(1);
  }

  const budget = Math.max(1, input.max_candidates || 10);
  let emitted = 0;
  let currIdx = itemIndex;

  while (currIdx < jobs.length && emitted < budget) {
    const job = jobs[currIdx];
    currIdx++;

    // Validate required fields without fabricated fallbacks
    const title = (job.title || "").trim();
    const externalId = String(job.id_icims || job.id || "").trim();
    const jobUrl = job.job_path
      ? (job.job_path.startsWith("http") ? job.job_path : `https://www.amazon.jobs${job.job_path}`)
      : (externalId ? `https://www.amazon.jobs/en/jobs/${externalId}` : "");

    const candidate = {
      type: "candidate",
      version: 2,
      external_id: externalId || null,
      title,
      url: jobUrl,
      location: job.normalized_location || job.location || null,
      company: job.company_name || "Amazon",
    };
    if (job.description) {
      candidate.description = job.description;
    }
    if (job.posted_date) {
      candidate.date_posted = job.posted_date;
    }
    if (job.preferred_qualifications) {
      const match = job.preferred_qualifications.match(/(\$[\d,]+(?:\s*-\s*\$[\d,]+)?(?:\s*(?:USD|per year|annually))?|[\d,]+\.\d{2}\s*-\s*[\d,]+\.\d{2}\s*USD\s*annually)/i);
      if (match) {
        candidate.salary = match[1];
      }
    }
    process.stdout.write(JSON.stringify(candidate) + "\n");
    emitted++;
  }

  let exhausted = false;
  let nextContinuation = null;

  if (currIdx < jobs.length) {
    nextContinuation = JSON.stringify({
      v: 2,
      page_offset: pageOffset,
      item_index: currIdx,
    });
  } else {
    const nextOffset = pageOffset + jobs.length;
    if (fixturePath || totalCount === null || jobs.length === 0) {
      exhausted = totalCount !== null && nextOffset >= totalCount;
    } else if (nextOffset >= totalCount) {
      exhausted = true;
    } else {
      nextContinuation = JSON.stringify({
        v: 2,
        page_offset: nextOffset,
        item_index: 0,
      });
    }
  }

  if (nextContinuation && !exhausted) {
    process.stdout.write(JSON.stringify({
      type: "continuation",
      version: 2,
      cursor: nextContinuation,
    }) + "\n");
  }

  process.stdout.write(JSON.stringify({
    type: "complete",
    version: 2,
    exhausted,
  }) + "\n");
}

main().catch((err) => {
  process.stderr.write(String(err?.stack || err) + "\n");
  process.exit(1);
});
