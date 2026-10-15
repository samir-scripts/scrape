import { test, expect } from './fixtures';
import type { Page } from '@playwright/test';
import { discoverFixtures } from './jobFixtures';
import { contrastFailures } from './contrast';

async function seed(page: Page, count: number) {
  await discoverFixtures(page);
  await page.evaluate(count => {
    const sample = JSON.parse(localStorage.getItem('scrape:demo:v1:jobs')!)[0];
    localStorage.setItem('scrape:demo:v1:jobs', JSON.stringify(Array.from({ length: count }, (_, i) => ({
      ...sample, id: `layout-${String(i).padStart(3, '0')}`, dedupe_key: `layout:${i}`, title: `Layout fixture ${String(i).padStart(3, '0')}`,
      status: 'viewed', notes: '', description: null,
      description_text: ('Actual authored layout regression content. ' + 'LongURL'.repeat(25) + '\n\n').repeat(30),
      date_added: '2026-10-14T12:00:00Z', url: `https://fixture.example.test/jobs/${i}`,
    }))));
    localStorage.removeItem('scrape:job_filters:v2');
  }, count);
  await page.reload();
  await expect(page.getByRole('button', { name: /^View Layout fixture 000 / })).toBeVisible();
}
test('one toolbar, bottom page jumps, pending notes, and filter resets', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 }); await seed(page, 105);
  const toolbar = page.getByRole('toolbar', { name: 'Job controls' });
  await expect(toolbar.getByLabel('Sort jobs')).toBeVisible();
  await expect(toolbar.getByRole('textbox', { name: 'Search jobs' })).toBeVisible();
  const paginator = page.getByRole('navigation', { name: 'Job pagination' });
  const list = page.getByRole('region', { name: 'Job results' });
  expect((await paginator.boundingBox())!.y).toBeGreaterThan((await list.boundingBox())!.y);
  await expect(paginator.getByRole('status')).toHaveText('1–50 of 105');
  const title = page.getByRole('button', { name: /^View Layout fixture 000 / });
  await title.click(); await page.getByLabel('Notes', { exact: true }).fill('Draft before page jump');
  const input = paginator.getByRole('textbox', { name: 'Page number' });
  await input.fill('3'); await input.press('Enter');
  await expect(paginator.getByRole('status')).toHaveText('101–105 of 105');
  await expect(input).toBeFocused();
  await expect(page.getByRole('complementary', { name: 'Job details' })).toHaveCount(0);
  await expect.poll(() => page.evaluate(() => JSON.parse(localStorage.getItem('scrape:demo:v1:jobs')!).find((job: {id:string}) => job.id === 'layout-000').notes)).toBe('Draft before page jump');
  await input.fill('4'); await input.press('Enter');
  await expect(paginator.getByRole('alert')).toContainText('1 to 3'); await expect(input).toBeFocused();
  await input.press('Escape'); await expect(input).toHaveValue('3');
  await paginator.getByRole('button', { name: 'First page' }).click(); await expect(input).toHaveValue('1');
  await title.click(); await expect(page.getByLabel('Notes', { exact: true })).toHaveValue('Draft before page jump');
  await toolbar.getByLabel('Sort jobs').selectOption('title_asc');
  await expect(page.getByRole('complementary', { name: 'Job details' })).toHaveCount(0);
  await paginator.getByRole('button', { name: 'Last page' }).click();
  await toolbar.getByRole('textbox', { name: 'Search jobs' }).fill('Layout fixture 000');
  await expect(input).toHaveValue('1'); await expect(paginator.getByRole('status')).toHaveText('1–1 of 1');
  await toolbar.getByRole('button', { name: 'Clear search' }).click();
  await expect(input).toHaveValue('1'); await expect(paginator.getByRole('status')).toHaveText('1–50 of 105');
});
for (const theme of ['paper', 'obsidian']) {
  test(`${theme} real pointer resizing, keyboard bounds, reload, and narrow draft preservation`, async ({ page }, info) => {
    await page.setViewportSize({ width: 1280, height: 900 }); await seed(page, 5);
    if (theme === 'obsidian') {
      await page.getByRole('button', { name: 'SETTINGS', exact: true }).click();
      await page.getByRole('button', { name: /^OBSIDIAN/ }).click();
      await page.getByRole('button', { name: /^DISCOVERED/ }).click();
    }
    const title = page.getByRole('button', { name: /^View Layout fixture 000 / });
    await title.click();
    const separator = page.getByRole('separator', { name: 'Job results width' });
    const before = Number(await separator.getAttribute('aria-valuenow'));
    const bounds = (await separator.boundingBox())!;
    await page.mouse.move(bounds.x + bounds.width / 2, bounds.y + 100); await page.mouse.down();
    await page.mouse.move(bounds.x - 120, bounds.y + 100, { steps: 8 });
    expect(await page.evaluate(() => localStorage.getItem('scrape:job_split:v1'))).toBeNull();
    await page.mouse.up();
    const resized = Number(await separator.getAttribute('aria-valuenow'));
    expect(resized).toBeLessThan(before);
    const input = page.getByLabel('Notes', { exact: true });
    await input.fill('Same draft through resizing');
    const original = await input.elementHandle();
    await page.getByRole('complementary', { name: 'Job details' }).locator('[data-dialog-scroll]').evaluate(el => el.scrollTop = 150);
    await page.setViewportSize({ width: 480, height: 900 });
    await expect(page.getByRole('dialog', { name: 'Job details' })).toBeVisible();
    await expect(input).toHaveValue('Same draft through resizing');
    expect(await input.evaluate((el, previous) => el === previous, original)).toBe(true);
    await page.setViewportSize({ width: 1280, height: 900 });
    await expect(separator).toBeVisible();
    expect(await input.evaluate((el, previous) => el === previous, original)).toBe(true);
    expect(await page.getByRole('complementary', { name: 'Job details' }).locator('[data-dialog-scroll]').evaluate(el => el.scrollTop)).toBe(150);
    await expect(page.getByRole('complementary', { name: 'Job details' }).getByRole('status')).toHaveText('Saved');
    await page.reload(); await title.click();
    expect(Number(await separator.getAttribute('aria-valuenow'))).toBe(resized);
    await expect(input).toHaveValue('Same draft through resizing');
    await separator.focus(); await separator.press('Home');
    expect(await separator.getAttribute('aria-valuenow')).toBe(await separator.getAttribute('aria-valuemin'));
    await separator.press('End'); expect(await separator.getAttribute('aria-valuenow')).toBe(await separator.getAttribute('aria-valuemax'));
    await page.getByRole('button', { name: 'Reset pane width' }).click();
    await expect(separator).toHaveAttribute('aria-valuenow', '60');
    const inspector = page.getByRole('complementary', { name: 'Job details' });
    expect(await inspector.evaluate(el => el.scrollWidth <= el.clientWidth)).toBe(true);
    expect(await contrastFailures(page)).toEqual([]);
    await page.screenshot({ path: info.outputPath(`${theme}-resizable-details.png`) });
    await page.getByRole('button', { name: 'Close job details' }).click(); await expect(title).toBeFocused();
  });
}
