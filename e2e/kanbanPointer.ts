import type { Page } from '@playwright/test';
import { expect } from './fixtures';
export async function moveByPointer(page: Page, title: string, targetStage: string) {
 const handle = page.getByRole('button', { name: `Drag ${title} to another stage` });
 await handle.scrollIntoViewIfNeeded();
 const start = (await handle.boundingBox())!;
 await page.mouse.move(start.x + start.width / 2, start.y + start.height / 2); await page.mouse.down();
 const target = page.getByRole('region', { name: `${targetStage.toUpperCase()} stage` });
 await target.scrollIntoViewIfNeeded();
 const end = (await target.boundingBox())!;
 await page.mouse.move(end.x + end.width / 2, end.y + 100, { steps: 12 });
 await expect(target).toContainText('Drop here'); await page.mouse.up();
 await expect(target.getByRole('button', { name: `View ${title} at Demo Company` })).toBeVisible();
}
