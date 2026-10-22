import { test, expect } from "./fixtures";
test("create a workflow through actual controls without an implicit scrape", async ({
  page,
}) => {
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
  await page
    .getByRole("button", { name: /add workflow|new workflow|create workflow/i })
    .click();
  const dialog = page.getByRole("dialog", { name: "Create workflow" });
  await dialog.getByRole("textbox", { name: "Workflow name", exact: true }).fill("fixture-board");
  await dialog.getByRole("combobox", { name: "Any keywords", exact: true }).fill("NoSuchRole");
  await expect(dialog.getByRole("button", { name: "Preview matches" })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Continue to Criteria" }).click();
  // Date matching is exercised through Rust/native fixtures, not this UI demo.
  await dialog.getByRole("button", { name: "Date posted range", exact: true }).click();
  await page.getByRole("option", { name: "Any time", exact: true }).click();
  const limit = dialog.getByRole("textbox", { name: "Maximum matching jobs per run" });
  await expect(limit).toHaveValue("20");
  await limit.fill("7");
  await dialog.getByRole("combobox", { name: "Country", exact: true }).fill("US");
  await page.keyboard.press("Tab");
  await dialog.getByRole("combobox", { name: "State/province", exact: true }).fill("CA");
  await page.keyboard.press("Enter");
  await dialog.getByRole("checkbox", { name: "Remote", exact: true }).check();
  await dialog.getByRole("button", { name: "Continue to Sources" }).click();
  await expect(dialog.getByRole("textbox", { name: /URL or board identifier/ })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Add source", exact: true }).click();
  await dialog.getByRole("textbox", { name: "Source 2 URL or board identifier", exact: true }).fill("https://boards.greenhouse.io/fixture-board");
  await dialog.getByRole("button", { name: "Remove source Cross-Company Search", exact: true }).click();
  await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
  await expect(
    dialog.getByRole("textbox", { name: "IANA timezone" }),
  ).toHaveValue("America/New_York");
  await expect(dialog.getByRole("button", { name: "Save workflow" })).toBeDisabled();
  await dialog.getByRole("button", { name: "Preview matches" }).click();
  await expect(dialog.getByText(/No eligible postings/)).toBeVisible();
  await dialog.getByRole("button", { name: "Save workflow" }).click();
  await expect(
    dialog.getByText(/Saving has not started a scrape/),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "Done" }).click();
  await expect(
    page.getByText("fixture-board", { exact: true }).first(),
  ).toBeVisible();
  await page.getByRole("row").filter({ hasText: "fixture-board" }).getByRole("button", { name: "Edit workflow fixture-board" }).click();
  const edit = page.getByRole("dialog", { name: "Edit workflow" });
  await expect(edit.getByLabel("Workflow name")).toHaveValue("fixture-board");
  await expect(edit.getByRole("button", { name: "Remove NoSuchRole from Any keywords", exact: true })).toBeVisible();
  await edit.getByRole("button", { name: "Continue to Criteria" }).click();
  await expect(edit.getByRole("textbox", { name: "Maximum matching jobs per run" })).toHaveValue("7");
  await expect(edit.getByRole("combobox", { name: "Country", exact: true })).toHaveValue("United States");
  await expect(edit.getByRole("button", { name: "Remove California", exact: true })).toBeVisible();
  await expect(edit.getByRole("checkbox", { name: "Remote", exact: true })).toBeChecked();
  await edit.getByRole("button", { name: "Back", exact: true }).click();
  await expect(edit.getByLabel("Workflow name")).toHaveValue("fixture-board");
  await edit.getByRole("button", { name: "Cancel", exact: true }).click();
});

test("registry file chooser imports an unsaved draft and invalid replacement keeps edits", async ({ page }) => {
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
  const choose = page.waitForEvent("filechooser");
  await page.getByRole("button", { name: "Import workflow YAML", exact: true }).click();
  await (await choose).setFiles({ name: "authored-workflow.yaml", mimeType: "text/yaml", buffer: Buffer.from(`version: 3
metadata: {name: Uploaded fixture, enabled: true}
extractor: {kind: greenhouse, url: https://boards.greenhouse.io/upload-fixture}
schedule: {cadence: manual, timezone: UTC}
filters: {include_keywords: [], exclude_keywords: [], locations: []}
`) });
  const dialog = page.getByRole("dialog", { name: "Create workflow" });
  await expect(dialog.getByLabel("Workflow name")).toHaveValue("Uploaded fixture");
  await dialog.getByLabel("Workflow name").fill("Unsaved revision");
  const bad = page.waitForEvent("filechooser"); await dialog.getByRole("button", { name: "Import workflow YAML" }).click();
  await (await bad).setFiles({ name: "invalid.yaml", mimeType: "text/yaml", buffer: Buffer.from("version: 99\nmetadata: {name: Invalid}\nextractor: {kind: greenhouse, url: fixture}\n") });
  await expect(dialog.getByRole("alert")).toContainText("Unsupported workflow version");
  await expect(dialog.getByLabel("Workflow name")).toHaveValue("Unsaved revision");
  const replacement = { name: "replacement.yaml", mimeType: "text/yaml", buffer: Buffer.from("version: 3\nmetadata: {name: Replacement fixture}\nextractor: {kind: greenhouse, url: https://boards.greenhouse.io/replacement}\nschedule: {cadence: manual, timezone: UTC}\n") };
  const cancelled = page.waitForEvent("filechooser"); await dialog.getByRole("button", { name: "Import workflow YAML" }).click(); await (await cancelled).setFiles(replacement);
  await dialog.getByRole("button", { name: "Keep editing" }).click();
  await expect(dialog.getByRole("button", { name: "Import workflow YAML" })).toBeFocused();
  await expect(dialog.getByLabel("Workflow name")).toHaveValue("Unsaved revision");
  const replaced = page.waitForEvent("filechooser"); await dialog.getByRole("button", { name: "Import workflow YAML" }).click(); await (await replaced).setFiles(replacement);
  await dialog.getByRole("button", { name: "Replace draft" }).click();
  await expect(dialog.getByLabel("Workflow name")).toHaveValue("Replacement fixture");
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByText("Uploaded fixture", { exact: true })).toHaveCount(0);
  await expect(page.getByText("fixture-board", { exact: true })).toHaveCount(0);
});

test("workflow registry consolidates actions while keeping execution and import visible", async ({ page }) => {
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
  const row = page.getByRole("row").filter({ hasText: "Cloudflare — Software Engineering Internships" });
  await expect(page.getByRole("button", { name: "Add workflow" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Run all" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Import workflow YAML" })).toBeVisible();
  await expect(row.getByRole("button", { name: "RUN", exact: true })).toBeVisible();
  await row.getByRole("button", { name: /More actions for/ }).click();
  const menu = page.getByRole("menu");
  await expect(menu.getByRole("menuitem")).toHaveText([
    "Edit workflow", "Preview matches", "View run history", "Delete workflow",
  ]);
  await menu.getByRole("menuitem", { name: "View run history" }).click();
  await expect(page.getByText(/WORKFLOW OBSERVABILITY & EXECUTION LOGS/)).toBeVisible();
  await row.getByRole("button", { name: /More actions for/ }).click();
  await page.getByRole("menuitem", { name: "Preview matches" }).click();
  await expect(page.getByText("DRY-RUN EXTRACTION PREVIEW (FIRST 10 JOBS)")).toBeVisible();
});
