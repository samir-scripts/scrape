import type { Page, Locator } from "@playwright/test";
export async function contrastFailures(page: Page) {
  // Capture a deliberate resting state, rather than incidental hover/tooltip timing.
  await page.mouse.move(0, 0);
  await page.evaluate(() => new Promise<void>(resolve => requestAnimationFrame(() => requestAnimationFrame(() => resolve()))));
  return page.evaluate(() => {
    const canvas = document.createElement("canvas"); canvas.width = canvas.height = 1;
    const context = canvas.getContext("2d", { willReadFrequently: true })!;
    const colors = new Map<string, number[]>();
    const rgb = (value: string): number[] => {
      if (colors.has(value)) return [...colors.get(value)!];
      context.clearRect(0, 0, 1, 1); context.fillStyle = value; context.fillRect(0, 0, 1, 1);
      const [r, g, b, alpha] = context.getImageData(0, 0, 1, 1).data;
      const result = [r, g, b, alpha / 255]; colors.set(value, result); return [...result];
    };
    const blend = (fg: number[], bg: number[]) => fg.slice(0, 3).map((c, i) => c * fg[3] + bg[i] * (1 - fg[3]));
    const luminance = (color: number[]) => color.map(value => value / 255).map(v => v <= .04045 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4).reduce((sum, value, i) => sum + value * [.2126, .7152, .0722][i], 0);
    const result: { text: string; contrast: number; foreground: string; background: number[] }[] = [];
    const modal = document.querySelector('dialog[open]:not([data-inline-dialog="true"])');
    for (const element of document.querySelectorAll<HTMLElement>("body *")) {
      if (modal && !modal.contains(element)) continue;
      if (!Array.from(element.childNodes).some(node => node.nodeType === Node.TEXT_NODE && node.textContent?.trim())) continue;
      if (element.closest("[aria-hidden=true], [disabled], script, style")) continue;
      const style = getComputedStyle(element); const bounds = element.getBoundingClientRect();
      if (!bounds.width || !bounds.height || bounds.bottom <= 0 || bounds.top >= innerHeight || bounds.right <= 0 || bounds.left >= innerWidth || style.visibility !== "visible") continue;
      const chain: HTMLElement[] = []; let ancestor: HTMLElement | null = element;
      while (ancestor) { chain.unshift(ancestor); ancestor = ancestor.parentElement; }
      let background = [255, 255, 255]; let opacity = 1;
      for (const item of chain) { const computed = getComputedStyle(item); background = blend(rgb(computed.backgroundColor), background); opacity *= Number(computed.opacity); }
      const foreground = rgb(style.color); foreground[3] *= opacity;
      const fg = luminance(blend(foreground, background)); const bg = luminance(background);
      const contrast = (Math.max(fg, bg) + .05) / (Math.min(fg, bg) + .05);
      const large = parseFloat(style.fontSize) >= 24 || (parseFloat(style.fontSize) >= 18.66 && parseInt(style.fontWeight) >= 700);
      if (contrast + .01 < (large ? 3 : 4.5)) result.push({ text: element.textContent!.trim().slice(0, 80), contrast: Number(contrast.toFixed(2)), foreground: style.color, background });
    }
    return result;
  });
}

export async function indicatorContrast(control: Locator, property: "color" | "outlineColor") {
  return control.evaluate((element, property) => {
    const canvas = document.createElement("canvas"); canvas.width = canvas.height = 1;
    const context = canvas.getContext("2d", { willReadFrequently: true })!;
    const luminance = (value: string) => {
      context.clearRect(0, 0, 1, 1); context.fillStyle = value; context.fillRect(0, 0, 1, 1);
      const rgb = [...context.getImageData(0, 0, 1, 1).data].slice(0, 3).map(c => c / 255).map(c => c <= .04045 ? c / 12.92 : ((c + .055) / 1.055) ** 2.4);
      return rgb.reduce((sum, c, i) => sum + c * [.2126, .7152, .0722][i], 0);
    };
    const style = getComputedStyle(element);
    const surface = getComputedStyle(document.documentElement).getPropertyValue("--color-surface").trim();
    // The remove icon and focus ring are essential indicators against the card surface.
    const a = luminance(style[property]); const b = luminance(surface);
    return (Math.max(a, b) + .05) / (Math.min(a, b) + .05);
  }, property);
}
