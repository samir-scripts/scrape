import { GEOGRAPHY_CATALOG } from "../bindings/geographyCatalog";
import type { CountryOption } from "../bindings/CountryOption";
import type { RegionOption } from "../bindings/RegionOption";
export type { CountryOption, RegionOption };

export const COUNTRIES: CountryOption[] = GEOGRAPHY_CATALOG.countries.map(country => ({ ...country }));
export const REGIONS_BY_COUNTRY: Record<string, RegionOption[]> = {};
for (const region of GEOGRAPHY_CATALOG.regions) {
  (REGIONS_BY_COUNTRY[region.countryCode] ||= []).push({ ...region });
}

const TIMEZONE_LOCATION_MAP: Record<string, { country: string; region: string | null }> = {
  "America/New_York": { country: "US", region: "NY" },
  "America/Detroit": { country: "US", region: "MI" },
  "America/Chicago": { country: "US", region: "IL" },
  "America/Indiana/Indianapolis": { country: "US", region: "IN" },
  "America/Denver": { country: "US", region: "CO" },
  "America/Phoenix": { country: "US", region: "AZ" },
  "America/Los_Angeles": { country: "US", region: "CA" },
  "America/Anchorage": { country: "US", region: "AK" },
  "Pacific/Honolulu": { country: "US", region: "HI" },
  "America/Boise": { country: "US", region: "ID" },
  "America/Toronto": { country: "CA", region: "ON" },
  "America/Montreal": { country: "CA", region: "QC" },
  "America/Vancouver": { country: "CA", region: "BC" },
  "America/Edmonton": { country: "CA", region: "AB" },
  "America/Winnipeg": { country: "CA", region: "MB" },
  "America/Halifax": { country: "CA", region: "NS" },
  "America/St_Johns": { country: "CA", region: "NL" },
  "Europe/London": { country: "GB", region: "ENG" },
  "Europe/Belfast": { country: "GB", region: "NIR" },
  "Europe/Berlin": { country: "DE", region: "BE" },
  "Europe/Paris": { country: "FR", region: "IDF" },
  "Europe/Dublin": { country: "IE", region: null },
  "Europe/Amsterdam": { country: "NL", region: null },
  "Europe/Zurich": { country: "CH", region: null },
  "Asia/Kolkata": { country: "IN", region: "KA" },
  "Asia/Calcutta": { country: "IN", region: "KA" },
  "Asia/Singapore": { country: "SG", region: null },
  "Asia/Tokyo": { country: "JP", region: null },
  "Australia/Sydney": { country: "AU", region: "NSW" },
  "Australia/Melbourne": { country: "AU", region: "VIC" },
  "Australia/Brisbane": { country: "AU", region: "QLD" },
  "Australia/Perth": { country: "AU", region: "WA" },
  "Australia/Adelaide": { country: "AU", region: "SA" },
};

export function getRegionsForCountry(countryCode: string | null): RegionOption[] {
  if (!countryCode || countryCode === "ALL") {
    // If all countries or none specified, return top US and CA states
    return [
      ...(REGIONS_BY_COUNTRY["US"] || []).slice(0, 15),
      ...(REGIONS_BY_COUNTRY["CA"] || []).slice(0, 5),
    ];
  }
  return REGIONS_BY_COUNTRY[countryCode.toUpperCase()] || [];
}

export function detectLocationFromTimezone(): { country: string | null; region: string | null } {
  try {
    const tz = Intl.DateTimeFormat().resolvedOptions().timeZone;
    if (tz && TIMEZONE_LOCATION_MAP[tz]) {
      return TIMEZONE_LOCATION_MAP[tz];
    }
    // Prefix matching
    if (tz?.startsWith("America/")) {
      return { country: "US", region: null };
    }
    if (tz?.startsWith("Europe/London")) {
      return { country: "GB", region: "ENG" };
    }
  } catch {
    // ignore
  }
  return { country: null, region: null };
}

export function detectLocationFromLocale(): { country: string | null; region: string | null } {
  try {
    const lang = (navigator.language || "").toUpperCase();
    if (lang.includes("-US") || lang === "EN-US") return { country: "US", region: null };
    if (lang.includes("-CA")) return { country: "CA", region: null };
    if (lang.includes("-GB")) return { country: "GB", region: null };
    if (lang.includes("-DE") || lang === "DE") return { country: "DE", region: null };
    if (lang.includes("-FR") || lang === "FR") return { country: "FR", region: null };
    if (lang.includes("-IN") || lang === "HI") return { country: "IN", region: null };
    if (lang.includes("-AU")) return { country: "AU", region: null };
    if (lang.includes("-SG")) return { country: "SG", region: null };
    if (lang.includes("-JP") || lang === "JA") return { country: "JP", region: null };
  } catch {
    // ignore
  }
  return { country: null, region: null };
}

