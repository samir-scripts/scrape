import { test, expect } from "./fixtures";
import { chromium } from "@playwright/test";
import { resolve } from "node:path";

test("both themes remain operable at actual 200% browser zoom", async () => {
  const extension = resolve("e2e/zoom-extension");
  const context = await chromium.launchPersistentContext("", {
    channel: "chromium", headless: true, viewport: null,
    args: [`--disable-extensions-except=${extension}`, `--load-extension=${extension}`, "--window-size=1280,900"],
  });
  try {
    await context.route("**/*", route => new URL(route.request().url()).hostname === "127.0.0.1" ? route.continue() : route.abort());
    const worker = context.serviceWorkers()[0] || await context.waitForEvent("serviceworker");
    const page = await context.newPage();
    const baseURL = String(test.info().project.use.baseURL);
    await page.goto(`${baseURL}/?demo=1&fixtures=1`);
    // Set zoom explicitly after navigation so the test does not depend on the
    // extension's onUpdated event racing with the page load.
    await worker.evaluate(async baseURL => {
      const tabs = await (globalThis as any).chrome.tabs.query({});
      const tab = tabs.find((tab: {url?:string}) => tab.url?.startsWith(baseURL));
      if (!tab) throw new Error("Zoom test tab was not found");
      await (globalThis as any).chrome.tabs.setZoom(tab.id, 2);
    }, baseURL);
    await expect.poll(() => page.evaluate(() => innerWidth)).toBeLessThanOrEqual(640);
    const zoom = await worker.evaluate(async baseURL => {
      const tabs = await (globalThis as any).chrome.tabs.query({});
      const tab = tabs.find((tab: {url?:string}) => tab.url?.startsWith(baseURL));
      return (globalThis as any).chrome.tabs.getZoom(tab.id);
    }, baseURL);
    expect(zoom).toBe(2);
    for (const theme of ["paper", "obsidian"]) {
      await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
      if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
      const add = page.getByRole("button", { name: /add workflow/i });
      await add.click();
      const dialog = page.getByRole("dialog", { name: "Create workflow" });
      await expect(dialog.getByRole("combobox", { name: "Target roles", exact: true })).toBeVisible();
      const role = dialog.getByRole("combobox", { name: "Target roles", exact: true });
      await role.fill("Zoom Custom Job Role");
      await expect(dialog.getByRole("option", { name: "Use “Zoom Custom Job Role”", exact: true })).toBeVisible();
      await role.press("ArrowDown");
      await role.press("Enter");
      const company = dialog.getByRole("combobox", { name: "Companies (Optional)", exact: true });
      await company.fill("Cloudflare");
      await expect(dialog.getByRole("option", { name: /^Cloudflare\b/ })).toBeVisible();
      await company.press("ArrowDown");
      await company.press("Enter");
      await expect(dialog.getByRole("button", { name: "Remove Zoom Custom Job Role from Target roles", exact: true })).toBeVisible();
      expect(await dialog.evaluate(el => el.scrollWidth <= el.clientWidth + 1)).toBe(true);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth + 1)).toBe(true);
      await company.press("ArrowDown");
      await expect(dialog.getByRole("option", { name: /^Cloudflare\b/ })).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(dialog.getByRole("combobox", { name: "Companies (Optional)", exact: true })).toBeFocused();
      await page.keyboard.press("Escape");
      await expect(add).toBeFocused();
    }
  } finally { await context.close(); }
});
