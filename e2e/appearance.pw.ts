import { test, expect } from "./fixtures";
import type { Locator } from "@playwright/test";

import { contrastFailures } from "./contrast";

async function indicatorContrast(control: Locator, property: "color" | "outlineColor") {
  return control.evaluate((element, property) => {
    const canvas = document.createElement("canvas"); canvas.width = canvas.height = 1;
    const context = canvas.getContext("2d", { willReadFrequently: true })!;
    const luminance = (value: string) => {
      context.clearRect(0, 0, 1, 1); context.fillStyle = value; context.fillRect(0, 0, 1, 1);
      const rgb = [...context.getImageData(0, 0, 1, 1).data].slice(0, 3).map(c => c / 255).map(c => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4);
      return rgb.reduce((sum, c, i) => sum + c * [.2126, .7152, .0722][i], 0);
    };
    const style = getComputedStyle(element);
    const surface = getComputedStyle(document.documentElement).getPropertyValue("--color-surface").trim();
    // The remove icon and focus ring are essential indicators against the card surface.
    const a = luminance(style[property]); const b = luminance(surface);
    return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
  }, property);
}
for (const theme of ["paper", "obsidian"]) {
  for (const width of [1280, 480]) {
    test(`${theme} ${width}px rendered text, keyboard focus and reduced-motion layout`, async ({ page }) => {
      await page.setViewportSize({ width, height: 900 });
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.clock.setFixedTime(new Date("2026-10-13T13:00:00Z"));
      await page.goto("/?demo=1&fixtures=1");
      await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
      if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
      await expect(page.getByRole("button", { name: new RegExp(`^${theme.toUpperCase()}`) })).toContainText("ACTIVE");
      await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
      expect(await contrastFailures(page)).toEqual([]);
      await expect(page).toHaveScreenshot(`${theme}-${width}-settings.png`, { animations: "disabled" });
      await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
      await page.getByRole("button", { name: /ADD WORKFLOW/ }).click();
      const dialog = page.getByRole("dialog", { name: "Create workflow" });
      await expect(dialog).toBeVisible();
      await dialog.getByRole("textbox", { name: "Source URL or board identifier" }).focus();
      await page.keyboard.press("Tab");
      await expect(dialog.getByRole("textbox", { name: "Workflow name" })).toBeFocused();
      await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
      expect(await contrastFailures(page)).toEqual([]);
      await expect(page).toHaveScreenshot(`${theme}-${width}-workflow.png`, { animations: "disabled" });
      await page.keyboard.press("Escape");
      await expect(page.getByRole("button", { name: /ADD WORKFLOW/ })).toBeFocused();
      await page.getByRole("button", { name: "RUN", exact: true }).first().click();
      await expect(page.getByRole("button", { name: "RUNNING…", exact: true })).toHaveCount(0, { timeout: 15000 });
      await page.getByRole("button", { name: /^DISCOVERED/ }).click();
      const remove = page.getByRole("button", { name: /^Remove .* at / }).first();
      await expect(remove).toBeVisible();
      await remove.hover();
      expect(await indicatorContrast(remove, "color")).toBeGreaterThanOrEqual(3);
      expect(await contrastFailures(page)).toEqual([]);
      await expect(page).toHaveScreenshot(`${theme}-${width}-jobs.png`, { animations: "disabled" });
      await remove.focus();
      await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
      await expect(remove).toBeFocused();
      expect(await remove.evaluate(el => getComputedStyle(el).outlineStyle)).toBe("solid");
      expect(await indicatorContrast(remove, "outlineColor")).toBeGreaterThanOrEqual(3);
      await page.keyboard.press("Enter");
      const removal = page.getByRole("dialog", { name: "Remove saved job" });
      await expect(removal.getByRole("button", { name: "Cancel", exact: true })).toBeFocused();
      expect(await contrastFailures(page)).toEqual([]);
      await expect(page).toHaveScreenshot(`${theme}-${width}-removal.png`, { animations: "disabled" });
      await page.keyboard.press("Escape");
      await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
      // A 200% zoom-equivalent viewport exercises reflow at the same CSS width.
      await page.setViewportSize({ width: Math.floor(width / 2), height: 450 });
      await page.getByRole("button", { name: /ADD WORKFLOW/ }).click();
      await expect(dialog.getByRole("textbox", { name: "Source URL or board identifier" })).toBeVisible();
      expect(await dialog.evaluate(el => el.scrollWidth <= innerWidth)).toBe(true);
    });
  }
}
