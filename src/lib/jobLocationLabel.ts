import type { ParsedLocation } from "../bindings/ParsedLocation";
import type { WorkplaceType } from "../bindings/WorkplaceType";
import { COUNTRIES } from "./locationData";

function countryName(code: string | null): string | null {
  if (!code) return null;
  return COUNTRIES.find((country) => country.code === code.toUpperCase())?.name ?? code;
}

function placeName(location: ParsedLocation): string | null {
  const country = countryName(location.country_code);
  const region = location.region_code?.split("-").at(-1);
  const parts = [location.city, region, country].filter(
    (value): value is string => Boolean(value),
  );
  return parts.length ? [...new Set(parts)].join(", ") : null;
}

export function formatJobLocation(
  location: string | null,
  workplaceType: WorkplaceType,
  locations: ParsedLocation[],
): string {
  const places = [...new Set(locations.map(placeName).filter((value): value is string => Boolean(value)))];
  const remote =
    workplaceType === "remote" ||
    (workplaceType === "unknown" && Boolean(location?.toLowerCase().includes("remote")));

  if (remote) return places.length ? `Remote · ${places.join(" / ")}` : "Remote";

  const workplace =
    workplaceType === "onsite"
      ? "On-site"
      : workplaceType === "hybrid"
        ? "Hybrid"
        : null;
  const place = places.join(" / ") || location?.trim() || null;
  if (workplace && place && place.toLowerCase() !== workplace.toLowerCase()) {
    return `${workplace} · ${place}`;
  }
  return place || workplace || "Location unknown";
}
