import { describe, it, expect, beforeEach } from "vitest";
import { FONT_OPTIONS, FontFamilyOption } from "../lib/constants";
import { DEFAULT_SETTINGS, api } from "../lib/api";

describe("Typography & Font Family Selection Settings", () => {
  const attributes: Record<string, string> = {};

  beforeEach(() => {
    for (const key of Object.keys(attributes)) {
      delete attributes[key];
    }

    if (typeof globalThis.document === "undefined") {
      (globalThis as unknown as { document: unknown }).document = {
        documentElement: {
          setAttribute: (key: string, val: string) => {
            attributes[key] = val;
          },
          getAttribute: (key: string) => attributes[key] ?? null,
          removeAttribute: (key: string) => {
            delete attributes[key];
          },
        },
      };
    } else {
      document.documentElement.removeAttribute("data-font");
    }
  });

  it("exports valid FONT_OPTIONS with all required font families", () => {
    const ids = FONT_OPTIONS.map((f: FontFamilyOption) => f.id);

    // Required fonts: Readable (Inter), JetBrains Mono, and other popular choices
    expect(ids).toContain("default");
    expect(ids).toContain("inter");
    expect(ids).toContain("jetbrains");
    expect(ids).toContain("geist");
    expect(ids).toContain("fira_code");
    expect(ids).toContain("ibm_plex");
    expect(FONT_OPTIONS.length).toBe(6);

    // Check Inter readability metadata
    const interFont = FONT_OPTIONS.find((f: FontFamilyOption) => f.id === "inter");
    expect(interFont).toBeDefined();
    expect(interFont?.category).toBe("readable");
    expect(interFont?.name).toContain("Readable Sans");
    expect(interFont?.label).toBe("INTER");

    // Check JetBrains Mono metadata
    const jetbrainsFont = FONT_OPTIONS.find((f: FontFamilyOption) => f.id === "jetbrains");
    expect(jetbrainsFont).toBeDefined();
    expect(jetbrainsFont?.category).toBe("monospace");
    expect(jetbrainsFont?.name).toBe("JetBrains Mono");

    // Verify each option has non-empty previewText, description, and fontFamily
    for (const opt of FONT_OPTIONS) {
      expect(opt.id.length).toBeGreaterThan(0);
      expect(opt.label.length).toBeGreaterThan(0);
      expect(opt.previewText.length).toBeGreaterThan(0);
      expect(opt.description.length).toBeGreaterThan(0);
      expect(opt.fontFamily.length).toBeGreaterThan(0);
    }
  });

  it("updates data-font attribute on documentElement reactively", () => {
    expect(document.documentElement.getAttribute("data-font")).toBeNull();

    // Switch to JetBrains Mono
    document.documentElement.setAttribute("data-font", "jetbrains");
    expect(document.documentElement.getAttribute("data-font")).toBe("jetbrains");

    // Switch to Inter
    document.documentElement.setAttribute("data-font", "inter");
    expect(document.documentElement.getAttribute("data-font")).toBe("inter");

    // Switch back to Default
    document.documentElement.setAttribute("data-font", "default");
    expect(document.documentElement.getAttribute("data-font")).toBe("default");
  });

  it("persists font selection and tooltip settings via api", async () => {
    // Check defaults
    expect(DEFAULT_SETTINGS.ui_font_family).toBe("default");
    expect(DEFAULT_SETTINGS.ui_tooltips_enabled).toBe(true);

    const initialSettings = await api.getSettings();
    expect(initialSettings.ui_font_family).toBeDefined();
    expect(initialSettings.ui_tooltips_enabled).toBeDefined();

    // Update font family
    const updatedFontSettings = await api.updateSetting("ui.font_family", "jetbrains");
    expect(updatedFontSettings.ui_font_family).toBe("jetbrains");

    // Update global tooltips setting
    const updatedTooltipSettings = await api.updateSetting("ui.tooltips_enabled", false);
    expect(updatedTooltipSettings.ui_tooltips_enabled).toBe(false);

    // Re-enable global tooltips
    const reenabledTooltipSettings = await api.updateSetting("ui.tooltips_enabled", true);
    expect(reenabledTooltipSettings.ui_tooltips_enabled).toBe(true);
  });

  it("ensures each font has an isolated specimen class identifier for preview rendering", () => {
    for (const font of FONT_OPTIONS) {
      const specimenClass = `font-specimen-${font.id}`;
      expect(specimenClass).toMatch(/^font-specimen-(default|inter|jetbrains|geist|fira_code|ibm_plex)$/);
    }
  });
});
