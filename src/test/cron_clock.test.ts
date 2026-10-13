import { describe, it, expect } from "vitest";
import {
  to24Hour,
  from24Hour,
  clockToCron,
  cronToClock,
  CLOCK_CADENCE_OPTIONS,
  type ClockCadence,
} from "../lib/cronClock";

describe("Cron-Clock Synchronizer & Geometry Math", () => {
  describe("12-Hour <-> 24-Hour Conversion", () => {
    it("converts AM hours correctly", () => {
      expect(to24Hour(12, "AM")).toBe(0); // Midnight
      expect(to24Hour(1, "AM")).toBe(1);
      expect(to24Hour(6, "AM")).toBe(6);
      expect(to24Hour(11, "AM")).toBe(11);
    });

    it("converts PM hours correctly", () => {
      expect(to24Hour(12, "PM")).toBe(12); // Noon
      expect(to24Hour(1, "PM")).toBe(13);
      expect(to24Hour(6, "PM")).toBe(18);
      expect(to24Hour(11, "PM")).toBe(23);
    });

    it("converts 24-hour integers back to 12-hour and AM/PM", () => {
      expect(from24Hour(0)).toEqual({ hour12: 12, ampm: "AM" });
      expect(from24Hour(1)).toEqual({ hour12: 1, ampm: "AM" });
      expect(from24Hour(11)).toEqual({ hour12: 11, ampm: "AM" });
      expect(from24Hour(12)).toEqual({ hour12: 12, ampm: "PM" });
      expect(from24Hour(13)).toEqual({ hour12: 1, ampm: "PM" });
      expect(from24Hour(23)).toEqual({ hour12: 11, ampm: "PM" });
    });

    it("has bijective symmetry for all 24 hours of the day", () => {
      for (let h24 = 0; h24 < 24; h24++) {
        const { hour12, ampm } = from24Hour(h24);
        const backTo24 = to24Hour(hour12, ampm);
        expect(backTo24).toBe(h24);
      }
    });
  });

  describe("clockToCron Generation", () => {
    it("generates daily cron expressions (0 {min} {h24} * * *)", () => {
      expect(clockToCron(9, 30, "AM", "daily")).toBe("0 30 9 * * *");
      expect(clockToCron(12, 0, "AM", "daily")).toBe("0 0 0 * * *");
      expect(clockToCron(12, 15, "PM", "daily")).toBe("0 15 12 * * *");
      expect(clockToCron(11, 59, "PM", "daily")).toBe("0 59 23 * * *");
    });

    it("generates weekdays cron expressions (0 {min} {h24} * * MON-FRI)", () => {
      expect(clockToCron(9, 0, "AM", "weekdays")).toBe("0 0 9 * * MON-FRI");
      expect(clockToCron(5, 45, "PM", "weekdays")).toBe("0 45 17 * * MON-FRI");
    });

    it("generates weekends cron expressions (0 {min} {h24} * * SAT,SUN)", () => {
      expect(clockToCron(10, 0, "AM", "weekends")).toBe("0 0 10 * * SAT,SUN");
      expect(clockToCron(2, 30, "PM", "weekends")).toBe("0 30 14 * * SAT,SUN");
    });

    it("generates interval cadences", () => {
      expect(clockToCron(9, 15, "AM", "every_12h")).toBe("0 15 */12 * * *");
      expect(clockToCron(12, 0, "PM", "every_6h")).toBe("0 0 */6 * * *");
      expect(clockToCron(4, 45, "PM", "hourly")).toBe("0 45 * * * *");
    });

    it("clamps out-of-range minutes safely", () => {
      expect(clockToCron(9, -5, "AM", "daily")).toBe("0 0 9 * * *");
      expect(clockToCron(9, 75, "AM", "daily")).toBe("0 59 9 * * *");
    });
  });

  describe("cronToClock Parsing", () => {
    it("parses 6-field daily cron expressions", () => {
      const parsed = cronToClock("0 30 9 * * *");
      expect(parsed).toEqual({
        hour12: 9,
        minute: 30,
        ampm: "AM",
        cadence: "daily",
      });
    });

    it("parses midnight and afternoon daily cron", () => {
      const midnight = cronToClock("0 0 0 * * *");
      expect(midnight).toEqual({
        hour12: 12,
        minute: 0,
        ampm: "AM",
        cadence: "daily",
      });

      const evening = cronToClock("0 45 22 * * *");
      expect(evening).toEqual({
        hour12: 10,
        minute: 45,
        ampm: "PM",
        cadence: "daily",
      });
    });

    it("parses weekdays cron expressions (numeric 2-6 and named MON-FRI)", () => {
      const numWeekdays = cronToClock("0 0 9 * * 2-6");
      expect(numWeekdays).toEqual({
        hour12: 9,
        minute: 0,
        ampm: "AM",
        cadence: "weekdays",
      });

      const namedWeekdays = cronToClock("0 15 14 * * MON-FRI");
      expect(namedWeekdays).toEqual({
        hour12: 2,
        minute: 15,
        ampm: "PM",
        cadence: "weekdays",
      });
    });

    it("parses weekends cron expressions", () => {
      const numWeekends = cronToClock("0 0 10 * * 7,1");
      expect(numWeekends).toEqual({
        hour12: 10,
        minute: 0,
        ampm: "AM",
        cadence: "weekends",
      });

      const namedWeekends = cronToClock("0 30 11 * * SAT,SUN");
      expect(namedWeekends).toEqual({
        hour12: 11,
        minute: 30,
        ampm: "AM",
        cadence: "weekends",
      });
    });

    it("parses interval and hourly cron expressions", () => {
      const ev12 = cronToClock("0 15 */12 * * *");
      expect(ev12).toEqual({
        hour12: 12,
        minute: 15,
        ampm: "PM",
        cadence: "every_12h",
      });

      const ev6 = cronToClock("0 20 */6 * * *");
      expect(ev6).toEqual({
        hour12: 6,
        minute: 20,
        ampm: "AM",
        cadence: "every_6h",
      });

      const hourly = cronToClock("0 45 * * * *");
      expect(hourly).toEqual({
        hour12: 12,
        minute: 45,
        ampm: "AM",
        cadence: "hourly",
      });
    });

    it("supports standard 5-field cron parsing", () => {
      const fiveField = cronToClock("30 14 * * MON-FRI");
      expect(fiveField).toEqual({
        hour12: 2,
        minute: 30,
        ampm: "PM",
        cadence: "weekdays",
      });
    });

    it("returns null for non-standard/unparseable expressions", () => {
      expect(cronToClock("invalid string")).toBeNull();
      expect(cronToClock("0 0 9 * * 1-5")).toBeNull(); // Sunday–Thursday stays an expert rule.
      expect(cronToClock("0 0 9 * * 6,7")).toBeNull(); // Friday–Saturday.
      expect(cronToClock("0 0 0 0")).toBeNull();
      expect(cronToClock("0 0 25 * * *")).toBeNull(); // Hour 25 is invalid
    });

    it("maintains round-trip fidelity between clockToCron and cronToClock", () => {
      const testCases: { h: number; m: number; ap: "AM" | "PM"; cad: ClockCadence }[] = [
        { h: 9, m: 0, ap: "AM", cad: "daily" },
        { h: 12, m: 30, ap: "PM", cad: "daily" },
        { h: 8, m: 15, ap: "AM", cad: "weekdays" },
        { h: 11, m: 45, ap: "PM", cad: "weekdays" },
        { h: 10, m: 0, ap: "AM", cad: "weekends" },
        { h: 4, m: 20, ap: "PM", cad: "weekends" },
      ];

      for (const tc of testCases) {
        const cron = clockToCron(tc.h, tc.m, tc.ap, tc.cad);
        const parsed = cronToClock(cron);
        expect(parsed).toEqual({
          hour12: tc.h,
          minute: tc.m,
          ampm: tc.ap,
          cadence: tc.cad,
        });
      }
    });
  });

  describe("Cadence Option Descriptions", () => {
    it("generates correct human-readable descriptions", () => {
      const daily = CLOCK_CADENCE_OPTIONS.find((c) => c.id === "daily");
      expect(daily?.description(9, 30, "AM")).toBe("EVERY DAY AT 09:30 AM");

      const weekdays = CLOCK_CADENCE_OPTIONS.find((c) => c.id === "weekdays");
      expect(weekdays?.description(5, 0, "PM")).toBe("MON–FRI AT 05:00 PM");

      const weekends = CLOCK_CADENCE_OPTIONS.find((c) => c.id === "weekends");
      expect(weekends?.description(10, 15, "AM")).toBe("SAT–SUN AT 10:15 AM");

      const every6h = CLOCK_CADENCE_OPTIONS.find((c) => c.id === "every_6h");
      expect(every6h?.description(0, 15, "AM")).toBe("EVERY 6 HOURS AT :15");

      const hourly = CLOCK_CADENCE_OPTIONS.find((c) => c.id === "hourly");
      expect(hourly?.description(0, 45, "AM")).toBe("HOURLY AT :45");
    });
  });
});
