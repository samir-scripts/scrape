import { describe, it, expect } from "vitest";
import { formatDate, formatTime, formatDateTime } from "../lib/formatters";

describe("formatters library", () => {
  const testDate = new Date(2026, 9, 10, 17, 30, 45); // Oct 10, 2026 17:30:45 local

  describe("formatDate", () => {
    it("handles null and undefined gracefully", () => {
      expect(formatDate(null)).toBe("—");
      expect(formatDate(undefined)).toBe("—");
    });

    it("formats MM/DD/YY correctly (default)", () => {
      expect(formatDate(testDate, { dateFormat: "MM/DD/YY" })).toBe("10/10/26");
    });

    it("formats DD/MM/YY correctly", () => {
      expect(formatDate(testDate, { dateFormat: "DD/MM/YY" })).toBe("10/10/26");
    });

    it("formats MM/DD/YYYY correctly", () => {
      expect(formatDate(testDate, { dateFormat: "MM/DD/YYYY" })).toBe("10/10/2026");
    });

    it("formats DD/MM/YYYY correctly", () => {
      expect(formatDate(testDate, { dateFormat: "DD/MM/YYYY" })).toBe("10/10/2026");
    });

    it("formats YYYY-MM-DD correctly", () => {
      expect(formatDate(testDate, { dateFormat: "YYYY-MM-DD" })).toBe("2026-10-10");
    });
  });

  describe("formatTime", () => {
    it("handles null and undefined gracefully", () => {
      expect(formatTime(null)).toBe("—");
      expect(formatTime(undefined)).toBe("—");
    });

    it("formats 12h time correctly without seconds", () => {
      expect(formatTime(testDate, { timeFormat: "12h" })).toBe("05:30 PM");
    });

    it("formats 12h time correctly with seconds", () => {
      expect(formatTime(testDate, { timeFormat: "12h" }, true)).toBe("05:30:45 PM");
    });

    it("formats 24h time correctly without seconds", () => {
      expect(formatTime(testDate, { timeFormat: "24h" })).toBe("17:30");
    });

    it("formats 24h time correctly with seconds", () => {
      expect(formatTime(testDate, { timeFormat: "24h" }, true)).toBe("17:30:45");
    });
  });

  describe("formatDateTime", () => {
    it("combines date and time according to options", () => {
      expect(
        formatDateTime(testDate, { dateFormat: "MM/DD/YY", timeFormat: "12h" })
      ).toBe("10/10/26 05:30 PM");

      expect(
        formatDateTime(testDate, { dateFormat: "DD/MM/YYYY", timeFormat: "24h" })
      ).toBe("10/10/2026 17:30");
    });
  });
});
