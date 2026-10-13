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

  describe("Modal Cadence Exclusivity & Time Selection", () => {
    it("simple mode clock cadence generates locked daily cron", async () => {
      const { clockToCron } = await import("../lib/cronClock");
      // 09:30 AM Daily
      const cronStr = clockToCron(9, 30, "AM", "daily");
      expect(cronStr).toBe("0 30 9 * * *");

      // 04:15 PM Daily -> 16:15
      const cronStrPm = clockToCron(4, 15, "PM", "daily");
      expect(cronStrPm).toBe("0 15 16 * * *");
    });

    it("advanced mode clock cadence generates distinct day expressions", async () => {
      const { clockToCron } = await import("../lib/cronClock");
      // Weekdays at 10:00 AM
      expect(clockToCron(10, 0, "AM", "weekdays")).toBe("0 0 10 * * MON-FRI");
      // Weekends at 08:00 AM
      expect(clockToCron(8, 0, "AM", "weekends")).toBe("0 0 8 * * SAT,SUN");
      // Hourly at :45
      expect(clockToCron(1, 45, "AM", "hourly")).toBe("0 45 * * * *");
    });

    it("converts cron expressions back to clock schedule state", async () => {
      const { cronToClock } = await import("../lib/cronClock");
      const state = cronToClock("0 45 16 * * MON-FRI");
      expect(state).not.toBeNull();
      expect(state?.hour12).toBe(4);
      expect(state?.minute).toBe(45);
      expect(state?.ampm).toBe("PM");
      expect(state?.cadence).toBe("weekdays");
    });
  });

  describe("URL Pattern Globs & YAML Import Ergonomics", () => {
    it("assembles url_patterns and exclude_url_patterns in config", () => {
      const assembleConfig = (
        url: string,
        urlPatterns: string,
        excludeUrlPatterns: string
      ) => {
        const config: Record<string, unknown> = { url };
        const incGlobs = urlPatterns
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        if (incGlobs.length > 0) {
          config.url_patterns = incGlobs;
          if (incGlobs.length === 1) config.url_pattern = incGlobs[0];
        }
        const excGlobs = excludeUrlPatterns
          .split(",")
          .map((s) => s.trim())
          .filter(Boolean);
        if (excGlobs.length > 0) {
          config.exclude_url_patterns = excGlobs;
        }
        return config;
      };

      const cfgSingle = assembleConfig(
        "https://example.com/careers",
        "*/careers/*",
        "*/archive/*"
      );
      expect(cfgSingle.url_pattern).toBe("*/careers/*");
      expect(cfgSingle.url_patterns).toEqual(["*/careers/*"]);
      expect(cfgSingle.exclude_url_patterns).toEqual(["*/archive/*"]);

      const cfgMulti = assembleConfig(
        "https://example.com/careers",
        "*/careers/*, example/link/*",
        "*/archive/*, */login*"
      );
      expect(cfgMulti.url_patterns).toEqual(["*/careers/*", "example/link/*"]);
      expect(cfgMulti.exclude_url_patterns).toEqual(["*/archive/*", "*/login*"]);
    });

    it("validates and imports workflow YAML specifications with URL globs", async () => {
      const { validateWorkflowYaml } = await import("../lib/yamlWorkflow");
      const yamlContent = `
version: 2
metadata:
  name: "Acme Power Scraper"
  company: "Acme Corp"
  enabled: true
extractor:
  kind: "selector"
  url: "https://acme.com/jobs"
  job_container: ".job-listing"
  title_selector: ".title"
  url_selector: "a.apply"
  url_patterns:
    - "https://acme.com/jobs/*"
    - "example/link/*"
  exclude_url_patterns:
    - "*/archive/*"
schedule:
  cadence: "daily"
  cron: "0 0 9 * * *"
retention:
  days: 60
  policy: "untracked_only"
`;
      const res = validateWorkflowYaml(yamlContent);
      expect(res.isValid).toBe(true);
      expect(res.parsedSpec).not.toBeNull();
      expect(res.parsedSpec?.metadata.name).toBe("Acme Power Scraper");
      expect(res.parsedSpec?.extractor.url_patterns).toEqual([
        "https://acme.com/jobs/*",
        "example/link/*",
      ]);
      expect(res.parsedSpec?.extractor.exclude_url_patterns).toEqual([
        "*/archive/*",
      ]);
      expect(res.parsedSpec?.retention?.days).toBe(60);
    });

    it("rejects unparseable YAML and returns structured line diagnostics", async () => {
      const { validateWorkflowYaml } = await import("../lib/yamlWorkflow");
      const brokenYaml = `
version: 2
metadata:
  name: "Broken"
  company: [unclosed list
extractor:
  kind: "greenhouse"
`;
      const res = validateWorkflowYaml(brokenYaml);
      expect(res.isValid).toBe(false);
      expect(res.errors.length).toBeGreaterThan(0);
      expect(res.errors[0].line).toBeDefined();
    });
  });
});

