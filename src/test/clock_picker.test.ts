import { describe, it, expect } from "vitest";
import {
  clockToCron,
  formatTime12,
  type ClockScheduleState,
} from "../lib/cronClock";

describe("ClockPicker Mechanics & Geometry Snapping", () => {
  // Polar angle calculation helper matching ClockPicker internal logic
  const angleToHour = (deg: number): number => {
    const step = 360 / 12; // 30 degrees
    let h = Math.round(deg / step) % 12;
    return h === 0 ? 12 : h;
  };

  const angleToMinute = (deg: number): number => {
    const step = 360 / 60; // 6 degrees
    return Math.round(deg / step) % 60;
  };

  describe("Angle-to-Hour Snapping (30° increments)", () => {
    it("snaps cardinal directions accurately", () => {
      expect(angleToHour(0)).toBe(12);   // North (12 o'clock)
      expect(angleToHour(90)).toBe(3);   // East (3 o'clock)
      expect(angleToHour(180)).toBe(6);  // South (6 o'clock)
      expect(angleToHour(270)).toBe(9);  // West (9 o'clock)
      expect(angleToHour(360)).toBe(12); // Full rotation wraps to 12
    });

    it("snaps intermediate hours symmetrically", () => {
      expect(angleToHour(30)).toBe(1);
      expect(angleToHour(60)).toBe(2);
      expect(angleToHour(120)).toBe(4);
      expect(angleToHour(150)).toBe(5);
      expect(angleToHour(210)).toBe(7);
      expect(angleToHour(240)).toBe(8);
      expect(angleToHour(300)).toBe(10);
      expect(angleToHour(330)).toBe(11);
    });

    it("handles boundary threshold tolerances within +/- 14 degrees", () => {
      // 1 o'clock bin is [15°, 45°) centered at 30°
      expect(angleToHour(16)).toBe(1);
      expect(angleToHour(29)).toBe(1);
      expect(angleToHour(31)).toBe(1);
      expect(angleToHour(44)).toBe(1);

      // 12 o'clock bin wraps across 0° / 360°
      expect(angleToHour(350)).toBe(12);
      expect(angleToHour(5)).toBe(12);
      expect(angleToHour(14)).toBe(12);
    });
  });

  describe("Angle-to-Minute Snapping (6° increments)", () => {
    it("snaps 5-minute major milestones", () => {
      expect(angleToMinute(0)).toBe(0);
      expect(angleToMinute(30)).toBe(5);
      expect(angleToMinute(60)).toBe(10);
      expect(angleToMinute(90)).toBe(15);
      expect(angleToMinute(180)).toBe(30);
      expect(angleToMinute(270)).toBe(45);
      expect(angleToMinute(330)).toBe(55);
      expect(angleToMinute(360)).toBe(0);
    });

    it("snaps precise 1-minute increments", () => {
      expect(angleToMinute(6)).toBe(1);
      expect(angleToMinute(12)).toBe(2);
      expect(angleToMinute(18)).toBe(3);
      expect(angleToMinute(24)).toBe(4);
      expect(angleToMinute(42)).toBe(7);
      expect(angleToMinute(222)).toBe(37);
    });

    it("handles minute threshold boundaries within +/- 2.9 degrees", () => {
      // 0 minute bin is [357°, 360°) U [0°, 3°)
      expect(angleToMinute(358)).toBe(0);
      expect(angleToMinute(1)).toBe(0);
      expect(angleToMinute(2.9)).toBe(0);

      // 1 minute bin is [3°, 9°) centered at 6°
      expect(angleToMinute(3.1)).toBe(1);
      expect(angleToMinute(6)).toBe(1);
      expect(angleToMinute(8.9)).toBe(1);
    });
  });

  describe("Polar to Cartesian Needle Geometry", () => {
    const CENTER = 130;
    const RADIUS = 88;

    const getHandTip = (angleDeg: number) => {
      const rad = (angleDeg * Math.PI) / 180;
      return {
        x: Math.round(CENTER + RADIUS * Math.sin(rad)),
        y: Math.round(CENTER - RADIUS * Math.cos(rad)),
      };
    };

    it("calculates cardinal hand needle tip coordinates", () => {
      // 12 o'clock (0°): directly above center
      expect(getHandTip(0)).toEqual({ x: 130, y: 42 });

      // 3 o'clock (90°): directly right of center
      expect(getHandTip(90)).toEqual({ x: 218, y: 130 });

      // 6 o'clock (180°): directly below center
      expect(getHandTip(180)).toEqual({ x: 130, y: 218 });

      // 9 o'clock (270°): directly left of center
      expect(getHandTip(270)).toEqual({ x: 42, y: 130 });
    });
  });

  describe("State Synchronization & Cadence Switching", () => {
    it("updates target cron when cadence changes without changing selected time", () => {
      const state: ClockScheduleState = {
        hour12: 8,
        minute: 15,
        ampm: "AM",
        cadence: "daily",
      };

      expect(clockToCron(state.hour12, state.minute, state.ampm, "daily")).toBe("0 15 8 * * *");
      expect(clockToCron(state.hour12, state.minute, state.ampm, "weekdays")).toBe("0 15 8 * * 1-5");
      expect(clockToCron(state.hour12, state.minute, state.ampm, "weekends")).toBe("0 15 8 * * 6,7");
      expect(clockToCron(state.hour12, state.minute, state.ampm, "every_12h")).toBe("0 15 */12 * * *");
      expect(clockToCron(state.hour12, state.minute, state.ampm, "every_6h")).toBe("0 15 */6 * * *");
      expect(clockToCron(state.hour12, state.minute, state.ampm, "hourly")).toBe("0 15 * * * *");
    });

    it("updates target cron when AM/PM is toggled", () => {
      const morningCron = clockToCron(9, 0, "AM", "daily");
      const eveningCron = clockToCron(9, 0, "PM", "daily");

      expect(morningCron).toBe("0 0 9 * * *");
      expect(eveningCron).toBe("0 0 21 * * *");
    });

    it("formats digital time display with zero-padding", () => {
      expect(formatTime12(9, 5, "AM")).toBe("09:05 AM");
      expect(formatTime12(12, 0, "PM")).toBe("12:00 PM");
      expect(formatTime12(1, 30, "PM")).toBe("01:30 PM");
    });
  });
});
