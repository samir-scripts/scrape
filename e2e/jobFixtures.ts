import { expect } from "./fixtures";
import type { Page } from "@playwright/test";
export async function discoverFixtures(page: Page) {
  await page.goto("/?demo=1&fixtures=1");
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
  await page.getByRole("button", { name: "RUN", exact: true }).first().click();
  await expect(
    page.getByRole("button", { name: "RUNNING…", exact: true }),
  ).toHaveCount(0, { timeout: 15000 });
  await page.getByRole("button", { name: /^DISCOVERED/ }).click();
  await expect(
    page.getByRole("button", { name: /^Remove .* at / }).first(),
  ).toBeVisible();
}
