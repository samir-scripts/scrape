import { test, expect } from "@playwright/test";
test("create a workflow through actual controls without an implicit scrape", async ({
  page,
}) => {
  await page.goto("/?demo=1");
  await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
  await page
    .getByRole("button", { name: /add workflow|new workflow|create workflow/i })
    .click();
  const dialog = page.getByRole("dialog", { name: "Create workflow" });
  await dialog
    .getByRole("textbox", { name: "Source URL or board identifier" })
    .fill("https://boards.greenhouse.io/fixture-board");
  await dialog.getByRole("button", { name: "Detect source" }).click();
  await dialog
    .getByRole("textbox", { name: "Title keywords (any)" })
    .fill("NoSuchRole");
  await dialog.getByRole("button", { name: "Preview matches" }).click();
  await expect(dialog.getByText(/No eligible postings/)).toBeVisible();
  await dialog.getByRole("button", { name: "Continue to schedule" }).click();
  await expect(
    dialog.getByRole("textbox", { name: "IANA timezone" }),
  ).toHaveValue("America/New_York");
  await dialog.getByRole("button", { name: "Save workflow" }).click();
  await expect(
    dialog.getByText(/Saving has not started a scrape/),
  ).toBeVisible();
  await dialog.getByRole("button", { name: "Done" }).click();
  await expect(
    page.getByText("fixture-board", { exact: true }).first(),
  ).toBeVisible();
});
