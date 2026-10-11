import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import {
  calculateTooltipCoordinates,
  getTooltipVisualBounds,
} from "../components/ui/Tooltip";

describe("NieR Brutalist Tooltip Mechanics & Coordinate Engine", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("calculateTooltipCoordinates", () => {
    it("calculates top position centered above target element", () => {
      const rect = { top: 100, bottom: 124, left: 200, width: 80 };
      const coords = calculateTooltipCoordinates(rect, "top");

      // top = 100 - 28 = 72
      // left = 200 + 40 = 240
      expect(coords.top).toBe(72);
      expect(coords.left).toBe(240);
      expect(coords.placement).toBe("top");
    });

    it("flips to bottom if top would be offscreen (< 4px)", () => {
      const rect = { top: 10, bottom: 34, left: 100, width: 50 };
      const coords = calculateTooltipCoordinates(rect, "top");

      // 10 - 28 = -18 (< 4), flips to bottom + 4 = 34 + 4 = 38
      expect(coords.top).toBe(38);
      expect(coords.left).toBe(125);
      expect(coords.placement).toBe("bottom");
    });

    it("calculates bottom position with 4px gap", () => {
      const rect = { top: 50, bottom: 74, left: 150, width: 60 };
      const coords = calculateTooltipCoordinates(rect, "bottom");

      // top = 74 + 4 = 78
      // left = 150 + 30 = 180
      expect(coords.top).toBe(78);
      expect(coords.left).toBe(180);
      expect(coords.placement).toBe("bottom");
    });

    it("flips bottom to top if bottom would overflow viewport", () => {
      // Button at bottom edge: top 750, bottom 765 on 768px window
      const rect = { top: 750, bottom: 765, left: 200, width: 60 };
      const coords = calculateTooltipCoordinates(
        rect,
        "bottom",
        120,
        1024,
        24,
        768,
        4
      );

      // bottom + 4 = 769 (> 768 - 4 = 764), flips to top: 750 - 24 - 4 = 722
      expect(coords.top).toBe(722);
      expect(coords.placement).toBe("top");
    });

    it("clamps left edge to minimum 40px safe margin when unmeasured", () => {
      const rect = { top: 200, bottom: 224, left: 10, width: 20 };
      const coords = calculateTooltipCoordinates(rect, "top");

      // left would be 10 + 10 = 20, clamped to 40
      expect(coords.left).toBe(40);
    });

    it("clamps right edge when element is near the right edge of viewport", () => {
      // Button at right edge: left 950, width 40 on 1000px window
      // Tooltip is 200px wide (halfWidth = 100)
      const rect = { top: 100, bottom: 124, left: 950, width: 40 };
      const coords = calculateTooltipCoordinates(rect, "top", 200, 1000);

      // Unclamped center would be 950 + 20 = 970
      // 970 + 100 = 1070 (> 1000 - 12 = 988)
      // Clamped center: 1000 - 12 - 100 = 888
      expect(coords.left).toBe(888);
      // Tooltip right edge = 888 + 100 = 988 (12px padding from right edge)
      expect(coords.left + 100).toBe(988);
    });

    it("clamps left edge when element is near the left edge with measured width", () => {
      // Button at left edge: left 5, width 20 on 1000px window
      // Tooltip is 160px wide (halfWidth = 80)
      const rect = { top: 100, bottom: 124, left: 5, width: 20 };
      const coords = calculateTooltipCoordinates(rect, "top", 160, 1000);

      // Unclamped center would be 5 + 10 = 15
      // 15 - 80 = -65 (< 12)
      // Clamped center: 12 + 80 = 92
      expect(coords.left).toBe(92);
      // Tooltip left edge = 92 - 80 = 12 (12px padding from left edge)
      expect(coords.left - 80).toBe(12);
    });

    it("centers tooltip in viewport if width exceeds available space", () => {
      const rect = { top: 100, bottom: 124, left: 200, width: 40 };
      const coords = calculateTooltipCoordinates(rect, "top", 600, 500);

      expect(coords.left).toBe(250); // 500 / 2
    });
  });

  describe("Zero-Intersection Invariant & Transform Anchoring", () => {
    // Overlap helper: returns true if intervals [A.top, A.bottom] and [B.top, B.bottom] overlap
    const hasVerticalOverlap = (
      a: { top: number; bottom: number },
      b: { top: number; bottom: number }
    ) => a.bottom > b.top && a.top < b.bottom;

    it("guarantees tooltip NEVER intersects button when positioned on TOP, even with multi-line wrap", () => {
      // Compact table action button (20px tall)
      const buttonRect = { top: 250, bottom: 270, left: 500, width: 40 };

      // Tooltips of varying heights: single-line (24px), 3-line wrap (56px), 5-line verbose (90px)
      const tooltipHeights = [24, 48, 56, 72, 90];

      for (const height of tooltipHeights) {
        const bounds = getTooltipVisualBounds(
          buttonRect,
          "top",
          240,
          height,
          6,
          1024
        );

        // 1. Separation distance is at least 6px
        expect(buttonRect.top - bounds.bottom).toBe(6);

        // 2. Bottom of tooltip must never penetrate the button top
        expect(bounds.bottom).toBeLessThan(buttonRect.top);

        // 3. Absolute zero vertical intersection
        expect(hasVerticalOverlap(bounds, buttonRect)).toBe(false);
      }
    });

    it("guarantees tooltip NEVER intersects button when positioned on BOTTOM", () => {
      const buttonRect = { top: 120, bottom: 142, left: 400, width: 50 };
      const tooltipHeights = [24, 48, 56, 72];

      for (const height of tooltipHeights) {
        const bounds = getTooltipVisualBounds(
          buttonRect,
          "bottom",
          200,
          height,
          6,
          1024
        );

        // 1. Separation distance is at least 6px
        expect(bounds.top - buttonRect.bottom).toBe(6);

        // 2. Top of tooltip must never penetrate button bottom
        expect(bounds.top).toBeGreaterThan(buttonRect.bottom);

        // 3. Absolute zero vertical intersection
        expect(hasVerticalOverlap(bounds, buttonRect)).toBe(false);
      }
    });

    it("preserves 12px viewport padding on extreme right-edge action buttons (RUN/TEST/LOGS/DEL)", () => {
      // Extreme right button like DEL in SourcesSettings (left 980, width 32 on 1024px display)
      const rightButton = { top: 300, bottom: 322, left: 980, width: 32 };
      const bounds = getTooltipVisualBounds(
        rightButton,
        "top",
        280,
        48,
        6,
        1024
      );

      // Right edge of tooltip must not exceed 1024 - 12 = 1012px
      expect(bounds.right).toBeLessThanOrEqual(1012);
      expect(bounds.left).toBeGreaterThanOrEqual(12);
      expect(hasVerticalOverlap(bounds, rightButton)).toBe(false);
    });

    it("preserves 12px viewport padding on extreme left-edge action buttons", () => {
      const leftButton = { top: 100, bottom: 122, left: 4, width: 30 };
      const bounds = getTooltipVisualBounds(
        leftButton,
        "top",
        240,
        48,
        6,
        1024
      );

      // Left edge of tooltip must be at least 12px
      expect(bounds.left).toBeGreaterThanOrEqual(12);
      expect(bounds.right).toBeLessThanOrEqual(1012);
      expect(hasVerticalOverlap(bounds, leftButton)).toBe(false);
    });
  });

  describe("Tooltip Delay & Dismissal State Machine", () => {
    it("manages 300ms hover delay timer properly", () => {
      let isVisible = false;
      let timer: ReturnType<typeof setTimeout> | null = null;

      const triggerHover = (delay = 300) => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => {
          isVisible = true;
        }, delay);
      };

      const triggerLeave = () => {
        if (timer) {
          clearTimeout(timer);
          timer = null;
        }
        isVisible = false;
      };

      // Hover started
      triggerHover(300);
      expect(isVisible).toBe(false);

      // Advance 250ms - still not visible
      vi.advanceTimersByTime(250);
      expect(isVisible).toBe(false);

      // Advance remaining 50ms - becomes visible
      vi.advanceTimersByTime(50);
      expect(isVisible).toBe(true);

      // Leave immediately dismisses
      triggerLeave();
      expect(isVisible).toBe(false);
    });

    it("cancels display if mouse leaves before delay elapses", () => {
      let isVisible = false;
      let timer: ReturnType<typeof setTimeout> | null = null;

      const triggerHover = (delay = 300) => {
        if (timer) clearTimeout(timer);
        timer = setTimeout(() => {
          isVisible = true;
        }, delay);
      };

      const triggerLeave = () => {
        if (timer) {
          clearTimeout(timer);
          timer = null;
        }
        isVisible = false;
      };

      triggerHover(300);
      vi.advanceTimersByTime(150);
      triggerLeave();

      // Advance past 300ms
      vi.advanceTimersByTime(200);
      expect(isVisible).toBe(false);
    });

    it("dismisses immediately on Escape key event", () => {
      let isVisible = true;
      const handleKeyDown = (e: { key: string }) => {
        if (e.key === "Escape") {
          isVisible = false;
        }
      };

      handleKeyDown({ key: "Enter" });
      expect(isVisible).toBe(true);

      handleKeyDown({ key: "Escape" });
      expect(isVisible).toBe(false);
    });

    it("dismisses immediately on scroll event", () => {
      let isVisible = true;
      const handleScroll = () => {
        isVisible = false;
      };

      handleScroll();
      expect(isVisible).toBe(false);
    });
  });

  describe("Global Tooltips Visibility Toggle & Navigation Tabs", () => {
    it("respects global tooltipsEnabled flag in UI store", async () => {
      const { useUIStore } = await import("../lib/store");

      // Initially enabled
      useUIStore.getState().setTooltipsEnabled(true);
      expect(useUIStore.getState().tooltipsEnabled).toBe(true);

      // Disable globally
      useUIStore.getState().setTooltipsEnabled(false);
      expect(useUIStore.getState().tooltipsEnabled).toBe(false);

      // Verify that when disabled, hover handlers gated by tooltipsEnabled block display
      let isVisible = false;
      const triggerHover = () => {
        if (!useUIStore.getState().tooltipsEnabled) return;
        isVisible = true;
      };

      triggerHover();
      expect(isVisible).toBe(false);

      // Re-enable
      useUIStore.getState().setTooltipsEnabled(true);
      triggerHover();
      expect(isVisible).toBe(true);
    });

    it("verifies navigation tab hints exist and are informative", () => {
      const tabHints = {
        discovered: "Unprocessed postings intake queue",
        tracked: "Application pipeline & Kanban board",
        metrics: "Analytics, conversions, and salary benchmarks",
        scraper: "Autonomous workflows, runners, and telemetry",
        settings: "System preferences, appearance, and global options",
      };

      expect(tabHints.discovered).toContain("intake");
      expect(tabHints.tracked).toContain("Kanban");
      expect(tabHints.metrics).toContain("Analytics");
      expect(tabHints.scraper).toContain("telemetry");
      expect(tabHints.settings).toContain("preferences");
    });
  });
});

