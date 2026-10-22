import { test, expect, type Page, type Locator } from "./fixtures";

test.use({ launchOptions: { ignoreDefaultArgs: ["--hide-scrollbars"] } });

const themes = ["paper", "obsidian"] as const;
const viewports = [
  { name: "desktop", width: 1280, height: 800 },
  { name: "narrow", width: 480, height: 700 },
] as const;

async function seedSources(page: Page, count = 30) {
  await page.evaluate(async count => {
    const moduleUrl = new URL("/src/lib/api.ts", location.origin).href;
    const { SAMPLE_SWE_SOURCE } = await import(moduleUrl);
    const sources = Array.from({ length: count }, (_, index) => ({
      ...structuredClone(SAMPLE_SWE_SOURCE),
      id: `scroll-fixture-${String(index).padStart(2, "0")}`,
      name: `Fictional scroll workflow ${String(index).padStart(2, "0")}`,
      identifier: `fictional-${index}`,
    }));
    localStorage.setItem("scrape:demo:v1:sources", JSON.stringify(sources, (_, value) => typeof value === "bigint" ? Number(value) : value));
  }, count);
  await page.reload();
}

async function findRegistryScroller(page: Page): Promise<Locator> {
  const table = page.locator("table").first();
  await expect(table).toBeVisible();
  const index = await table.evaluate(element => {
    let parent = element.parentElement;
    let depth = 0;
    while (parent) {
      const style = getComputedStyle(parent);
      if (["auto", "scroll"].includes(style.overflowY) && parent.scrollHeight > parent.clientHeight) return depth;
      parent = parent.parentElement;
      depth++;
    }
    return -1;
  });
  expect(index, "workflow table has a vertically overflowing ancestor").toBeGreaterThanOrEqual(0);
  return table.locator(`xpath=..${"/..".repeat(index)}`);
}

async function wheelAt(page: Page, locator: Locator, deltaY: number, deltaX = 0) {
  const box = await locator.boundingBox();
  if (!box) throw new Error("Expected a visible scroll surface");
  const viewport = page.viewportSize();
  if (!viewport) throw new Error("Expected a browser viewport");
  const left = Math.max(5, box.x), right = Math.min(viewport.width - 5, box.x + box.width);
  const top = Math.max(5, box.y), bottom = Math.min(viewport.height - 5, box.y + box.height);
  await page.mouse.move((left + right) / 2, (top + bottom) / 2);
  await page.mouse.wheel(deltaX, deltaY);
}

