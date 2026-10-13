import { test, expect, type Page } from "@playwright/test";
export async function discoverFixtures(page: Page) {
  await page.goto("/?demo=1");
  await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
  await page.getByRole("button", { name: "RUN", exact: true }).first().click();
  await expect(
    page.getByRole("button", { name: "RUNNING…", exact: true }),
  ).toHaveCount(0, { timeout: 15000 });
  await page.getByRole("button", { name: /^DISCOVERED/ }).click();
  await expect(
    page.getByRole("button", { name: /^Remove .* at / }).first(),
  ).toBeVisible();
}
test("removal, Undo, blocking, unblocking and restoration operate through actual controls", async ({
  page,
}) => {
  await discoverFixtures(page);
  const remove = page.getByRole("button", { name: /^Remove .* at / }).first();
  const accessibleName = await remove.getAttribute("aria-label");
  await remove.focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog");
  await expect(dialog.getByRole("checkbox")).not.toBeChecked();
  await expect(
    dialog.getByRole("button", { name: "Cancel", exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(
    page.getByRole("button", { name: accessibleName!, exact: true }),
  ).toBeFocused();
  await page.keyboard.press("Enter");
  await dialog.getByRole("checkbox").check();
  await dialog.getByRole("button", { name: "Remove job", exact: true }).click();
  await expect(
    page.getByRole("button", { name: accessibleName!, exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Undo", exact: true }).click();
  await expect(
    page.getByRole("button", { name: accessibleName!, exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: accessibleName!, exact: true })
    .click();
  await dialog.getByRole("checkbox").check();
  await dialog.getByRole("button", { name: "Remove job", exact: true }).click();
  await expect(dialog).not.toBeVisible();
  await expect(page.getByRole("button", { name: accessibleName!, exact: true })).toHaveCount(0);
  await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
  await page.getByRole("button", { name: /REMOVED & BLOCKED JOBS/ }).click();
  await page.getByRole("button", { name: "Unblock future discovery" }).click();
  await expect(
    page.getByRole("button", { name: "Block future discovery" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Restore saved job and unblock" })
    .click();
  await expect(page.getByText(/No removed or blocked jobs/)).toBeVisible();
});
test("hourly cents are shown in active filters and persist across reload", async ({
  page,
}) => {
  await discoverFixtures(page);
  await page.getByRole("button", { name: /FILTERS/ }).click();
  await page.getByRole("button", { name: "Hourly", exact: true }).click();
  await page.getByLabel("Minimum per hour").fill("45.25");
  await page.getByLabel("Maximum per hour").fill("55.75");
  await page.getByLabel("Maximum per hour").blur();
  await page.keyboard.press("Escape");
  await expect(page.getByText(/RATE: .*45.25.*55.75\/HR/)).toBeVisible();
  await page.reload();
  await expect(page.getByText(/RATE: .*45.25.*55.75\/HR/)).toBeVisible();
  await page.getByRole("button", { name: /FILTERS/ }).click();
  await page.getByRole("button", { name: "Annual", exact: true }).click();
  await expect(page.getByLabel("Minimum per year")).toHaveValue("");
  await page.getByRole("button", { name: "Clear compensation filter" }).click();
  await expect(page.getByLabel("Maximum per year")).toHaveValue("");
});
