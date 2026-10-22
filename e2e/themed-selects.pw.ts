import { test, expect } from "./fixtures";

for (const theme of ["paper", "obsidian"] as const) {
  for (const width of [480, 1280] as const) {
    test(`${theme} ${width}px opens themed date and searchable geography options`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width, height: 900 });
      await page.goto("/?demo=1&fixtures=1");
      await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
      if (theme === "obsidian") await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
      await page.getByRole("button", { name: "SYSTEM", exact: true }).click();
      const date = page.getByRole("button", { name: "Date format" });
      await date.click();
      await expect(page.getByRole("listbox")).toBeVisible();
      await expect(page.getByRole("option")).toHaveCount(5);
      await page.screenshot({ path: testInfo.outputPath(`date-format-${theme}-${width}.png`), fullPage: true });
      if (theme === "paper" && width === 480) {
        await page.keyboard.press("D");
        await page.keyboard.press("Enter");
        await expect(date).toContainText("DD/MM/YY");
      } else {
        await page.getByRole("option", { name: /DD\/MM\/YYYY/ }).click();
        await expect(date).toContainText("DD/MM/YYYY");
      }
      if (width === 480) {
        await page.setViewportSize({ width: 240, height: 700 });
        await date.click();
        const bounds = await page.getByRole("listbox").boundingBox();
        expect(bounds).not.toBeNull();
        expect(bounds!.x).toBeGreaterThanOrEqual(0);
        expect(bounds!.x + bounds!.width).toBeLessThanOrEqual(240);
        expect(bounds!.y).toBeGreaterThanOrEqual(0);
        await page.keyboard.press("Escape");
      }

      const time = page.getByRole("button", { name: "Time format" });
      await time.click();
      await expect(page.getByRole("listbox")).toBeVisible();
      await expect(page.getByRole("option")).toHaveCount(2);
      await page.screenshot({ path: testInfo.outputPath(`time-format-${theme}-${width}.png`), fullPage: true });
      await page.getByRole("option", { name: /24-hour/ }).click();
      await expect(time).toContainText("24-hour · 15:25");

      await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
      await page.getByRole("button", { name: /add workflow/i }).click();
      await page.getByRole("button", { name: "Continue to Criteria" }).click();
      const country = page.getByRole("combobox", { name: "Country", exact: true });
      await country.fill("can");
      await expect(page.getByRole("listbox")).toBeVisible();
      await expect(page.getByRole("option", { name: /Canada \(CA\)/ })).toBeVisible();
      await page.screenshot({ path: testInfo.outputPath(`country-options-${theme}-${width}.png`), fullPage: true });
      await page.getByRole("option", { name: /Canada \(CA\)/ }).click();
      await expect(country).toHaveValue("Canada");
      await country.fill("united");
      await page.keyboard.press("Escape");
      await expect(page.getByRole("dialog", { name: "Create workflow" })).toBeVisible();
      await expect(page.getByRole("combobox", { name: "Country", exact: true })).toBeFocused();
      await expect(country).toHaveValue("Canada");
    });
  }
}
