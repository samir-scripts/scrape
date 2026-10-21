import { test, expect } from "./fixtures";

import { contrastFailures, indicatorContrast } from "./contrast";

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
      await expect.soft(page).toHaveScreenshot(`${theme}-${width}-settings.png`, { animations: "disabled" });
      await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
      await page.getByRole("button", { name: "Add workflow", exact: true }).click();
      const dialog = page.getByRole("dialog", { name: "Create workflow" });
      await expect(dialog).toBeVisible();
      await dialog.getByRole("textbox", { name: "Workflow name", exact: true }).focus();
      await page.keyboard.press("Tab");
      await expect(dialog.getByRole("textbox", { name: "Target roles", exact: true })).toBeFocused();
      await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
      expect(await contrastFailures(page)).toEqual([]);
      await expect.soft(page).toHaveScreenshot(`${theme}-${width}-workflow.png`, { animations: "disabled" });
      await page.keyboard.press("Escape");
      await expect(page.getByRole("button", { name: "Add workflow", exact: true })).toBeFocused();
      await page.getByRole("button", { name: "RUN", exact: true }).first().click();
      await expect(page.getByRole("button", { name: "RUNNING…", exact: true })).toHaveCount(0, { timeout: 15000 });
      await page.getByRole("button", { name: /^DISCOVERED/ }).click();
      const remove = page.getByRole("button", { name: /^Remove .* at / }).first();
      await expect(remove).toBeVisible();
      await remove.hover();
      expect(await indicatorContrast(remove, "color")).toBeGreaterThanOrEqual(3);
      expect(await contrastFailures(page)).toEqual([]);
      await expect.soft(page).toHaveScreenshot(`${theme}-${width}-jobs.png`, { animations: "disabled" });
      await remove.focus();
      await page.keyboard.press("Tab"); await page.keyboard.press("Shift+Tab");
      await expect(remove).toBeFocused();
      expect(await remove.evaluate(el => getComputedStyle(el).outlineStyle)).toBe("solid");
      expect(await indicatorContrast(remove, "outlineColor")).toBeGreaterThanOrEqual(3);
      await page.keyboard.press("Enter");
      const removal = page.getByRole("dialog", { name: "Remove saved job" });
      await expect(removal.getByRole("button", { name: "Cancel", exact: true })).toBeFocused();
      expect(await contrastFailures(page)).toEqual([]);
      await expect.soft(page).toHaveScreenshot(`${theme}-${width}-removal.png`, { animations: "disabled" });
      await page.keyboard.press("Escape");
      await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
      // A 200% zoom-equivalent viewport exercises reflow at the same CSS width.
      await page.setViewportSize({ width: Math.floor(width / 2), height: 450 });
      await page.getByRole("button", { name: "Add workflow", exact: true }).click();
      await expect(dialog.getByRole("textbox", { name: "Workflow name", exact: true })).toBeVisible();
      expect(await dialog.evaluate(el => el.scrollWidth <= innerWidth)).toBe(true);
    });
  }
}
