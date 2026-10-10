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

  describe("24H Concentric Dial Snapping", () => {
    const snap24Hour = (deg: number, distance: number): number => {
      const step = 360 / 12;
      const sector = Math.round(deg / step) % 12;
      const isInner = distance < 79;
      return isInner ? (sector === 0 ? 0 : sector + 12) : (sector === 0 ? 12 : sector);
    };

    it("snaps outer ring to 01-12", () => {
      // 0 deg outer -> 12:00
      expect(snap24Hour(0, 94)).toBe(12);
      // 90 deg outer -> 03:00
      expect(snap24Hour(90, 94)).toBe(3);
      // 180 deg outer -> 06:00
      expect(snap24Hour(180, 94)).toBe(6);
      // 270 deg outer -> 09:00
      expect(snap24Hour(270, 94)).toBe(9);
    });

    it("snaps inner ring to 13-23 and 00", () => {
      // 0 deg inner -> 00:00 (Midnight)
      expect(snap24Hour(0, 64)).toBe(0);
      // 90 deg inner -> 15:00
      expect(snap24Hour(90, 64)).toBe(15);
      // 180 deg inner -> 18:00
      expect(snap24Hour(180, 64)).toBe(18);
      // 270 deg inner -> 21:00
      expect(snap24Hour(270, 64)).toBe(21);
      // 30 deg inner -> 13:00
      expect(snap24Hour(30, 64)).toBe(13);
      // 330 deg inner -> 23:00
      expect(snap24Hour(330, 64)).toBe(23);
    });
  });

  describe("Direct Typing & Input Validation", () => {
    it("auto-advances single digit 12h hours >= 2 immediately", () => {
      const shouldAutoAdvance12h = (input: string): boolean => {
        if (input.length === 1) {
          const digit = parseInt(input, 10);
          return digit >= 2;
        }
        return input.length >= 2;
      };

      expect(shouldAutoAdvance12h("3")).toBe(true);
      expect(shouldAutoAdvance12h("9")).toBe(true);
      expect(shouldAutoAdvance12h("1")).toBe(false); // buffer "1" for 10, 11, 12
      expect(shouldAutoAdvance12h("12")).toBe(true);
    });

    it("auto-advances single digit 24h hours >= 3 immediately", () => {
      const shouldAutoAdvance24h = (input: string): boolean => {
        if (input.length === 1) {
          const digit = parseInt(input, 10);
          return digit >= 3;
        }
        return input.length >= 2;
      };

      expect(shouldAutoAdvance24h("4")).toBe(true);
      expect(shouldAutoAdvance24h("8")).toBe(true);
      expect(shouldAutoAdvance24h("2")).toBe(false); // buffer "2" for 20-23
      expect(shouldAutoAdvance24h("23")).toBe(true);
    });

    it("auto-advances minute input when tens digit >= 6", () => {
      const shouldAutoAdvanceMin = (input: string): boolean => {
        if (input.length === 1) {
          const digit = parseInt(input, 10);
          return digit >= 6;
        }
        return input.length >= 2;
      };

      expect(shouldAutoAdvanceMin("7")).toBe(true);
      expect(shouldAutoAdvanceMin("4")).toBe(false);
      expect(shouldAutoAdvanceMin("45")).toBe(true);
    });

    it("handles overtyping when length > 2 by extracting the latest keystroke", () => {
      const resolveRawInput = (value: string): string => {
        let raw = value.replace(/\D/g, "");
        if (!raw) return "";
        if (raw.length > 2) {
          raw = raw.slice(-1);
        }
        return raw;
      };

      expect(resolveRawInput("094")).toBe("4");
      expect(resolveRawInput("125")).toBe("5");
      expect(resolveRawInput("001")).toBe("1");
      expect(resolveRawInput("08")).toBe("08");
      expect(resolveRawInput("")).toBe("");
    });

    it("simulates 12H direct typing state machine and blur transitions", () => {
      // Helper simulating the ClockPicker 12H typing and blur logic
      class ClockPickerTypingHarness {
        hour12 = 9;
        minute = 0;
        hourDraft: string | null = null;
        hourDraftRef: string | null = null;
        mode: "hours" | "minutes" = "hours";

        setDraft(val: string | null) {
          this.hourDraft = val;
          this.hourDraftRef = val;
        }

        typeInput(text: string) {
          let raw = text.replace(/\D/g, "");
          if (!raw) {
            this.setDraft("");
            return;
          }
          if (raw.length > 2) {
            raw = raw.slice(-1);
          }

          if (raw.length === 1) {
            const digit = parseInt(raw, 10);
            if (digit >= 2) {
              this.hour12 = digit;
              this.setDraft(null);
              this.mode = "minutes";
              // Synchronous blur occurs when focus moves to minute input
              this.handleBlur();
            } else {
              this.setDraft(raw);
              if (digit === 1) {
                this.hour12 = 1;
              }
            }
          } else {
            let val = parseInt(raw.slice(0, 2), 10);
            if (val === 0) val = 12;
            if (val > 12) val = 12;
            this.hour12 = val;
            this.setDraft(null);
            this.mode = "minutes";
            this.handleBlur();
          }
        }

        handleBlur() {
          const draft = this.hourDraftRef;
          if (draft === null) {
            return; // Already committed; blur is no-op
          }
          if (draft === "" || draft === "0") {
            const val = draft === "0" ? 12 : this.hour12;
            this.hour12 = val;
          } else {
            let val = parseInt(draft, 10);
            let clamped = Math.min(12, Math.max(1, val));
            if (clamped === 0) clamped = 12;
            this.hour12 = clamped;
          }
          this.setDraft(null);
        }
      }

      // 1. Typing '4' directly should set hour to 4, advance to minutes, and NOT default to 12 on blur
      const harness1 = new ClockPickerTypingHarness();
      harness1.typeInput("4");
      expect(harness1.hour12).toBe(4);
      expect(harness1.mode).toBe("minutes");
      expect(harness1.hourDraftRef).toBeNull();

      // 2. Typing '1' (draft '1') and then blurring should commit 1, NOT 12
      const harness2 = new ClockPickerTypingHarness();
      harness2.typeInput("1");
      expect(harness2.hourDraftRef).toBe("1");
      harness2.handleBlur();
      expect(harness2.hour12).toBe(1);
      expect(harness2.hourDraftRef).toBeNull();

      // 3. Typing '1' then '0' commits 10 and does not revert to 12
      const harness3 = new ClockPickerTypingHarness();
      harness3.typeInput("1");
      harness3.typeInput("10");
      expect(harness3.hour12).toBe(10);
      expect(harness3.mode).toBe("minutes");

      // 4. Typing '1' then '2' commits 12
      const harness4 = new ClockPickerTypingHarness();
      harness4.typeInput("1");
      harness4.typeInput("12");
      expect(harness4.hour12).toBe(12);
      expect(harness4.mode).toBe("minutes");

      // 5. Backspacing to empty and blurring should revert to previous value (9), NOT force 12
      const harness5 = new ClockPickerTypingHarness();
      harness5.typeInput("");
      expect(harness5.hourDraftRef).toBe("");
      harness5.handleBlur();
      expect(harness5.hour12).toBe(9);

      // 6. Typing '0' (draft) and blurring defaults to 12 because 0 is invalid in 12H
      const harness6 = new ClockPickerTypingHarness();
      harness6.typeInput("0");
      expect(harness6.hourDraftRef).toBe("0");
      harness6.handleBlur();
      expect(harness6.hour12).toBe(12);

      // 7. Typing '0' then '5' sets 5
      const harness7 = new ClockPickerTypingHarness();
      harness7.typeInput("0");
      harness7.typeInput("05");
      expect(harness7.hour12).toBe(5);
      expect(harness7.mode).toBe("minutes");

      // 8. Overtyping '09' with '3' without selecting text: '093' -> '3' sets 3
      const harness8 = new ClockPickerTypingHarness();
      harness8.typeInput("093");
      expect(harness8.hour12).toBe(3);
      expect(harness8.mode).toBe("minutes");
    });

    it("simulates 24H direct typing state machine and conversions", () => {
      class ClockPicker24HTypingHarness {
        hour24 = 14;
        hourDraftRef: string | null = null;
        mode: "hours" | "minutes" = "hours";

        setDraft(val: string | null) {
          this.hourDraftRef = val;
        }

        typeInput(text: string) {
          let raw = text.replace(/\D/g, "");
          if (!raw) {
            this.setDraft("");
            return;
          }
          if (raw.length > 2) {
            raw = raw.slice(-1);
          }

          if (raw.length === 1) {
            const digit = parseInt(raw, 10);
            if (digit >= 3) {
              this.hour24 = digit;
              this.setDraft(null);
              this.mode = "minutes";
              this.handleBlur();
            } else {
              this.setDraft(raw);
              this.hour24 = digit;
            }
          } else {
            const val = Math.min(23, Math.max(0, parseInt(raw.slice(0, 2), 10)));
            this.hour24 = val;
            this.setDraft(null);
            this.mode = "minutes";
            this.handleBlur();
          }
        }

        handleBlur() {
          const draft = this.hourDraftRef;
          if (draft === null) return;
          if (draft === "" || draft === "0") {
            this.hour24 = 0;
          } else {
            const val = parseInt(draft, 10);
            this.hour24 = Math.min(23, Math.max(0, val));
          }
          this.setDraft(null);
        }
      }

      // Typing '5' auto-advances to minutes with 05
      const h1 = new ClockPicker24HTypingHarness();
      h1.typeInput("5");
      expect(h1.hour24).toBe(5);
      expect(h1.mode).toBe("minutes");

      // Typing '2' then '1' commits 21
      const h2 = new ClockPicker24HTypingHarness();
      h2.typeInput("2");
      h2.typeInput("21");
      expect(h2.hour24).toBe(21);
      expect(h2.mode).toBe("minutes");

      // Typing '2' then '8' clamps to 23
      const h3 = new ClockPicker24HTypingHarness();
      h3.typeInput("2");
      h3.typeInput("28");
      expect(h3.hour24).toBe(23);
      expect(h3.mode).toBe("minutes");

      // Typing '0' and blurring commits 0 (Midnight 00:00)
      const h4 = new ClockPicker24HTypingHarness();
      h4.typeInput("0");
      h4.handleBlur();
      expect(h4.hour24).toBe(0);
    });
  });
});
