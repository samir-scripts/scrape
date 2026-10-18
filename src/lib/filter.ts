import type { Job } from "../bindings/Job";
import type { JobFilter } from "../bindings/JobFilter";
import type { TriState } from "../bindings/TriState";
import type { CompensationRange } from "../bindings/CompensationRange";

/**
 * Checks whether a job satisfies the given filter specification.
 * Directly mirrors the SQLite WHERE clauses in `src-tauri/src/db/jobs.rs`.
 */
export function matchesJob(job: Job, filter: Partial<JobFilter>): boolean {
  // 1. Search text / Contains
  if (filter.search && filter.search.trim()) {
    const q = filter.search.trim().toLowerCase();
    const titleMatch = job.title.toLowerCase().includes(q);
    const companyMatch = job.company.toLowerCase().includes(q);
    const locMatch = job.location?.toLowerCase().includes(q) ?? false;
    const descMatch = job.description_text?.toLowerCase().includes(q) ?? false;
    const notesMatch = job.notes?.toLowerCase().includes(q) ?? false;
    if (!titleMatch && !companyMatch && !locMatch && !descMatch && !notesMatch) {
      return false;
    }
  }

  // 2. Role types
  if (filter.role_types && filter.role_types.length > 0) {
    if (!filter.role_types.includes(job.role_type)) {
      return false;
    }
  }

  // 3. Statuses
  if (filter.statuses && filter.statuses.length > 0) {
    if (!filter.statuses.includes(job.status)) {
      return false;
    }
  }

  // 4. Source ID
  if (filter.source_id && job.source_id !== filter.source_id) {
    return false;
  }

  // 4b. Favorited only
  if (filter.favorited_only && !job.favorited) {
    return false;
  }

  const ranges = jobCompensation(job);
  const salaryActive = filter.salary_min != null || filter.salary_max != null || !!filter.salary_interval || !!(filter.salary_currency && filter.salary_currency !== "ALL");
  if (salaryActive) {
    const currency = filter.salary_currency && filter.salary_currency !== "ALL" ? filter.salary_currency.toUpperCase() : null;
    const matched = ranges.some((range) => {
      if (filter.salary_interval && range.interval !== filter.salary_interval) return false;
      if (currency && range.currency?.toUpperCase() !== currency) return false;
      if (range.min_minor == null && range.max_minor == null) return !!filter.salary_include_unknown;
      return (filter.salary_min == null || range.max_minor == null || range.max_minor >= Math.round(filter.salary_min * 100)) &&
        (filter.salary_max == null || range.min_minor == null || range.min_minor <= Math.round(filter.salary_max * 100));
    });
    if (!matched && !(ranges.length === 0 && filter.salary_include_unknown && !filter.salary_interval && !currency)) return false;
  }

  // 7. Experience (YOE) overlap
  const hasExpFilter =
    filter.experience_min != null || filter.experience_max != null;
  if (hasExpFilter) {
    const minYoe = filter.experience_min ?? 0;
    const maxYoe = filter.experience_max ?? Number.POSITIVE_INFINITY;

    if (job.experience_min == null && job.experience_max == null) {
      if (!filter.experience_include_unknown) {
        return false;
      }
    } else {
      const jMin = job.experience_min ?? 0;
      const jMax = job.experience_max;
      const overlaps =
        jMin <= maxYoe && (jMax == null || jMax >= minYoe);
      if (!overlaps) {
        return false;
      }
    }
  }

  // 8. New Grad tri-toggle
  if (filter.new_grad && filter.new_grad !== "any") {
    if ((filter.new_grad === "only" || filter.new_grad === "new_grad_only") && job.is_new_grad !== "yes") {
      return false;
    }
    if ((filter.new_grad === "exclude" || filter.new_grad === "exclude_new_grad") && job.is_new_grad === "yes") {
      return false;
    }
  }

  // 9. Sponsorship tri-state
  if (filter.sponsorship && filter.sponsorship.length > 0) {
    if (!filter.sponsorship.includes(job.sponsorship)) {
      return false;
    }
  }

  // 9b. Work authorization / Green Card
  if (filter.work_auth && filter.work_auth.length > 0) {
    if (!filter.work_auth.includes(job.work_auth)) {
      return false;
    }
  }

  // 10. Clearance tri-state
  if (filter.clearance && filter.clearance.length > 0) {
    if (!filter.clearance.includes(job.clearance)) {
      return false;
    }
  }

  if (filter.remote && !["all", "any"].includes(filter.remote)) {
    const mode = filter.remote.replace("_only", "");
    if (mode === "exclude_remote" ? job.workplace_type === "remote" : job.workplace_type !== mode) return false;
  }
  const countries = filter.countries || [], regions = filter.regions || [], cities = filter.cities || [];
  if (countries.length || regions.length || cities.length) {
    const locations = job.locations || [];
    if (!locations.length) {
      if (!filter.location_include_unknown) return false;
    } else {
      const match = locations.some((loc) => {
        const countryMatch = !countries.length || countries.some((c) => c.toUpperCase() === loc.country_code?.toUpperCase());
        const regionMatch = !regions.length || regions.some((r) => loc.region_code?.toUpperCase() === r.toUpperCase() || loc.region_code?.toUpperCase().endsWith(`-${r.toUpperCase()}`));
        const cityMatch = !cities.length || cities.some((city) => {
          const cLower = city.toLowerCase();
          return loc.city?.toLowerCase().includes(cLower) || loc.raw?.toLowerCase().includes(cLower) || (job.location?.toLowerCase().includes(cLower) ?? false);
        });
        return countryMatch && regionMatch && cityMatch;
      });
      if (!match) return false;
    }
  }

  return true;
}

