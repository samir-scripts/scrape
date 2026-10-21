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
    totalCount = Number.isSafeInteger(data.count) && data.count >= 0 ? data.count : null;
  } else {
    const parsedUrl = new URL(input.url);
    parsedUrl.searchParams.set("start", String(pageOffset));
    if (!parsedUrl.searchParams.has("num")) {
      parsedUrl.searchParams.set("num", "10");
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

    let res;
    try {
      res = await fetch(fetchUrl, {
        headers: { "User-Agent": "Scrape/1.0 (Job Aggregator; +https://github.com/scrape)" },
        signal: AbortSignal.timeout(input.timeout_ms || 30000),
      });
    } catch (err) {
      process.stderr.write(`Netflix careers portal endpoint unreachable (${err.message}); verify network or update URL to active portal\n`);
      process.exit(1);
    }
    if (!res.ok) {
      process.stderr.write(`Provider API HTTP error: ${res.status} ${res.statusText}\n`);
      process.exit(1);
    }
    data = await res.json();
    totalCount = Number.isSafeInteger(data.count) && data.count >= 0 ? data.count : null;
  }

  const positions = data.positions;
  if (!Array.isArray(positions)) {
    process.stderr.write("Provider API response missing positions array\n");
    process.exit(1);
  }

  const budget = Math.max(1, input.max_candidates || 10);
  let emitted = 0;
  let currIdx = itemIndex;

  while (currIdx < positions.length && emitted < budget) {
    const pos = positions[currIdx];
    currIdx++;

    // Validate required fields without fabricated fallbacks
    const title = (pos.name || pos.posting_name || "").trim();
    const externalId = String(pos.id || pos.ats_job_id || "").trim();
    const jobUrl = pos.canonicalPositionUrl || (pos.id ? `https://explore.jobs.netflix.net/careers/job/${pos.id}` : "");

    const candidate = {
      type: "candidate",
      version: 2,
      external_id: externalId || null,
      title,
      url: jobUrl,
      location: Array.isArray(pos.locations) && pos.locations.length > 0
        ? pos.locations.join("; ")
        : (pos.location || null),
      company: "Netflix",
      workplace_type: pos.work_location_option === "remote_local" ? "remote"
        : ["onsite", "remote", "hybrid"].includes(pos.work_location_option)
          ? pos.work_location_option : "unknown",
    };
    if (pos.job_description) {
      candidate.description = pos.job_description;
    }
    // Note: t_create / t_update are internal ATS creation/edit timestamps,
    // not verified publication dates.

    process.stdout.write(JSON.stringify(candidate) + "\n");
    emitted++;
  }

  let exhausted = false;
  let nextContinuation = null;

  if (currIdx < positions.length) {
    nextContinuation = JSON.stringify({
      v: 2,
      page_offset: pageOffset,
      item_index: currIdx,
    });
  } else {
    const nextOffset = pageOffset + positions.length;
    if (fixturePath || totalCount === null || positions.length === 0) {
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
