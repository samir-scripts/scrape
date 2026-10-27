import { test, expect } from "./fixtures";
test("create a workflow through actual controls without an implicit scrape", async ({
  page,
}) => {
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
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
  const limit = dialog.getByRole("textbox", { name: "Target new Discovered jobs per run" });
  await expect(limit).toHaveValue("20");
  await limit.fill("7");
  const country = dialog.getByRole("combobox", { name: "Country", exact: true });
  await country.fill("United States");
  await country.press("ArrowDown");
  await country.press("Enter");
  const state = dialog.getByRole("combobox", { name: "State/province", exact: true });
  await state.fill("New Jersey");
  await state.press("ArrowDown");
  await state.press("Enter");
  const cities = dialog.getByRole("combobox", { name: "Cities / Location terms", exact: true });
  await cities.fill("New");
  await cities.press("ArrowDown");
  await cities.press("Enter");
  await expect(dialog.getByRole("button", { name: "Remove New York from Cities / Location terms", exact: true })).toBeVisible();
  await dialog.getByRole("checkbox", { name: "Remote", exact: true }).check();
  await dialog.getByRole("button", { name: "Continue to Sources" }).click();
  await expect(dialog.getByRole("textbox", { name: /URL or board identifier/ })).toHaveCount(0);
  await dialog.getByRole("button", { name: "Add source", exact: true }).click();
  await dialog.getByRole("textbox", { name: "Source 3 URL or board identifier", exact: true }).fill("https://boards.greenhouse.io/fixture-board");
  await dialog.getByRole("button", { name: "Remove source Cross-Company Search", exact: true }).click();
  await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
  const minute = dialog.getByRole("textbox", { name: "Minute input" });
  await minute.click();
  await page.keyboard.press("ArrowUp");
  await expect(minute).toHaveValue("01");
  await dialog.getByRole("button", { name: "Continue to Review" }).click();
  await expect(dialog.getByText(/daily 09:01.*America\/New_York/)).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Save workflow" })).toBeDisabled();
  await dialog.getByRole("button", { name: "Preview matches" }).click();
  await expect(dialog.getByRole("button", { name: "Save workflow" })).toBeEnabled();
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
  await expect(edit.getByRole("textbox", { name: "Target new Discovered jobs per run" })).toHaveValue("7");
  await expect(edit.getByRole("combobox", { name: "Country", exact: true })).toHaveValue("United States");
  await expect(edit.getByRole("button", { name: "Remove New Jersey", exact: true })).toBeVisible();
  await expect(edit.getByRole("button", { name: "Remove New York from Cities / Location terms", exact: true })).toBeVisible();
  await expect(edit.getByRole("checkbox", { name: "Remote", exact: true })).toBeChecked();
  await edit.getByRole("button", { name: "Back", exact: true }).click();
  await expect(edit.getByLabel("Workflow name")).toHaveValue("fixture-board");
  await edit.getByRole("button", { name: "Cancel", exact: true }).click();
});

test("registry file chooser imports an unsaved draft and invalid replacement keeps edits", async ({ page }) => {
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
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
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
  const row = page.getByRole("row").filter({ hasText: "Software Engineering & Related — Past 12 Hours" });
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

test("workflow deletion is confirmed and remains deleted after reload", async ({ page }) => {
  const interactions: string[] = [];
  page.on("console", message => { if (message.text().startsWith("UI ")) interactions.push(message.text()); });
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
  const name = "Software Engineering & Related — Past 12 Hours";
  const row = page.getByRole("row").filter({ hasText: name });
  const remove = async () => {
    await row.getByRole("button", { name: /More actions for/ }).click();
    await page.getByRole("menuitem", { name: "Delete workflow" }).click();
  };
  await remove();
  const confirmation = page.getByRole("dialog", { name: "Delete workflow" });
  await expect(confirmation).toBeVisible();
  await confirmation.getByRole("button", { name: "Cancel" }).click();
  await expect(row).toBeVisible();
  await remove();
  await confirmation.getByRole("button", { name: "Delete workflow" }).click();
  await expect(row).toHaveCount(0);
  await page.reload();
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
  await expect(row).toHaveCount(0);
  await expect(page.getByRole("row").filter({ hasText: "Fast Food — Cherry Hill, NJ — Past 30 Days" })).toBeVisible();
  expect(interactions.some(message => message.includes("workflow.delete.request"))).toBe(true);
  expect(interactions.some(message => message.includes("workflow.delete.cancel"))).toBe(true);
  expect(interactions.some(message => message.includes("workflow.delete.confirm"))).toBe(true);
  expect(interactions.every(message => !message.includes(name))).toBe(true);
});
