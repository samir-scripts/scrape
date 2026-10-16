import { test, expect } from "./fixtures";
import { contrastFailures } from "./contrast";
for (const theme of ["paper", "obsidian"]) {
  test(`${theme} distinguishes successful warnings, incomplete eligibility and cancellation`, async ({ page }) => {
    await page.addInitScript(() => {
      const sources = ["warning", "partial", "cancelled"].map((id, index) => ({
        id, kind: "greenhouse", name: `Authored ${id} fixture`, identifier: `fixture-${index}`, config: {},
        enabled: true, last_run_at: "2026-10-16T00:00:00Z", last_status: index ? "error" : "success",
        last_outcome: index === 0 ? "success_with_warnings" : id,
        last_error: index === 0 ? "Optional detail enrichment unavailable." : "Fixture outcome",
        created_at: "2026-10-16T00:00:00Z", cadence: "manual", cadence_cron: null, next_run_at: null,
        last_run_duration_ms: 5, retention_days: 30, retention_policy: "untracked_only", filters: {}, needs_review: false,
      }));
      const scan = { source_id: "warning", limit: 20, examined: 10, matched: 2, accepted: 2, rejected: 8, blocked: 0, invalid: 0, duplicates: 0, unresolved: 0, limit_reached: false, provider_total: 20, rejections: [{ reason: "keyword_any", count: 8 }], rejection_examples: [], enrichment_warnings: 1, outcome: "success_with_warnings", termination: "time_limit" };
      localStorage.setItem("scrape:demo:v1:sources", JSON.stringify(sources));
      localStorage.setItem("scrape:demo:v1:runs", JSON.stringify([{ id: "warning-run", source_id: "warning", trigger: "single_source", status: "success_with_warnings", started_at: "2026-10-16T00:00:00Z", finished_at: "2026-10-16T00:01:00Z", jobs_found: 2, jobs_new: 2, error: "Optional detail enrichment unavailable.", counts: { fetched: 10, rejected: 8, inserted: 2, updated: 0, blocked: 0, failed: 0, retention_removed: 0, enrichment_warnings: 1, sources: [scan] } }]));
    });
    await page.setViewportSize({ width: 1024, height: 900 });
    await page.clock.setFixedTime(new Date("2026-10-16T00:00:00Z"));
    await page.goto("/?demo=1&fixtures=1");
    await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
    if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    await expect(page.getByTestId("status-warning")).toContainText("SUCCESS WITH WARNINGS");
    await expect(page.getByTestId("status-partial")).toContainText("PARTIAL");
    await expect(page.getByTestId("status-cancelled")).toContainText("CANCELLED");
    await page.getByRole("button", { name: "ERRORS (1)", exact: true }).click();
    await expect(page.getByRole("button", { name: "Edit workflow Authored partial fixture" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Edit workflow Authored cancelled fixture" })).toHaveCount(0);
    await page.getByRole("button", { name: "ALL (3)", exact: true }).click();
    const row = page.getByRole("row").filter({ hasText: "Authored warning fixture" });
    await row.getByRole("button", { name: /More actions for/ }).click();
    await page.getByRole("menuitem", { name: "View run history" }).click();
    await page.getByText("Provider-reported total: 20", { exact: true }).scrollIntoViewIfNeeded();
    await expect(page.getByText(/0 unresolved · 1 enrichment warnings/)).toBeVisible();
    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `/tmp/scrape-enrichment-${theme}-1024.png` });
  });
}
