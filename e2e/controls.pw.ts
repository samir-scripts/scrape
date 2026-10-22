import { test, expect } from './fixtures';
import { contrastFailures, indicatorContrast } from './contrast';

for (const theme of ['paper', 'obsidian']) {
  test(`${theme} themed native controls and consistent icon actions`, async ({ page }, testInfo) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.goto('/?demo=1&fixtures=1');
    await page.getByRole('button', { name: 'SETTINGS', exact: true }).click();
    if (theme === 'obsidian') await page.getByRole('button', { name: /^OBSIDIAN/ }).click();
    expect(await page.locator('html').evaluate(el => getComputedStyle(el).colorScheme)).toBe(theme === 'paper' ? 'light' : 'dark');
    await page.getByRole('button', { name: 'WORKFLOWS', exact: true }).click();
    await page.getByRole('button', { name: 'RUN', exact: true }).first().click();
    await expect(page.getByRole('button', { name: 'RUNNING…', exact: true })).toHaveCount(0, { timeout: 15000 });
    await page.getByRole('button', { name: /^DISCOVERED/ }).click();
    for (const name of [/^Open .* in browser$/, /^Remove .* at /, /^Favorite posting$/]) {
      const action = page.getByRole('button', { name }).first();
      await expect(action).toBeVisible();
      const bounds = await action.boundingBox();
      expect(bounds!.width).toBeGreaterThanOrEqual(32);
      expect(bounds!.height).toBeGreaterThanOrEqual(32);
      await expect(action.locator('svg')).toHaveAttribute('aria-hidden', 'true');
      await action.hover();
      expect(await indicatorContrast(action, "color")).toBeGreaterThanOrEqual(3);
      await action.focus();
      await page.keyboard.press('Tab'); await page.keyboard.press('Shift+Tab');
      await expect(action).toBeFocused();
      expect(await action.evaluate(el => getComputedStyle(el).outlineStyle)).toBe('solid');
      expect(await indicatorContrast(action, 'outlineColor')).toBeGreaterThanOrEqual(3);
    }
    const favorite = page.getByRole('button', { name: 'Favorite posting', exact: true }).first();
    await favorite.focus(); await page.keyboard.press('Enter');
    await expect(page.getByRole('button', { name: 'Unfavorite posting', exact: true }).first()).toHaveAttribute('aria-pressed', 'true');
    expect(await contrastFailures(page)).toEqual([]);
    const scroll = page.locator('.overflow-y-auto').first();
    expect(await scroll.evaluate(el => getComputedStyle(el).scrollbarColor)).not.toBe('auto');
    await page.screenshot({ path: testInfo.outputPath(`${theme}-controls.png`) });
  });
}
