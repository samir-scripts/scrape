import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("../styles/index.css", import.meta.url), "utf8");
const tokens = (section: string) => Object.fromEntries(
  [...section.matchAll(/--color-([\w-]+):\s*(#[\da-f]{6})/gi)].map((m) => [m[1], m[2]])
);
const paper = tokens(css.split('[data-theme="obsidian"]')[0]);
const obsidian = { ...paper, ...tokens(css.split('[data-theme="obsidian"]')[1].split("}")[0]) };
function luminance(hex: string) {
  const channels = [1, 3, 5].map((offset) => parseInt(hex.slice(offset, offset + 2), 16) / 255)
    .map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4);
  return channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722;
}
describe("semantic text token contrast (not a substitute for rendered QA)", () => {
  for (const [theme, colors] of Object.entries({ paper, obsidian })) {
    for (const foreground of ["ink", "ink-mute", "danger", "success", "warning", "information"]) {
      for (const background of ["paper", "paper-2"]) {
        it(`${theme}: ${foreground} on ${background} meets 4.5:1`, () => {
          const values = [luminance(colors[foreground]), luminance(colors[background])].sort((a, b) => b - a);
          expect((values[0] + 0.05) / (values[1] + 0.05)).toBeGreaterThanOrEqual(4.5);
        });
      }
    }
  }
});
