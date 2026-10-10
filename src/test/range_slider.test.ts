import { describe, it, expect } from "vitest";

describe("RangeSlider Logic", () => {
  it("computes percentage offsets accurately across min/max range", () => {
    const min = 0;
    const max = 500000;
    const getPercent = (val: number) =>
      Math.max(0, Math.min(100, ((val - min) / (max - min)) * 100));

    expect(getPercent(0)).toBe(0);
    expect(getPercent(250000)).toBe(50);
    expect(getPercent(500000)).toBe(100);
    expect(getPercent(-100)).toBe(0);
    expect(getPercent(600000)).toBe(100);
  });

  it("prevents dual-thumb collision where min exceeds max", () => {
    const clampValues = (
      newMin: number,
      newMax: number,
      minBound: number,
      maxBound: number
    ): [number, number] => {
      const clampedMin = Math.max(minBound, Math.min(newMin, newMax));
      const clampedMax = Math.min(maxBound, Math.max(newMax, clampedMin));
      return [clampedMin, clampedMax];
    };

    expect(clampValues(100000, 200000, 0, 500000)).toEqual([100000, 200000]);
    // When min is dragged past max:
    expect(clampValues(300000, 200000, 0, 500000)).toEqual([200000, 200000]);
    // Out of bounds:
    expect(clampValues(-5000, 600000, 0, 500000)).toEqual([0, 500000]);
  });
});
