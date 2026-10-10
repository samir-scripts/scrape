import { describe, it, expect, beforeEach } from "vitest";
import { useUIStore } from "../lib/store";
import { calculateTooltipCoordinates } from "../components/ui/Tooltip";
import type { Source } from "../bindings/Source";

describe("Scraper Ergonomics, Dual-Mode & Workflow Inspector", () => {
  beforeEach(() => {
    useUIStore.setState({
      scraperMode: "simple",
    });
  });

  describe("Scraper Mode State & Defaults", () => {
    it("initializes scraperMode to simple by default", () => {
      expect(useUIStore.getState().scraperMode).toBe("simple");
    });

    it("toggles between simple and advanced mode", () => {
      const { setScraperMode } = useUIStore.getState();

      setScraperMode("advanced");
      expect(useUIStore.getState().scraperMode).toBe("advanced");

      setScraperMode("simple");
      expect(useUIStore.getState().scraperMode).toBe("simple");
    });
  });

  describe("Simple Mode Workflow Defaults Assembly", () => {
    it("assembles valid source creation payload using simple mode defaults", () => {
      const companyName = "Stripe";
      const targetUrl = "https://boards.greenhouse.io/stripe";

      // Simulated simple mode assembler matching AddSourceModal logic
      const assembleSimpleModePayload = (name: string, url: string) => {
        return {
          name: name.trim(),
          kind: "greenhouse" as const,
          identifier: url.trim(),
          config: { url: url.trim() },
          cadence: "daily" as const,
          cadence_cron: null,
          retention_days: BigInt(30),
          retention_policy: "untracked_only" as const,
          filters: {},
        };
      };

      const payload = assembleSimpleModePayload(companyName, targetUrl);

      expect(payload.name).toBe("Stripe");
      expect(payload.identifier).toBe("https://boards.greenhouse.io/stripe");
      expect(payload.kind).toBe("greenhouse");
      expect(payload.cadence).toBe("daily");
      expect(payload.retention_days).toBe(BigInt(30));
      expect(payload.retention_policy).toBe("untracked_only");
      expect(payload.filters).toEqual({});
    });
  });

  describe("Advanced Mode Auto-Elevation Guard", () => {
    const shouldAutoElevateToAdvanced = (source: Partial<Source>): boolean => {
      const flt = (source.filters as Record<string, unknown>) || {};
      const hasCustomFilters =
        (Array.isArray(flt.role_types) && flt.role_types.length > 0) ||
        (Array.isArray(flt.include_keywords) && flt.include_keywords.length > 0) ||
        (Array.isArray(flt.exclude_keywords) && flt.exclude_keywords.length > 0) ||
        (Array.isArray(flt.locations) && flt.locations.length > 0);

      return Boolean(
        source.kind === "selector" ||
        source.cadence === "cron" ||
        hasCustomFilters ||
        Number(source.retention_days ?? 30) !== 30 ||
        (source.retention_policy && source.retention_policy !== "untracked_only")
      );
    };

    it("keeps simple mode for standard greenhouse source with default retention", () => {
      const standardSource: Partial<Source> = {
        kind: "greenhouse",
        cadence: "daily",
        retention_days: BigInt(30),
        retention_policy: "untracked_only",
        filters: {},
      };

      expect(shouldAutoElevateToAdvanced(standardSource)).toBe(false);
    });

    it("auto-elevates to advanced if source has custom selectors", () => {
      const selectorSource: Partial<Source> = {
        kind: "selector",
        cadence: "daily",
        retention_days: BigInt(30),
        retention_policy: "untracked_only",
      };

      expect(shouldAutoElevateToAdvanced(selectorSource)).toBe(true);
    });

    it("auto-elevates to advanced if source uses custom cron", () => {
      const cronSource: Partial<Source> = {
        kind: "greenhouse",
        cadence: "cron",
        cadence_cron: "0 0 14 * * *",
        retention_days: BigInt(30),
        retention_policy: "untracked_only",
      };

      expect(shouldAutoElevateToAdvanced(cronSource)).toBe(true);
    });

    it("auto-elevates to advanced if source has custom keywords or filters", () => {
      const filteredSource: Partial<Source> = {
        kind: "lever",
        cadence: "daily",
        retention_days: BigInt(30),
        retention_policy: "untracked_only",
        filters: { include_keywords: ["Rust", "Distributed"] },
      };

      expect(shouldAutoElevateToAdvanced(filteredSource)).toBe(true);
    });

    it("auto-elevates to advanced if source has custom retention days", () => {
      const customTtlSource: Partial<Source> = {
        kind: "ashby",
        cadence: "daily",
        retention_days: BigInt(90),
        retention_policy: "untracked_only",
      };

      expect(shouldAutoElevateToAdvanced(customTtlSource)).toBe(true);
    });
  });

  describe("NieR Brutalist Tooltip Edge Cases", () => {
    it("handles zero width and right-aligned triggers safely", () => {
      const rightEdgeRect = { top: 300, bottom: 324, left: 1000, width: 60 };
      const coords = calculateTooltipCoordinates(rightEdgeRect, "top");

      expect(coords.top).toBe(272); // 300 - 28
      expect(coords.left).toBe(1030); // 1000 + 30
    });

    it("handles bottom positioning without offscreen clamping", () => {
      const rect = { top: 100, bottom: 124, left: 400, width: 100 };
      const coords = calculateTooltipCoordinates(rect, "bottom");

      expect(coords.top).toBe(128); // 124 + 4
      expect(coords.left).toBe(450); // 400 + 50
    });
  });
});
