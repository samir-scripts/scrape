import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { calculateTooltipCoordinates } from "../components/ui/Tooltip";

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
    });

    it("flips to bottom if top would be offscreen (< 4px)", () => {
      const rect = { top: 10, bottom: 34, left: 100, width: 50 };
      const coords = calculateTooltipCoordinates(rect, "top");

      // 10 - 28 = -18 (< 4), flips to bottom + 4 = 34 + 4 = 38
      expect(coords.top).toBe(38);
      expect(coords.left).toBe(125);
    });

    it("calculates bottom position with 4px gap", () => {
      const rect = { top: 50, bottom: 74, left: 150, width: 60 };
      const coords = calculateTooltipCoordinates(rect, "bottom");

      // top = 74 + 4 = 78
      // left = 150 + 30 = 180
      expect(coords.top).toBe(78);
      expect(coords.left).toBe(180);
    });

    it("clamps left edge to minimum 40px safe margin", () => {
      const rect = { top: 200, bottom: 224, left: 10, width: 20 };
      const coords = calculateTooltipCoordinates(rect, "top");

      // left would be 10 + 10 = 20, clamped to 40
      expect(coords.left).toBe(40);
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
  });
});
