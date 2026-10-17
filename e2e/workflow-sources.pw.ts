import { test, expect } from "./fixtures";
import { contrastFailures } from "./contrast";

for (const theme of ["paper", "obsidian"]) {
  test(`${theme} custom extractor mode switching, locators, browser and script panels`, async ({ page }) => {
    await page.setViewportSize({ width: 1024, height: 900 });
    await page.goto("/?demo=1&fixtures=1");

    // Toggle theme if obsidian
    if (theme === "obsidian") {
      await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
      await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", "obsidian");
    }

    // Open Scraper tab and create workflow modal
    await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
    await page.getByRole("button", { name: "Add workflow", exact: true }).click();

    const dialog = page.getByRole("dialog", { name: "Create workflow" });
    await expect(dialog).toBeVisible();

    // Expand Advanced settings
    await dialog.getByRole("button", { name: "Advanced settings" }).click();

    // Switch adapter to selector
    const adapterSelect = dialog.getByRole("button", { name: "Source 1 adapter" });
    await adapterSelect.click();
    await page.getByRole("option", { name: "selector", exact: true }).click();

    // 1. Static HTML Mode
    const modeSelect = dialog.getByRole("button", { name: "Source 1 Extraction mode" });
    await expect(modeSelect).toContainText("Static HTML");

    const containerInput = dialog.getByRole("textbox", { name: "Source 1 Job container locator" });
    await containerInput.fill("xpath://article[@data-job-id]");
    const titleInput = dialog.getByRole("textbox", { name: "Source 1 Title locator" });
    await titleInput.fill("xpath:.//h3[contains(@class, 'title')]");
    const linkInput = dialog.getByRole("textbox", { name: "Source 1 Destination link locator" });
    await linkInput.fill("a.apply-link");

    // Contrast check on static mode
    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `plans/assets/11-browser-script-review/${theme}-static-1024.png` });

    // 2. Headless Browser Mode
    await modeSelect.click();
    await page.getByRole("option", { name: /Headless Browser/i }).click();

    await expect(dialog.getByText("Browser Runtime & Session")).toBeVisible();
    await expect(dialog.getByRole("spinbutton", { name: "Source 1 Navigation timeout" })).toBeVisible();
    await expect(dialog.getByRole("spinbutton", { name: "Source 1 Max pages" })).toBeVisible();
    await expect(dialog.getByRole("button", { name: "Source 1 Browser session" })).toBeVisible();

    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `plans/assets/11-browser-script-review/${theme}-browser-1024.png` });

    // 3. Custom Script Mode
    await modeSelect.click();
    await page.getByRole("option", { name: /Custom Script/i }).click();

    await expect(dialog.getByText("Custom Script Configuration")).toBeVisible();
    await expect(
      dialog.getByText(/Custom scripts execute with your local user account privileges/i)
    ).toBeVisible();

    const scriptPathInput = dialog.getByRole("textbox", { name: "Source 1 Script file path" });
    await expect(scriptPathInput).toBeVisible();
    await scriptPathInput.fill("./scripts/custom_scraper.py");

    await expect(
      dialog.getByText(/Save the workflow to fingerprint your changes, then trust it/i)
    ).toBeVisible();

    const checkEnvBtn = dialog.getByRole("button", { name: "Check environment" });
    await expect(checkEnvBtn).toBeVisible();
    await checkEnvBtn.click();
    await expect(dialog.getByText(/v20\.0\.0/)).toBeVisible();

    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: `plans/assets/11-browser-script-review/${theme}-script-1024.png` });

    // Responsive 480px width check
    await page.setViewportSize({ width: 480, height: 900 });
    const overflow = await dialog.evaluate(el => el.scrollWidth > el.clientWidth + 1);
    expect(overflow).toBe(false);

    await page.screenshot({ path: `plans/assets/11-browser-script-review/${theme}-script-480.png` });
  });
}