for (const theme of themes) {
  for (const viewport of viewports) {
    test(`${theme} ${viewport.name}: workflow editor owns scrolling and releases registry`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto("/?demo=1&fixtures=1");
      await seedSources(page);
      if (theme === "obsidian") {
        await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
        await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
      }
      await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
      const registry = await findRegistryScroller(page);
      const shell = page.locator("body > #root > div").first();
      const footer = page.locator("footer").first();
      const lastName = "Fictional scroll workflow 29";
      await registry.evaluate(element => { element.scrollTop = element.scrollHeight; });
      await expect(page.getByRole("button", { name: `Edit workflow ${lastName}`, exact: true })).toBeVisible();
      await wheelAt(page, registry, -320);
      await expect.poll(() => registry.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
      const editName = "Fictional scroll workflow 15";
      const edit = page.getByRole("button", { name: `Edit workflow ${editName}`, exact: true });
      await edit.scrollIntoViewIfNeeded();
      const registryTop = await registry.evaluate(element => element.scrollTop);
      const shellRect = await shell.boundingBox();
      const footerRect = await footer.boundingBox();
      await edit.click();

      const dialog = page.getByRole("dialog", { name: "Edit workflow" });
      await dialog.getByRole("button", { name: "Continue to Criteria" }).click();
      await dialog.getByRole("button", { name: "Continue to Sources" }).click();
      await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
      await dialog.getByText("All configured rules", { exact: true }).click();
      const body = dialog.locator("[data-dialog-scroll]");
      expect(await registry.evaluate(element => getComputedStyle(element).overflowY)).toBe("hidden");
      await expect.poll(() => body.evaluate(element => element.scrollHeight - element.clientHeight)).toBeGreaterThan(0);
      const layers = await dialog.evaluate(element => {
        const panel = element.firstElementChild as HTMLElement;
        const body = element.querySelector<HTMLElement>("[data-dialog-scroll]")!;
        return {
          body: { top: body.scrollTop, range: body.scrollHeight - body.clientHeight, overflow: getComputedStyle(body).overflowY },
          dialog: { top: element.scrollTop, range: element.scrollHeight - element.clientHeight, overflow: getComputedStyle(element).overflowY },
          panel: { top: panel.scrollTop, range: panel.scrollHeight - panel.clientHeight, overflow: getComputedStyle(panel).overflowY },
        };
      });
      console.log(`${theme}/${viewport.name} scroll owners`, JSON.stringify(layers));
      expect(layers.body.overflow).toMatch(/auto|scroll/);
      expect(layers.dialog.top).toBe(0);
      expect(layers.panel.top).toBe(0);
      for (const layer of [layers.dialog, layers.panel]) {
        if (/auto|scroll/.test(layer.overflow)) expect(layer.range).toBeLessThanOrEqual(1);
      }

      const editorBottom = async () => body.evaluate(element => {
        element.scrollTop = element.scrollHeight;
        return new Promise<number>(resolve => requestAnimationFrame(() => resolve(element.scrollTop)));
      });
      await editorBottom();
      const finalReview = dialog.locator("details pre");
      const lastRect = await finalReview.boundingBox();
      const bodyRect = await body.boundingBox();
      expect(lastRect!.y + lastRect!.height).toBeLessThanOrEqual(bodyRect!.y + bodyRect!.height + 1);
      for (const label of ["Cancel", "Save workflow"]) {
        const button = dialog.getByRole("button", { name: label, exact: true });
        const buttonRect = await button.boundingBox();
        expect(buttonRect).not.toBeNull();
        expect(buttonRect!.y).toBeGreaterThanOrEqual(0);
        expect(buttonRect!.y + buttonRect!.height).toBeLessThanOrEqual(viewport.height);
      }
      const initialShell = shellRect;
      const initialFooter = footerRect;
      const initialDocument = await page.evaluate(() => ({ top: document.documentElement.scrollTop, body: document.body.scrollTop }));
      const backdropBox = await dialog.boundingBox();
      if (!backdropBox) throw new Error("Expected modal dialog bounds");
      await wheelAt(page, body, 900);
      await page.waitForTimeout(120);
      const atBottom = await registry.evaluate(element => element.scrollTop);
      await wheelAt(page, body, 900);
      await page.waitForTimeout(120);
      await page.mouse.move(8, 8);
      await page.mouse.wheel(0, 900);
      await page.waitForTimeout(120);
      expect(await registry.evaluate(element => element.scrollTop)).toBe(atBottom);
      await body.evaluate(element => { element.scrollTop = 0; });
      await wheelAt(page, body, -900);
      await page.waitForTimeout(120);
      const atTop = await registry.evaluate(element => element.scrollTop);
      await wheelAt(page, body, -900);
      await page.waitForTimeout(120);
      await page.mouse.move(8, 8);
      await page.mouse.wheel(0, -900);
      await page.waitForTimeout(120);
      expect(await registry.evaluate(element => element.scrollTop)).toBe(atTop);
      expect(await registry.evaluate(element => element.scrollTop)).toBe(registryTop);
      expect(await page.evaluate(() => ({ top: document.documentElement.scrollTop, body: document.body.scrollTop }))).toEqual(initialDocument);
      expect(await shell.boundingBox()).toEqual(initialShell);
      expect(await footer.boundingBox()).toEqual(initialFooter);
      await page.screenshot({ path: `plans/assets/29-workflow-scroll-review/${theme}-${viewport.name}.png` });

      await dialog.getByRole("button", { name: "Cancel", exact: true }).click();
      await expect(edit).toBeFocused();
      expect(await registry.evaluate(element => element.scrollTop)).toBe(registryTop);
      await wheelAt(page, registry, 240);
      await expect.poll(() => registry.evaluate(element => element.scrollTop)).not.toBe(registryTop);
      const add = page.getByRole("button", { name: "Add workflow", exact: true });
      await add.click();
      await expect(page.getByRole("dialog", { name: "Create workflow" })).toBeVisible();
      await page.keyboard.press("Escape");
      await expect(add).toBeFocused();
      const released = await registry.evaluate(element => {
        const style = getComputedStyle(element);
        return style.overflowY === "auto" && element.scrollHeight > element.clientHeight;
      });
      expect(released).toBe(true);
      if (viewport.name === "narrow") {
        const horizontal = page.locator("table").first().locator("xpath=..");
        await horizontal.scrollIntoViewIfNeeded();
        await horizontal.evaluate(element => { element.scrollLeft = 0; });
        const range = await horizontal.evaluate(element => element.scrollWidth - element.clientWidth);
        expect(range).toBeGreaterThan(0);
        await wheelAt(page, horizontal, 0, 320);
        await horizontal.evaluate(element => { element.scrollLeft = element.scrollWidth; });
        await expect.poll(() => horizontal.evaluate(element => element.scrollLeft)).toBeGreaterThan(0);
        await horizontal.evaluate(element => { element.scrollLeft = 0; });
      }
    });
  }
}

test("empty workflow registry can open and close a usable editor without a stale lock", async ({ page }) => {
  await page.setViewportSize({ width: 480, height: 700 });
  await page.goto("/?demo=1&fixtures=1");
  await seedSources(page, 0);
  await page.getByRole("button", { name: "SCRAPER", exact: true }).click();
  const add = page.getByRole("button", { name: "Add workflow", exact: true });
  await add.click();
  const dialog = page.getByRole("dialog", { name: "Create workflow" });
  await expect(dialog.getByRole("button", { name: "Cancel", exact: true })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Continue to Criteria" })).toBeVisible();
  await dialog.getByRole("button", { name: "Continue to Criteria" }).click();
  await dialog.getByRole("button", { name: "Continue to Sources" }).click();
  await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
  await dialog.getByText("All configured rules", { exact: true }).click();
  const body = dialog.locator("[data-dialog-scroll]");
  await expect.poll(() => body.evaluate(element => element.scrollHeight - element.clientHeight)).toBeGreaterThan(0);
  await body.hover();
  await page.mouse.wheel(0, 10000);
  await expect.poll(() => body.evaluate(element => element.scrollTop)).toBeGreaterThan(0);
  await expect(dialog.getByRole("button", { name: "Cancel", exact: true })).toBeVisible();
  const registry = page.locator(".workflow-viewport");
  await page.keyboard.press("Escape");
  await expect(add).toBeFocused();
  expect(await registry.evaluate(element => getComputedStyle(element).overflowY)).toBe("auto");
});
