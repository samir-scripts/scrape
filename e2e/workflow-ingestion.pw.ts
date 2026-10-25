import { test, expect } from "./fixtures";
import type { ScrapeRun } from "../src/bindings/ScrapeRun";
import type { ScrapeLogEntry } from "../src/bindings/ScrapeLogEntry";

type PersistedDemoRun = Omit<ScrapeRun, "jobs_found" | "jobs_new"> & { jobs_found: number; jobs_new: number };

// Fictional browser fixtures exercise the rendered profile and local demo matcher only.
for (const theme of ["paper", "obsidian"]) {
  test(`${theme} current engineering profile → previewed fixture run → Discovered`, async ({ page }) => {
    const providerRequests: string[] = [];
    page.on("request", request => {
      if (/https:\/\/(api\.adzuna\.com|jobicy\.com|www\.themuse\.com|www\.arbeitnow\.com)\//i.test(request.url())) {
        providerRequests.push(request.url());
      }
    });
    await page.goto("/?demo=1&fixtures=1");
    if (theme === "obsidian") {
      await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
      await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
    }
    await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
    const engineeringName = "Software Engineering & Related — Past 12 Hours";
    const row = page.getByRole("row").filter({ hasText: engineeringName });
    await expect(row).toHaveCount(1);
    await expect(page.getByRole("button", { name: "Edit workflow Fast Food — Cherry Hill, NJ — Past 30 Days", exact: true })).toBeVisible();
    await expect(page.getByRole("button", { name: "Edit workflow Remote — Advertised Annual USD Pay Over $60,000", exact: true })).toBeVisible();
    const readRuns = () => page.evaluate(() =>
      JSON.parse(localStorage.getItem("scrape:demo:v1:runs") || "[]") as PersistedDemoRun[]);
    const previousRuns = await readRuns();
    const previousIds = previousRuns.map(item => item.id);
    await row.getByRole("button", { name: "RUN", exact: true }).click();
    await expect(page.getByText(/WORKFLOW OBSERVABILITY & EXECUTION LOGS/)).toBeVisible();
    await expect.poll(async () => (await readRuns()).filter(item => !previousIds.includes(item.id)).map(item => item.status)).toEqual(["success"]);
    const completed = (await readRuns()).find(item => !previousIds.includes(item.id))!;
    expect(completed).toMatchObject({
      source_id: "demo-software-engineering",
      trigger: "single_source",
      status: "success",
      error: null,
      jobs_found: 4,
      jobs_new: 3,
    });
    const logs = await page.evaluate(runId =>
      (JSON.parse(localStorage.getItem("scrape:demo:v1:logs") || "{}")[runId] || []) as ScrapeLogEntry[], completed.id);
    expect(logs).toHaveLength(4);
    expect(logs[1].message).toContain("no provider requests were made");
    expect(logs[2].message).toBe("Demonstration fixtures: 4 fictional postings generated; 3 matched the saved workflow.");
    const jobs = await page.evaluate(() => JSON.parse(localStorage.getItem("scrape:demo:v1:jobs") || "[]"));
    expect(jobs).toHaveLength(3);
    expect(jobs.map((job: { title: string }) => job.title).sort()).toEqual([
      "Demo Company Software Engineer - Backend Systems",
      "Full Stack Engineer - Product Experience",
      "Infrastructure Engineer - Platform & Reliability",
    ].sort());
    expect(jobs.every((job: { description_text: string }) => job.description_text.includes("FICTIONAL DEMONSTRATION POSTING"))).toBe(true);
    expect(providerRequests).toEqual([]);
    await page.getByRole("button", { name: /^DISCOVERED/ }).click();
    await expect(page.getByRole("button", { name: /^View / })).toHaveCount(3);
  });
}
