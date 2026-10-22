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
      for (const step of ["Criteria", "Sources", "Schedule"]) {
        await dialog.getByRole("button", { name: `Continue to ${step}`, exact: true }).click();
      }
      await dialog.getByText("All configured rules", { exact: true }).click();
      const body = dialog.locator("[data-dialog-scroll]");
      const review = dialog.locator("details pre");
      const expectOuterLayersStill = async () => {
        expect(await body.evaluate(el => {
          const panel = el.parentElement!;
          const outer = el.closest("dialog")!;
          return [panel, outer].every(layer =>
            layer.scrollTop === 0 && layer.scrollHeight <= layer.clientHeight + 1);
        })).toBe(true);
      };
      await expectOuterLayersStill();
      await body.hover();
      await page.mouse.wheel(0, 10000);
      await expect.poll(() => body.evaluate(el =>
        el.scrollHeight - el.clientHeight - el.scrollTop)).toBeLessThanOrEqual(1);
      // Check the final YAML character against the body's clipping rectangle,
      // rather than accepting Playwright's visibility check for offscreen text.
      expect(await review.evaluate(el => {
        const text = el.firstChild;
        if (!text?.textContent?.length) return false;
        const range = document.createRange();
        range.setStart(text, text.textContent.length - 1);
        range.setEnd(text, text.textContent.length);
        const last = range.getBoundingClientRect();
        const viewport = el.closest("[data-dialog-scroll]")!.getBoundingClientRect();
        return last.top >= viewport.top && last.bottom <= viewport.bottom + 1;
      })).toBe(true);
      for (const name of ["Cancel", "Save workflow"]) {
        const action = dialog.getByRole("button", { name, exact: true });
        await expect(action).toBeVisible();
        expect(await action.evaluate(el => {
          const rect = el.getBoundingClientRect();
          return rect.top >= 0 && rect.bottom <= innerHeight &&
            rect.left >= 0 && rect.right <= innerWidth;
        })).toBe(true);
      }
      await expectOuterLayersStill();
      await page.mouse.wheel(0, -10000);
      await expect.poll(() => body.evaluate(el => el.scrollTop)).toBe(0);
      await expectOuterLayersStill();
      await page.keyboard.press("Escape");
      await expect(add).toBeFocused();
    }
  } finally { await context.close(); }
});
