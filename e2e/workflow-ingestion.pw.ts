import { test, expect } from "./fixtures";
import type { ScrapeRun } from "../src/bindings/ScrapeRun";
import type { ScrapeLogEntry } from "../src/bindings/ScrapeLogEntry";

// Demo storage serializes the generated contract's bigint counters as numbers.
type PersistedDemoRun = Omit<ScrapeRun, "jobs_found" | "jobs_new"> & { jobs_found: number; jobs_new: number };

// Synthetic browser/demo UI acceptance only. No native/provider acceptance claim.
for (const theme of ["paper", "obsidian"]) {
  test(`${theme} demo seed → Run → diagnostics → Discovered preserves restrictive rules and suppression`, async ({ page }) => {
    await page.goto("/?demo=1&fixtures=1");
    if (theme === "obsidian") {
      await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
      await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
    }
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    await page.getByRole("button", { name: "Load test workflows" }).click();
    await expect(page.getByText(/Scripts remain disabled until reviewed and explicitly enabled/)).toBeVisible();
    const row = page.getByRole("row").filter({ hasText: "Cloudflare — Software Engineering Internships" });
    await expect(row).toHaveCount(1);
    const sourceId = "sample-swe-internships-1";
    const readRuns = () => page.evaluate(() =>
      JSON.parse(localStorage.getItem("scrape:demo:v1:runs") || "[]") as PersistedDemoRun[]);
    const run = async (inserted: number, refreshed: number) => {
      const previous = await readRuns();
      const previousIds = previous.map(item => item.id);
      // Registry RUN executes immediately and opens the dock; do not execute again from the dock.
      await row.getByRole("button", { name: "RUN", exact: true }).click();
      await expect(page.getByText(/WORKFLOW OBSERVABILITY & EXECUTION LOGS/)).toBeVisible();
      await expect.poll(async () => (await readRuns()).filter(item => !previousIds.includes(item.id)).map(item => item.status)).toEqual(["success"]);
      const added = (await readRuns()).filter(item => !previousIds.includes(item.id));
      expect(added).toHaveLength(1);
      const completed = added[0];
      expect(completed).toMatchObject({
        source_id: sourceId, trigger: "single_source", status: "success", error: null,
        finished_at: expect.any(String), jobs_found: 8, jobs_new: inserted,
      });
      const logs = await page.evaluate(runId =>
        (JSON.parse(localStorage.getItem("scrape:demo:v1:logs") || "{}")[runId] || []) as ScrapeLogEntry[], completed.id);
      expect(logs).toHaveLength(4);
      expect(logs.every(entry => entry.run_id === completed.id && entry.source_id === sourceId && entry.level === "info")).toBe(true);
      expect(logs[2].message).toBe("Extracted 8 postings from source. Policy engine matched 2 postings against criteria.");
      const terminal = logs[3].message;
      expect(terminal).toContain("with status: SUCCESS.");
      expect(terminal).toContain(inserted
        ? `Committed ${inserted} new postings to Discovered inbox (${refreshed} existing postings refreshed).`
        : `0 new postings found (${refreshed} existing postings refreshed). Discovered inbox preserved without duplicates.`);
      await expect(page.getByText(terminal, { exact: true })).toBeVisible();
      await expect(row.getByTestId(`status-${sourceId}`)).toContainText("SUCCESS");
      await expect(row.getByRole("button", { name: "RUN", exact: true })).toBeEnabled();
      await expect(page.getByRole("button", { name: "RUN WORKFLOW NOW", exact: true })).toBeEnabled();
      const recorded = await readRuns();
      expect(recorded).toHaveLength(previous.length + 1);
      expect(recorded.filter(item => !previousIds.includes(item.id)).map(item => item.id)).toEqual([completed.id]);
      return completed;
    };
    const first = await run(2, 0);
    await page.getByRole("button", { name: /^DISCOVERED/ }).click();
    const search = page.getByRole("textbox", { name: "Search jobs" });
    await search.fill("Software Engineer Intern");
    const view = page.getByRole("button", { name: /^View Software Engineer Intern \(2027\) - Austin, TX/ });
    await expect(view).toBeVisible();
    await expect(page.getByRole("button", { name: /^View / })).toHaveCount(1);
    await page.reload();
    await expect(page.getByRole("textbox", { name: "Search jobs" })).toHaveValue("Software Engineer Intern");
    await expect(view).toBeVisible();
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    const refreshed = await run(0, 2);
    await page.getByRole("button", { name: /^DISCOVERED/ }).click();
    await expect(view).toBeVisible();
    await page.getByRole("button", { name: /^Remove Software Engineer Intern \(2027\) - Austin, TX/ }).click();
    const removal = page.getByRole("dialog");
    await removal.getByRole("checkbox", { name: "Keep this job out of future scrapes" }).check();
    await removal.getByRole("button", { name: "Remove job", exact: true }).click();
    await expect(view).toHaveCount(0);
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    const suppressed = await run(0, 1);
    await page.getByRole("button", { name: /^DISCOVERED/ }).click();
    await expect(page.getByRole("textbox", { name: "Search jobs" })).toHaveValue("Software Engineer Intern");
    await expect(view).toHaveCount(0);
    await page.reload();
    await expect(view).toHaveCount(0);
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    await row.getByRole("button", { name: "Edit workflow Cloudflare — Software Engineering Internships" }).click();
    const edit = page.getByRole("dialog", { name: "Edit workflow" });
    await edit.getByRole("button", { name: "Continue to Criteria" }).click();
    for (const term of ["Software", "Engineer", "Intern"]) {
      await expect(edit.getByRole("button", { name: `Remove ${term} from Legacy include keywords` })).toBeVisible();
    }
    await edit.getByRole("button", { name: "Cancel", exact: true }).click();
    expect((await readRuns()).map(item => item.id)).toEqual([suppressed.id, refreshed.id, first.id]);
  });
}
