import { readFile } from "node:fs/promises";
import { test, expect } from "./fixtures";
import { contrastFailures } from "./contrast";

// Authored browser demonstration. Rust descriptions/execution are verified by
// host and packaged-worker tests, never inferred from this browser fixture.
for (const theme of ["paper", "obsidian"]) {
  test(`${theme} preserves visible compatibility restrictions and exports explicit edits`, async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem("scrape:demo:v1:sources", JSON.stringify([{
        id: "compatibility", kind: "greenhouse", name: "Authored compatibility fixture", identifier: "compatibility-fixture", config: {},
        enabled: true, created_at: "2026-10-18T00:00:00Z", cadence: "manual", cadence_cron: null, next_run_at: null,
        retention_days: 30, retention_policy: "untracked_only", needs_review: false, script_trusted: false,
        filters: { include_keywords: ["Software"], exclude_keywords: ["Staff"], locations: ["Remote", "Austin"], role_types: ["internship"],
          query: "title LIKE '%Engineer%' OR title LIKE '%Intern%'", policy: { company: "%Fixture%" },
          matching: { role_types: ["full_time"], keywords: { any: ["Engineer"], all: ["Software"], exclude: ["Senior"], fields: ["title"] } } },
      }]));
    });
    await page.setViewportSize({ width: 1024, height: 900 });
    await page.goto("/?demo=1&fixtures=1");
    await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
    if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
    await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
    await page.getByRole("button", { name: "Edit workflow Authored compatibility fixture", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Edit workflow" });
    await dialog.getByRole("button", { name: "Continue to Criteria" }).click();
    await expect(dialog.getByRole("region", { name: "Compatibility restrictions" })).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Remove Austin from Legacy location terms", exact: true })).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Migrate to structured rules" })).toHaveCount(0);
    await expect(dialog.getByText(/Compatibility explanations require the desktop validator/)).toBeVisible();
    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `/tmp/plan12-step3-compatibility-${theme}-1024.png` });
    const query = dialog.getByRole("textbox", { name: "Expert query", exact: true });
    await expect(query).toHaveValue("title LIKE '%Engineer%' OR title LIKE '%Intern%'");
    await query.fill("title LIKE '%Software%'");
    await dialog.getByRole("button", { name: "Remove legacy role-type restriction", exact: true }).click();
    await dialog.getByRole("button", { name: "Advanced", exact: true }).click();
    const downloading = page.waitForEvent("download");
    await dialog.getByRole("button", { name: "Export YAML", exact: true }).click();
    const yaml = await readFile((await (await downloading).path())!, "utf8");
    expect(yaml).toContain("title LIKE '%Software%'");
    expect(yaml).toContain("include_keywords:");
    expect(yaml).toContain("- Austin");
    expect(yaml).toContain("role_types: []");
    expect(yaml).toContain("- full_time");
    await page.setViewportSize({ width: 480, height: 900 });
    await dialog.getByRole("textbox", { name: "Legacy include keywords", exact: true }).scrollIntoViewIfNeeded();
    expect(await dialog.evaluate(element => element.scrollWidth <= element.clientWidth + 1)).toBe(true);
    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `/tmp/plan12-step3-compatibility-${theme}-480.png` });
  });
}
