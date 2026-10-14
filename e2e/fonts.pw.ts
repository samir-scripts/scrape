import { test, expect } from './fixtures';

test('selected fonts load locally and apply to navigation, body, fields and dialogs', async ({ page }) => {
  await page.goto('/?demo=1');
  await page.getByRole('button', { name: 'SETTINGS', exact: true }).click();
  await page.getByRole('button', { name: /APPEARANCE/ }).click();
  const picker = page.getByLabel('Interface font');
  for (const [id, family] of Object.entries({ inter: 'Inter Variable', jetbrains: 'JetBrains Mono Variable', geist: 'Geist Mono Variable', fira_code: 'Fira Code Variable', ibm_plex: 'IBM Plex Mono' })) {
    await picker.selectOption(id);
    await expect(page.locator('html')).toHaveAttribute('data-font', id);
    expect(await page.evaluate(async family => {
      const faces = await document.fonts.load(`14px "${family}"`, 'Software Engineer 45.25');
      return faces.length > 0 && faces.every(face => face.status === 'loaded');
    }, family)).toBe(true);
    for (const target of [picker, page.getByRole('button', { name: 'SETTINGS', exact: true }), page.getByRole('heading', { name: 'Software Engineer at Acme' })]) {
      expect(await target.evaluate(el => getComputedStyle(el).fontFamily)).toContain(family);
    }
  }
  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-font', 'ibm_plex');
  await page.getByRole('button', { name: 'SCRAPER', exact: true }).click();
  await page.getByRole('button', { name: /ADD WORKFLOW/ }).click();
  expect(await page.getByRole('dialog').evaluate(el => getComputedStyle(el).fontFamily)).toContain('IBM Plex Mono');
  expect(await page.getByLabel('Source URL or board identifier').evaluate(el => getComputedStyle(el).fontFamily)).toContain('IBM Plex Mono');
});
