import { describe, it, expect, beforeEach } from "vitest";
import { api, __resetDevData, SAMPLE_ENGINEERING_SOURCE } from "../lib/api";

describe("Dev-Mode Scrape Workflow Execution & Discovered Pipeline", () => {
  beforeEach(() => {
    __resetDevData();
  });

  it("keeps browser fixture execution explicitly fictional and applies the engineering profile locally", async () => {
    // 1. Initial State: Inbox is empty
    const initialJobs = await api.listJobs({ statuses: ["scraped"] });
    expect(initialJobs.items).toHaveLength(0);
    expect(initialJobs.total).toBe(BigInt(0));

    // 2. Trigger workflow execution
    const run = await api.runSourceNow(SAMPLE_ENGINEERING_SOURCE.id);

    // Assert run finished successfully with jobs reported
    expect(run.status).toBe("success");
    expect(run.source_id).toBe(SAMPLE_ENGINEERING_SOURCE.id);
    expect(Number(run.jobs_found)).toBe(4);
    expect(Number(run.jobs_new)).toBe(3);

    // 3. Terminal Log Stream verification
    const logs = await api.getScrapeLogs(run.id);
    expect(logs).toHaveLength(4);

    expect(logs[0].level).toBe("info");
    expect(logs[0].message).toContain("Software Engineering & Related — Past 12 Hours");

    expect(logs[1].level).toBe("info");
    expect(logs[1].message).toContain("no provider requests were made");

    expect(logs[2].level).toBe("info");
    expect(logs[2].message).toContain("4 fictional postings generated");
    expect(logs[2].message).toContain("3 matched the saved workflow");

    expect(logs[3].level).toBe("info");
    expect(logs[3].message).toContain("Workflow run completed");
    expect(logs[3].message).toContain("SUCCESS");
    expect(logs[3].message).toContain("Committed 3 new postings to Discovered inbox");

    // 4. Discovered Inbox verification
    const discovered = await api.listJobs({ statuses: ["scraped"] });
    expect(discovered.items).toHaveLength(3);
    expect(discovered.total).toBe(BigInt(3));

    // Verify job structure and fields
    const titles = discovered.items.map((j) => j.title);
    expect(titles).toEqual(expect.arrayContaining([
      "Demo Company Software Engineer - Backend Systems",
      "Infrastructure Engineer - Platform & Reliability",
      "Full Stack Engineer - Product Experience",
    ]));

    for (const job of discovered.items) {
      expect(job.description_text).toContain("FICTIONAL DEMONSTRATION POSTING");
      expect(job.status).toBe("scraped");
      expect(job.source_id).toBe(SAMPLE_ENGINEERING_SOURCE.id);
      expect(job.url).toContain("https://example.com/demo/search/themuse/jobs/demo-");
      expect(job.salary_min).toBeGreaterThan(0);
    }

    // 5. Run History verification
    const runs = await api.listSourceRuns(SAMPLE_ENGINEERING_SOURCE.id);
    expect(runs).toHaveLength(1);
    expect(runs[0].id).toBe(run.id);
    expect(runs[0].status).toBe("success");
    expect(Number(runs[0].jobs_new)).toBe(3);

    // Source metadata updated
    const sources = await api.listSources();
    const cf = sources.find((s) => s.id === SAMPLE_ENGINEERING_SOURCE.id);
    expect(cf?.last_status).toBe("ok");
    expect(cf?.last_run_at).not.toBeNull();
  });

  it("supports interactive triage in dev mode (applying, favoriting, moving from Discovered)", async () => {
    // Run workflow to populate jobs
    await api.runSourceNow(SAMPLE_ENGINEERING_SOURCE.id);
    const jobs = await api.listJobs();
    expect(jobs.items).toHaveLength(3);

    const firstJob = jobs.items[0];

    // Favorite job
    const favorited = await api.toggleJobFavorite(firstJob.id);
    expect(favorited.favorited).toBe(true);

    const jobCheck = await api.getJob(firstJob.id);
    expect(jobCheck.favorited).toBe(true);

    // Apply for job (transitions from Discovered to Tracked)
    const updated = await api.updateJobStatus(firstJob.id, "applied");
    expect(updated.status).toBe("applied");

    // Check remaining in Discovered
    const inDiscovered = await api.listJobs({ statuses: ["scraped", "viewed"] });
    expect(inDiscovered.items).toHaveLength(2);

    // Check in Tracked (applied)
    const inTracked = await api.listJobs({ statuses: ["applied"] });
    expect(inTracked.items).toHaveLength(1);
    expect(inTracked.items[0].id).toBe(firstJob.id);

    // Metrics reflect triage
    const metrics = await api.getMetrics();
    expect(metrics.totals.scraped).toBe(2);
    expect(metrics.totals.applied).toBe(1);
    expect(metrics.totals.favorited).toBe(1);
  });

  it("handles workflow error execution lifecycle with error logs and failed status", async () => {
    // Create a source configured to fail
    const failingSource = await api.createSource({
      name: "Acme Fail Endpoint",
      identifier: "https://fail.example.com",
      kind: "greenhouse",
    });

    const run = await api.runSourceNow(failingSource.id);
    expect(run.status).toBe("failed");
    expect(run.error).toContain("demonstration mode; no network request was made");
    expect(Number(run.jobs_new)).toBe(0);

    // Logs contain warning and errors
    const logs = await api.getScrapeLogs(run.id);
    expect(logs).toHaveLength(4);

    const errorLogs = logs.filter((l) => l.level === "error");
    const warnLogs = logs.filter((l) => l.level === "warn");
    expect(errorLogs).toHaveLength(2);
    expect(warnLogs).toHaveLength(1);
    expect(errorLogs[0].message).toContain("Demonstration fixture failure");
    expect(errorLogs[1].message).toContain("Workflow run failed");

    // No jobs were added to discovered
    const jobs = await api.listJobs({ statuses: ["scraped"] });
    expect(jobs.items).toHaveLength(0);

    // Source status reflects failure
    const updatedSource = await api.getSource(failingSource.id);
    expect(updatedSource.last_status).toBe("error");
    expect(updatedSource.last_error).toContain("demonstration mode; no network request was made");
  });
});
