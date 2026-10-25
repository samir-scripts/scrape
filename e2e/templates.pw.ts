import { test, expect } from "./fixtures";
import { readFileSync } from "node:fs";
import type { WorkflowTemplateCatalog } from "../src/bindings/WorkflowTemplateCatalog";
const catalog: WorkflowTemplateCatalog = JSON.parse(readFileSync(new URL("../src/generated/workflowTemplates.json", import.meta.url), "utf8"));
import { contrastFailures } from "./contrast";

// Offline browser demonstration only. Rust fixtures prove execution/persistence.
for (const theme of ["paper", "obsidian"]) {
  test(`${theme} loads and previews the current engineering workflow template without scraping`, async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 900 });
    await page.goto("/?demo=1&fixtures=1");
    await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
    if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
    await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
    for (const name of [
      "Software Engineering & Related — Past 12 Hours",
      "Fast Food — Cherry Hill, NJ — Past 30 Days",
      "Remote — Advertised Annual USD Pay Over $60,000",
    ]) await expect(page.getByRole("button", { name: `Edit workflow ${name}`, exact: true })).toBeVisible();

    const runsBefore = await page.evaluate(() => JSON.parse(localStorage.getItem("scrape:demo:v1:runs") || "[]").length);
    await page.getByRole("button", { name: "Add workflow", exact: true }).click();
    const dialog = page.getByRole("dialog", { name: "Create workflow" });
    const template = catalog.templates.find(template => template.id === "software-engineering-last-12-hours")!;
    await dialog.getByRole("button", { name: "Advanced", exact: true }).click();
    await dialog.getByRole("button", { name: "Load example workflow" }).click();
    await page.getByRole("option", { name: template.name, exact: true }).click();
    await dialog.getByRole("textbox", { name: "Workflow name" }).fill("E2E engineering profile");
    await dialog.getByRole("button", { name: "Continue to Criteria" }).click();
    await dialog.getByRole("button", { name: "Continue to Sources" }).click();
    await expect(dialog.getByText(/Adzuna requires personal API credentials/)).toBeVisible();
    await expect(dialog.getByText(/coverage.*not.*exhaustive|never every/i)).toBeVisible();
    await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
    await dialog.getByRole("button", { name: "Continue to Review" }).click();
    const summary = dialog.getByRole("region", { name: "Matching rules summary" });
    await expect(summary).toContainText("Software Engineer");
    await expect(summary).toContainText("past 12h");
    await expect(summary).toContainText("Exclude unknown dates");
    await expect(dialog.getByRole("button", { name: "Preview matches" })).toBeEnabled();
    await dialog.getByRole("button", { name: "Preview matches", exact: true }).click();
    await expect(dialog.getByRole("heading", { name: "3 eligible matches", exact: true })).toBeVisible();
    await dialog.getByRole("button", { name: "Save workflow", exact: true }).click();
    await expect(dialog.getByText(/Saving has not started a scrape/)).toBeVisible();
    await dialog.getByRole("button", { name: "Done", exact: true }).click();
    await expect(page.getByRole("button", { name: "Edit workflow E2E engineering profile", exact: true })).toBeVisible();
    expect(await page.evaluate(() => JSON.parse(localStorage.getItem("scrape:demo:v1:runs") || "[]").length)).toBe(runsBefore);
  });
}
