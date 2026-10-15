import { test, expect } from "./fixtures";
import { contrastFailures } from "./contrast";
for (const theme of ["paper", "obsidian"]) for (const width of [480, 1280]) {
  test(`${theme} ${width}: data management explains desktop scope without overflow`, async ({ page }) => {
    await page.setViewportSize({ width, height: 800 });
    await page.goto('/?demo=1&fixtures=1');
    await page.getByRole('button', { name: 'SETTINGS', exact: true }).click();
    if (theme === 'obsidian') await page.getByRole('button', { name: /^OBSIDIAN/ }).click();
    const navigation = page.getByRole('button', { name: 'DATA MANAGEMENT', exact: true });
    await navigation.focus(); await page.keyboard.press('Enter');
    await expect(page.getByRole('heading', { name: 'Data management', exact: true })).toBeVisible();
    await expect(page.getByRole('status').filter({ hasText: 'Available in the desktop app only' })).toContainText('Browser previews cannot clear the desktop database');
    const reviews = page.getByRole('button', { name: /^Review / });
    await expect(reviews).toHaveCount(4);
    for (const review of await reviews.all()) await expect(review).toBeDisabled();
    expect(await contrastFailures(page)).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    // An artifact for review, not a newly blessed baseline of the dirty worktree.
    await page.screenshot({ path: test.info().outputPath(`${theme}-${width}-cleanup.png`), fullPage: true });
  });
}
