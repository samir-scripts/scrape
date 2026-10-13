import { describe, it, expect } from "vitest";
import { cronToClock, clockToCron, type ClockScheduleState } from "../lib/cronClock";

describe("ScheduleSettings Tab Logic & State Machine", () => {
  it("determines initial tab mode based on active cron expression", () => {
    const getInitialMode = (cronExpr: string): "clock" | "cron" => {
      return cronToClock(cronExpr) ? "clock" : "cron";
    };

    // Standard clock expressions default to 'clock'
    expect(getInitialMode("0 0 9 * * *")).toBe("clock");
    expect(getInitialMode("0 30 14 * * MON-FRI")).toBe("clock");
    expect(getInitialMode("0 0 */6 * * *")).toBe("clock");
    expect(getInitialMode("0 15 * * * *")).toBe("clock");

    // Arbitrary/non-standard cron expressions default to 'cron'
    expect(getInitialMode("0 15 2,8,14 * * 2,4")).toBe("cron");
    expect(getInitialMode("non standard cron")).toBe("cron");
  });

  it("synchronizes clock state when switching from custom cron to clock tab", () => {
    let cronInput = "0 45 18 * * MON-FRI"; // 6:45 PM Weekdays
    let currentClockState: ClockScheduleState = {
      hour12: 9,
      minute: 0,
      ampm: "AM",
      cadence: "daily",
    };

    // Simulate switching mode to 'clock'
    const parsed = cronToClock(cronInput);
    if (parsed) {
      currentClockState = parsed;
    }

    expect(currentClockState).toEqual({
      hour12: 6,
      minute: 45,
      ampm: "PM",
      cadence: "weekdays",
    });
  });

  it("correctly generates updated cron when applying clock schedule", () => {
    const clockState: ClockScheduleState = {
      hour12: 10,
      minute: 15,
      ampm: "AM",
      cadence: "weekends",
    };

    const newCron = clockToCron(
      clockState.hour12,
      clockState.minute,
      clockState.ampm,
      clockState.cadence
    );

    expect(newCron).toBe("0 15 10 * * SAT,SUN");
  });

  it("detects when custom cron is out of sync with standard clock representation", () => {
    const isOutOfSync = (activeCron: string): boolean => {
      return !cronToClock(activeCron);
    };

    expect(isOutOfSync("0 0 9 * * *")).toBe(false); // standard daily -> in sync
    expect(isOutOfSync("0 15 2,8,14 * * 2,4")).toBe(true); // arbitrary -> out of sync
  });
});
