import { describe, it, expect, vi } from "vitest";
import {
  COUNTRIES,
  REGIONS_BY_COUNTRY,
  getRegionsForCountry,
  detectLocationFromCoords,
  detectLocationFromTimezone,
  detectLocationFromLocale,
  detectUserLocation,
} from "../lib/locationData";

describe("locationData", () => {
  it("exports valid countries and regions", () => {
    expect(COUNTRIES.length).toBeGreaterThan(5);
    expect(REGIONS_BY_COUNTRY["US"]).toBeDefined();
    expect(REGIONS_BY_COUNTRY["CA"]).toBeDefined();
    expect(REGIONS_BY_COUNTRY["US"].length).toBeGreaterThanOrEqual(50);
  });

  it("cascades regions based on country", () => {
    const usRegions = getRegionsForCountry("US");
    expect(usRegions.some((r) => r.code === "CA")).toBe(true);
    expect(usRegions.some((r) => r.code === "NY")).toBe(true);
    expect(usRegions.every((r) => r.countryCode === "US")).toBe(true);

    const caRegions = getRegionsForCountry("CA");
    expect(caRegions.some((r) => r.code === "ON")).toBe(true);
    expect(caRegions.some((r) => r.code === "BC")).toBe(true);
    expect(caRegions.every((r) => r.countryCode === "CA")).toBe(true);

    const gbRegions = getRegionsForCountry("GB");
    expect(gbRegions.some((r) => r.code === "ENG")).toBe(true);

    // Fallback for ALL or unknown
    const allRegions = getRegionsForCountry("ALL");
    expect(allRegions.length).toBeGreaterThan(0);
  });

  it("detects location from GPS coordinates bounding boxes", () => {
    // San Francisco coords (37.7749, -122.4194)
    const sf = detectLocationFromCoords(37.7749, -122.4194);
    expect(sf.country).toBe("US");
    expect(sf.region).toBe("CA");

    // Seattle coords (47.6062, -122.3321)
    const seattle = detectLocationFromCoords(47.6062, -122.3321);
    expect(seattle.country).toBe("US");
    expect(seattle.region).toBe("WA");

    // Toronto coords (43.6532, -79.3832)
    const toronto = detectLocationFromCoords(43.6532, -79.3832);
    expect(toronto.country).toBe("CA");
    expect(toronto.region).toBe("ON");

    // London coords (51.5074, -0.1278)
    const london = detectLocationFromCoords(51.5074, -0.1278);
    expect(london.country).toBe("GB");
  });

  it("detects location from timezones", () => {
    // Mock Intl.DateTimeFormat
    const originalResolvedOptions = Intl.DateTimeFormat().resolvedOptions;
    const resolvedSpy = vi.spyOn(Intl.DateTimeFormat.prototype, "resolvedOptions");

    resolvedSpy.mockReturnValue({
      ...originalResolvedOptions.call(Intl.DateTimeFormat()),
      timeZone: "America/Los_Angeles",
    });

    const laLoc = detectLocationFromTimezone();
    expect(laLoc.country).toBe("US");
    expect(laLoc.region).toBe("CA");

    resolvedSpy.mockReturnValue({
      ...originalResolvedOptions.call(Intl.DateTimeFormat()),
      timeZone: "America/Toronto",
    });

    const toLoc = detectLocationFromTimezone();
    expect(toLoc.country).toBe("CA");
    expect(toLoc.region).toBe("ON");

    resolvedSpy.mockRestore();
  });

  it("falls back to timezone when geolocation fails or is denied", async () => {
    const resolvedSpy = vi.spyOn(Intl.DateTimeFormat.prototype, "resolvedOptions");
    resolvedSpy.mockReturnValue({
      ...Intl.DateTimeFormat().resolvedOptions(),
      timeZone: "America/New_York",
    });

    // Mock navigator.geolocation error
    const originalGeolocation = global.navigator.geolocation;
    Object.defineProperty(global.navigator, "geolocation", {
      value: {
        getCurrentPosition: (_success: any, error: any) => {
          error(new Error("Permission denied"));
        },
      },
      configurable: true,
    });

    const detected = await detectUserLocation();
    expect(detected.country).toBe("US");
    expect(detected.region).toBe("NY");
    expect(detected.method).toBe("timezone");

    resolvedSpy.mockRestore();
    Object.defineProperty(global.navigator, "geolocation", {
      value: originalGeolocation,
      configurable: true,
    });
  });

  it("detects country from navigator locale", () => {
    const originalLanguage = global.navigator.language;
    Object.defineProperty(global.navigator, "language", {
      value: "en-US",
      configurable: true,
    });
    expect(detectLocationFromLocale().country).toBe("US");

    Object.defineProperty(global.navigator, "language", {
      value: "en-CA",
      configurable: true,
    });
    expect(detectLocationFromLocale().country).toBe("CA");

    Object.defineProperty(global.navigator, "language", {
      value: originalLanguage,
      configurable: true,
    });
  });
});