/**
 * Maps TriState to clean brutalist user-facing string.
 */
export function triStateToLabel(value: TriState): "Yes" | "No" | "N/A" {
  switch (value) {
    case "yes":
      return "Yes";
    case "no":
      return "No";
    case "unknown":
    default:
      return "N/A";
  }
}

/**
 * Format salary into human-readable compact string.
 */
export function jobCompensation(job: Job): CompensationRange[] {
  if (job.compensation) return job.compensation;
  // The annual legacy contract is unambiguous. Legacy hourly values require
  // backend migration; client code never guesses their unit from magnitude.
  if (job.salary_interval === "year" && (job.salary_min != null || job.salary_max != null)) return [{
    min_minor: job.salary_min == null ? null : Math.round(job.salary_min * 100),
    max_minor: job.salary_max == null ? null : Math.round(job.salary_max * 100),
    currency: job.salary_currency, interval: "year", raw: job.salary_raw || "", source: job.salary_source || "structured", derived: true, location: null,
  }];
  return [];
}
export function formatSalary(job: Job): string {
  const ranges = jobCompensation(job);
  if (!ranges.length) return job.salary_raw ? (job.salary_min != null || job.salary_max != null ? `${job.salary_raw} (unit unverified)` : job.salary_raw) : "—";
  return ranges.map((range) => {
    const symbol = range.currency === "USD" ? "$" : range.currency ? `${range.currency} ` : "";
    const amount = (minor: number) => {
      const value = minor / 100;
      if (range.interval === "year" && value >= 1000) return `${Number.isInteger(value / 1000) ? value / 1000 : (value / 1000).toFixed(1)}K`;
      return value.toLocaleString("en-US", { maximumFractionDigits: 2 });
    };
    const unit = range.interval === "hour" ? "/HR" : range.interval && range.interval !== "year" ? `/${range.interval.toUpperCase()}` : range.interval ? "" : " (unit unknown)";
    const text = range.min_minor != null && range.max_minor != null ? range.min_minor === range.max_minor ? `${symbol}${amount(range.min_minor)}` : `${symbol}${amount(range.min_minor)} - ${symbol}${amount(range.max_minor)}` : range.min_minor != null ? `FROM ${symbol}${amount(range.min_minor)}` : range.max_minor != null ? `UP TO ${symbol}${amount(range.max_minor)}` : "Unknown pay";
    return `${text}${unit}${range.location ? ` (${range.location})` : ""}`;
  }).join("; ");
}

export function formatExperience(job: Job): string {
  if (job.experience_min == null && job.experience_max == null) {
    return job.experience_raw || "—";
  }

  if (job.experience_min != null && job.experience_max != null) {
    if (job.experience_min === job.experience_max) {
      return `${job.experience_min} YOE`;
    }
    return `${job.experience_min}-${job.experience_max} YOE`;
  }

  if (job.experience_min != null) {
    return `${job.experience_min}+ YOE`;
  }

  if (job.experience_max != null) {
    return `≤${job.experience_max} YOE`;
  }

  return "—";
}

/**
 * Format work authorization / Green Card eligibility into clean brutalist label.
 */
export function formatWorkAuth(job: Job): string {
  switch (job.work_auth) {
    case "citizen_only":
      return "CITIZEN ONLY";
    case "us_person":
      return "GC / US PERSON";
    case "open":
      return "OPEN";
    case "unknown":
    default:
      return "—";
  }
}

/**
 * Count how many non-default filter dimensions are currently set.
 */
export function countActiveFilters(filter: Partial<JobFilter>): number {
  let count = 0;
  if (filter.search && filter.search.trim()) count++;
  if (filter.favorited_only) count++;
  if (filter.salary_min != null || filter.salary_max != null) count++;
  if (filter.salary_interval) count++;
  if (filter.experience_min != null || filter.experience_max != null) count++;
  if (filter.new_grad && filter.new_grad !== "any") count++;
  if (filter.sponsorship && filter.sponsorship.length > 0) count++;
  if (filter.work_auth && filter.work_auth.length > 0) count++;
  if (filter.clearance && filter.clearance.length > 0) count++;
  if (filter.remote && filter.remote !== "all") count++;
  if (filter.countries && filter.countries.length > 0) count++;
  if (filter.regions && filter.regions.length > 0) count++;
  if (filter.role_types && filter.role_types.length > 0) count++;
  return count;
}
