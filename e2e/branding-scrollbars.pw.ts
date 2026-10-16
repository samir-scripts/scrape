import { test, expect } from './fixtures';
import { discoverFixtures } from './jobFixtures';

// Headless Chromium hides scrollbars by default, which defeats visual acceptance.
test.use({ launchOptions: { ignoreDefaultArgs: ['--hide-scrollbars'] } });

for (const theme of ['paper', 'obsidian']) {
  test(`${theme} job logos load and results use square themed scrollbars`, async ({ page }) => {
    await page.setViewportSize({ width: 1280, height: 800 });
    await page.route('**/company-logo-fixture.svg', route => route.fulfill({
      contentType: 'image/svg+xml',
      body: '<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32"><path fill="#b34c25" d="M4 4h24v7H11v5h13v7H11v5H4z"/></svg>',
    }));
    await discoverFixtures(page);
    await page.evaluate(() => {
      const sample = JSON.parse(localStorage.getItem('scrape:demo:v1:jobs')!)[0];
      localStorage.setItem('scrape:demo:v1:jobs', JSON.stringify(Array.from({ length: 25 }, (_, i) => ({
        ...sample, id: `branding-fixture-${i}`, dedupe_key: `branding-fixture:${i}`,
        company: 'Branding fixture', company_icon_url: '/company-logo-fixture.svg',
        title: `Branding and scrollbar fixture ${String(i).padStart(2, '0')}`,
        description_text: 'Authored browser fixture for image rendering and scrolling. Fictional company and logo.',
        status: 'viewed',
      }))));
    });
    await page.reload();
    if (theme === 'obsidian') {
      await page.getByRole('button', { name: 'SETTINGS', exact: true }).click();
      await page.getByRole('button', { name: /^OBSIDIAN/ }).click();
      await page.getByRole('button', { name: /^DISCOVERED/ }).click();
    }
    const logo = page.getByRole('img', { name: 'Branding fixture' }).first();
    await expect(logo).toBeVisible();
    await expect.poll(() => logo.evaluate(image => (image as HTMLImageElement).naturalWidth)).toBe(32);
    const results = page.getByRole('region', { name: 'Job results' });
    const colors = await results.evaluate(region => {
      const scroller = [...region.querySelectorAll<HTMLElement>('*')].find(element =>
        getComputedStyle(element).overflowY === 'auto' && element.scrollHeight > element.clientHeight);
      if (!scroller) throw new Error('Expected an overflowing job list');
      scroller.scrollTop = 100;
      const thumb = getComputedStyle(scroller, '::-webkit-scrollbar-thumb');
      const track = getComputedStyle(scroller, '::-webkit-scrollbar-track');
      return {
        scrollTop: scroller.scrollTop, width: getComputedStyle(scroller, '::-webkit-scrollbar').width,
        radius: thumb.borderRadius, thumb: thumb.backgroundColor, track: track.backgroundColor,
        standardColor: getComputedStyle(scroller).scrollbarColor,
      };
    });
    expect(colors.scrollTop).toBe(100);
    expect(colors.width).toBe('8px');
    expect(colors.radius).toBe('0px');
    expect(colors.thumb).toBe(theme === 'paper' ? 'rgb(103, 99, 87)' : 'rgb(185, 181, 175)');
    expect(colors.track).toBe(theme === 'paper' ? 'rgb(230, 226, 200)' : 'rgb(54, 54, 58)');
    expect(colors.standardColor).toBe('auto');
    await page.screenshot({ path: `plans/assets/10-ui-review/branding-scrollbars-${theme}.png` });
  });
}
