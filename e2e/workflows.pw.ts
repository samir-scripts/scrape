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
  await dialog
    .getByRole("textbox", { name: "Source URL or board identifier" })
    .fill("https://boards.greenhouse.io/fixture-board");
  await dialog.getByRole("button", { name: "Continue to Matching" }).click();
  await dialog
    .getByRole("textbox", { name: "Title keywords (any)" })
    .fill("NoSuchRole");
  await dialog.getByRole("button", { name: "Preview matches" }).click();
  await expect(dialog.getByText(/No eligible postings/)).toBeVisible();
  await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
  await expect(
    dialog.getByRole("textbox", { name: "IANA timezone" }),
  ).toHaveValue("America/New_York");
  await dialog.getByRole("button", { name: "Review workflow" }).click();
  await dialog.getByRole("button", { name: "Save workflow" }).click();
  await expect(
    dialog.getByText(/Saving has not started a scrape/),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "Done" }).click();
  await expect(
    page.getByText("fixture-board", { exact: true }).first(),
  ).toBeVisible();
  await page.getByRole("row").filter({ hasText: "fixture-board" }).getByRole("button", { name: "EDIT", exact: true }).click();
  const edit = page.getByRole("dialog", { name: "Edit workflow" });
  await expect(edit.getByLabel("Workflow name")).toHaveValue("fixture-board");
  await edit.getByRole("button", { name: "Continue to Matching" }).click();
  await expect(edit.getByLabel("Title keywords (any)")).toHaveValue("NoSuchRole");
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
  const dismissal = new Promise<void>(resolve => page.once("dialog", async prompt => { await prompt.dismiss(); resolve(); }));
  const cancelled = page.waitForEvent("filechooser"); await dialog.getByRole("button", { name: "Import workflow YAML" }).click(); await (await cancelled).setFiles(replacement);
  await dismissal;
  await expect(dialog.getByLabel("Workflow name")).toHaveValue("Unsaved revision");
  page.once("dialog", prompt => prompt.accept());
  const replaced = page.waitForEvent("filechooser"); await dialog.getByRole("button", { name: "Import workflow YAML" }).click(); await (await replaced).setFiles(replacement);
  await expect(dialog.getByLabel("Workflow name")).toHaveValue("Replacement fixture");
  await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
  await expect(page.getByText("Uploaded fixture", { exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "Scraper settings", exact: true }).click();
  await expect(page.getByRole("dialog", { name: "Scraper settings" })).toBeVisible();
});
