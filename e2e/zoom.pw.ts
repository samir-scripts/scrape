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
    const page = await context.newPage();
    const baseURL = String(test.info().project.use.baseURL);
    await page.goto(`${baseURL}/?demo=1&fixtures=1`);
    await expect.poll(() => page.evaluate(() => innerWidth)).toBeLessThanOrEqual(640);
    const worker = context.serviceWorkers()[0] || await context.waitForEvent("serviceworker");
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
      const add = page.getByRole("button", { name: /ADD WORKFLOW/ });
      await add.click();
      const dialog = page.getByRole("dialog", { name: "Create workflow" });
      await expect(dialog.getByRole("textbox", { name: "Source URL or board identifier" })).toBeVisible();
      expect(await dialog.evaluate(el => el.getBoundingClientRect().right <= innerWidth && el.getBoundingClientRect().left >= 0)).toBe(true);
      await page.keyboard.press("Escape");
      await expect(add).toBeFocused();
    }
  } finally { await context.close(); }
});
