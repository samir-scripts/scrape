import { test, expect } from "./fixtures";
import { readFileSync } from "node:fs";
import type { WorkflowTemplateCatalog } from "../src/bindings/WorkflowTemplateCatalog";
const catalog: WorkflowTemplateCatalog = JSON.parse(readFileSync(new URL("../src/generated/workflowTemplates.json", import.meta.url), "utf8"));
import { contrastFailures } from "./contrast";

// Offline browser demonstration only. Rust fixtures prove execution/persistence.
for (const theme of ["paper", "obsidian"]) {
  test(`${theme} selects a canonical template, saves unknown choices and reopens them`, async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 900 });
    await page.goto("/?demo=1&fixtures=1");
    await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
    if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    await page.getByRole("button", { name: /add workflow|new workflow|create workflow/i }).click();
    const dialog = page.getByRole("dialog", { name: "Create workflow" });
    const template = catalog.templates.find(template => template.id === "sep-midwest-consulting")!;
    await dialog.getByRole("button", { name: "Choose sample workflow template" }).click();
    await page.getByRole("option", { name: template.name, exact: true }).click();
    await expect(dialog.getByText(template.description)).toBeVisible();
    await dialog.getByRole("button", { name: "Continue to Matching" }).click();
    await dialog.getByRole("checkbox", { name: "Internship", exact: true }).check();
    await dialog.getByRole("checkbox", { name: "Unknown employment", exact: true }).check();
    await dialog.getByRole("checkbox", { name: "Remote", exact: true }).check();
    await dialog.getByRole("checkbox", { name: "Unknown workplace", exact: true }).check();
    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `plans/assets/11-template-catalog-review/${theme}-matching-1024.png` });
    await dialog.getByRole("button", { name: "Preview matches", exact: true }).click();
    await expect(dialog.getByRole("heading", { name: "1 eligible matches", exact: true })).toBeVisible();
    await expect(dialog.getByRole("link", { name: "Systems Engineer Intern - Distributed Data", exact: true })).toBeVisible();
    await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
    await dialog.getByRole("button", { name: "Review workflow" }).click();
    const summary = dialog.getByRole("region", { name: "Matching rules summary" });
    await expect(summary).toContainText("unknown");
    await expect(summary).toContainText("internship");
    await dialog.getByRole("button", { name: "Save workflow", exact: true }).click();
    await expect(dialog.getByText(/Saving has not started a scrape/)).toBeVisible();
    await dialog.getByRole("button", { name: "Done", exact: true }).click();
    await page.getByRole("button", { name: `Edit workflow ${template.name}`, exact: true }).click();
    const reopened = page.getByRole("dialog", { name: "Edit workflow" });
    await reopened.getByRole("button", { name: "Continue to Matching" }).click();
    for (const name of ["Internship", "Unknown employment", "Remote", "Unknown workplace"]) {
      await expect(reopened.getByRole("checkbox", { name, exact: true })).toBeChecked();
    }
  });
}
