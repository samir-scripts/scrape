import { test, expect } from './fixtures';
import { discoverFixtures } from './jobFixtures';
import { moveByPointer } from './kanbanPointer';
import { contrastFailures } from './contrast';
const title = 'Software Engineer Intern (2027) - Austin, TX';
for (const theme of ['paper', 'obsidian']) test(`${theme} handle pointer, cancellation, Move to and restart`, async ({ page }) => {
 await page.setViewportSize({ width: 1440, height: 900 }); await discoverFixtures(page);
 if (theme === 'obsidian') { await page.getByRole('button', { name: 'SETTINGS', exact: true }).click(); await page.getByRole('button', { name: /^OBSIDIAN/ }).click(); await page.getByRole('button', { name: /^DISCOVERED/ }).click(); }
 await page.getByRole('button', { name: `View ${title} at Demo Company` }).click();
 await page.getByRole('button', { name: 'Mark as applied', exact: true }).click();
 await page.getByRole('button', { name: 'View in Tracked', exact: true }).click();
 const handle = page.getByRole('button', { name: `Drag ${title} to another stage` });
 await expect(handle.locator('svg')).toBeVisible();
 await moveByPointer(page, title, 'interviewing');
 await handle.focus(); await page.keyboard.press('Space'); await page.keyboard.press('ArrowRight'); await page.keyboard.press('Escape');
 await expect(page.getByRole('region', { name: 'INTERVIEWING stage' }).getByRole('article')).toHaveCount(1);
 const before = (await handle.boundingBox())!;
 await page.mouse.move(before.x + 5, before.y + 5); await page.mouse.down();
 const empty = (await page.getByRole('region', { name: 'REJECTED stage' }).boundingBox())!;
 await page.mouse.move(empty.x + 80, empty.y + 100, { steps: 8 }); await page.keyboard.press('Escape'); await page.mouse.up();
 await expect(page.getByRole('region', { name: 'INTERVIEWING stage' }).getByRole('article')).toHaveCount(1);
 await page.getByRole('combobox', { name: `Move ${title} to` }).selectOption('offer');
 await expect(page.getByRole('region', { name: 'OFFER stage' }).getByRole('article')).toHaveCount(1);
 expect(await contrastFailures(page)).toEqual([]);
 await page.reload(); await expect(page.getByRole('combobox', { name: `Move ${title} to` })).toHaveValue('offer');
 // Narrow boards retain all drop zones in a horizontal scroller; edge movement
 // reveals an offscreen target without hiding stages behind a mutually exclusive view.
 await page.setViewportSize({ width: 480, height: 900 });
 await handle.scrollIntoViewIfNeeded(); const source = (await handle.boundingBox())!;
 const board = page.locator('.tracked-scroll'); await board.evaluate(node => { node.scrollLeft = 0; });
 await page.getByLabel('Board stage').selectOption('offer');
 const visible = (await handle.boundingBox())!;
 await page.mouse.move(visible.x + 4, visible.y + 4); await page.mouse.down();
 const offset = await board.evaluate(node => node.scrollLeft);
 await page.mouse.move(472, visible.y + 40, { steps: 10 });
 await expect.poll(() => board.evaluate(node => node.scrollLeft)).toBeGreaterThan(offset);
 await page.keyboard.press('Escape'); await page.mouse.up();
 await expect(page.getByRole('combobox', { name: `Move ${title} to` })).toHaveValue('offer');
 expect(source.width).toBeGreaterThan(0);
});