export function detectLocationFromCoords(
  latitude: number,
  longitude: number
): { country: string | null; region: string | null } {
  // Approximate coordinates bounding boxes
  // Canada (Southern Ontario, BC, Quebec, etc.)
  if (
    (latitude >= 43.5 && latitude <= 46.0 && longitude >= -81.5 && longitude <= -76.0) ||
    (latitude >= 49.0 && latitude <= 70.0 && longitude >= -141.0 && longitude <= -52.6)
  ) {
    if (longitude >= -95.2 && longitude <= -74.3) {
      return { country: "CA", region: "ON" };
    }
    if (longitude <= -114.0) {
      return { country: "CA", region: "BC" };
    }
    return { country: "CA", region: null };
  }

  // United States (Contiguous + AK/HI)
  if (latitude >= 24.5 && latitude <= 49.5 && longitude >= -125.0 && longitude <= -66.9) {
    if (latitude >= 32.5 && latitude <= 42.0 && longitude <= -114.1) {
      return { country: "US", region: "CA" };
    }
    if (latitude >= 45.5 && latitude <= 49.0 && longitude <= -116.9) {
      return { country: "US", region: "WA" };
    }
    if (latitude >= 25.8 && latitude <= 36.5 && longitude >= -106.6 && longitude <= -93.5) {
      return { country: "US", region: "TX" };
    }
    if (latitude >= 40.5 && latitude <= 45.0 && longitude >= -79.8 && longitude <= -71.8) {
      return { country: "US", region: "NY" };
    }
    if (latitude >= 41.2 && latitude <= 42.9 && longitude >= -73.5 && longitude <= -69.9) {
      return { country: "US", region: "MA" };
    }
    if (latitude >= 41.6 && latitude <= 42.5 && longitude >= -88.5 && longitude <= -87.5) {
      return { country: "US", region: "IL" };
    }
    if (latitude >= 37.0 && latitude <= 41.0 && longitude >= -109.0 && longitude <= -102.0) {
      return { country: "US", region: "CO" };
    }
    return { country: "US", region: null };
  }

  // UK
  if (latitude >= 49.9 && latitude <= 60.9 && longitude >= -8.6 && longitude <= 1.8) {
    return { country: "GB", region: "ENG" };
  }

  // Germany
  if (latitude >= 47.3 && latitude <= 55.1 && longitude >= 5.9 && longitude <= 15.0) {
    return { country: "DE", region: "BE" };
  }

  // France
  if (latitude >= 42.3 && latitude <= 51.1 && longitude >= -4.8 && longitude <= 9.6) {
    return { country: "FR", region: "IDF" };
  }

  // India
  if (latitude >= 8.1 && latitude <= 35.5 && longitude >= 68.1 && longitude <= 97.4) {
    return { country: "IN", region: "KA" };
  }

  // Australia
  if (latitude >= -43.7 && latitude <= -10.7 && longitude >= 113.0 && longitude <= 153.6) {
    return { country: "AU", region: "NSW" };
  }

  return { country: null, region: null };
}

export async function detectUserLocation(): Promise<{
  country: string | null;
  region: string | null;
  method: "geolocation" | "timezone" | "locale" | "none";
}> {
  // 1. Try Geolocation API with a short timeout
  if (typeof navigator !== "undefined" && navigator.geolocation) {
    try {
      const pos = await new Promise<GeolocationPosition>((resolve, reject) => {
        navigator.geolocation.getCurrentPosition(resolve, reject, {
          timeout: 2500,
          maximumAge: 600000,
        });
      });
      const coords = detectLocationFromCoords(
        pos.coords.latitude,
        pos.coords.longitude
      );
      if (coords.country) {
        return { ...coords, method: "geolocation" };
      }
    } catch {
      // Permission denied, prompt dismissed, or timeout: fall back gracefully
    }
  }

  // 2. Try Timezone resolution
  const tzLoc = detectLocationFromTimezone();
  if (tzLoc.country) {
    return { ...tzLoc, method: "timezone" };
  }

  // 3. Try Locale resolution
  const localeLoc = detectLocationFromLocale();
  if (localeLoc.country) {
    return { ...localeLoc, method: "locale" };
  }

  return { country: null, region: null, method: "none" };
}
