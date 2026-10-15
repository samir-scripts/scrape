import { test, expect } from "./fixtures";
import { contrastFailures } from "./contrast";
for (const theme of ["paper", "obsidian"]) {
 test(`${theme} schedule control explains browser limitations and restores keyboard focus`, async ({page}) => {
  await page.setViewportSize({width:480,height:800});
  await page.goto('/?demo=1&fixtures=1');
  await page.getByRole('button',{name:'SETTINGS',exact:true}).click();
  if(theme==='obsidian') await page.getByRole('button',{name:/^OBSIDIAN/}).click();
  const trigger=page.getByRole('button',{name:'Schedule: Desktop required'});
  await trigger.focus();await page.keyboard.press('Enter');
  const summary=page.getByRole('dialog',{name:'Schedule summary'});await expect(summary).toBeVisible();
  await expect(summary).toContainText('Browser examples do not run an automatic scheduler');
  await expect(summary.getByRole('button',{name:'Pause automatic runs'})).toHaveCount(0);
  expect(await contrastFailures(page)).toEqual([]);
  await expect(page).toHaveScreenshot(`${theme}-schedule-summary.png`,{animations:'disabled'});
  await page.keyboard.press('Escape');await expect(trigger).toBeFocused();
  await trigger.click();await summary.getByRole('button',{name:'Open Scraper'}).click();
  await expect(page.getByRole('button',{name:'Run all',exact:true})).toBeVisible();
 });
}
