import type { Job } from "../bindings/Job";
import type { JobFilter } from "../bindings/JobFilter";
import type { TriState } from "../bindings/TriState";
import { toHourlyRate } from "./salary";

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

  // 5. Salary range overlap
  const hasSalaryFilter =
    filter.salary_min != null || filter.salary_max != null;
  if (hasSalaryFilter) {
    const minBound = filter.salary_min ?? 0;
    const maxBound = filter.salary_max ?? Number.POSITIVE_INFINITY;

    if (job.salary_min == null && job.salary_max == null) {
      if (!filter.salary_include_unknown) {
        return false;
      }
    } else {
      const jMin = job.salary_min ?? 0;
      const jMax = job.salary_max ?? Number.POSITIVE_INFINITY;
      if (!(jMax >= minBound && jMin <= maxBound)) {
        return false;
      }
    }
  }

  // 6. Salary currency
  if (filter.salary_currency && filter.salary_currency !== "ALL") {
    if (
      job.salary_currency &&
      job.salary_currency.toUpperCase() !== filter.salary_currency.toUpperCase()
    ) {
      return false;
    }
  }

  // 6b. Salary interval (e.g. "hour" or "year")
  if (filter.salary_interval) {
    if (job.salary_interval !== filter.salary_interval) {
      return false;
    }
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
    if (filter.new_grad === "only" && job.is_new_grad !== "yes") {
      return false;
    }
    if (filter.new_grad === "exclude" && job.is_new_grad === "yes") {
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

  // 11. Workplace type / Remote
  if (filter.remote && filter.remote !== "all") {
    if (job.workplace_type !== filter.remote) {
      return false;
    }
  }

  // 12. Countries
  if (filter.countries && filter.countries.length > 0) {
    const hasLocations = job.locations && job.locations.length > 0;
    if (!hasLocations) {
      if (!filter.location_include_unknown) return false;
    } else {
      const match = job.locations.some((loc) => {
        if (!loc.country_code) return false;
        return filter.countries!.some(
          (c) => c.toLowerCase() === loc.country_code?.toLowerCase()
        );
      });
      if (!match) return false;
    }
  }

  // 13. Regions (e.g. US states / CA provinces)
  if (filter.regions && filter.regions.length > 0) {
    const hasLocations = job.locations && job.locations.length > 0;
    if (!hasLocations) {
      if (!filter.location_include_unknown) return false;
    } else {
      const match = job.locations.some((loc) => {
        if (!loc.region_code) return false;
        const regLow = loc.region_code.toLowerCase();
        return filter.regions!.some((r) => {
          const rLow = r.toLowerCase();
          return regLow === rLow || regLow.endsWith(`-${rLow}`) || rLow.endsWith(`-${regLow}`);
        });
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
export function formatSalary(job: Job): string {
  if (job.salary_min == null && job.salary_max == null) {
    return job.salary_raw || "—";
  }

  const curr = job.salary_currency === "USD" || !job.salary_currency ? "$" : `${job.salary_currency} `;

  // If job is specifically hourly, format as $/HR
  if (job.salary_interval === "hour") {
    const minHr = toHourlyRate(job.salary_min);
    const maxHr = toHourlyRate(job.salary_max);

    if (minHr != null && maxHr != null) {
      if (minHr === maxHr) return `${curr}${minHr}/HR`;
      return `${curr}${minHr} - ${curr}${maxHr}/HR`;
    }
    if (minHr != null) return `FROM ${curr}${minHr}/HR`;
    if (maxHr != null) return `UP TO ${curr}${maxHr}/HR`;
    return job.salary_raw || "—";
  }

  const fmtNum = (n: number) => {
    if (n >= 1000) {
      const k = n / 1000;
      return `${Number.isInteger(k) ? k : k.toFixed(1)}K`;
    }
    return n.toLocaleString();
  };

  if (job.salary_min != null && job.salary_max != null) {
    if (job.salary_min === job.salary_max) {
      return `${curr}${fmtNum(job.salary_min)}`;
    }
    return `${curr}${fmtNum(job.salary_min)} - ${curr}${fmtNum(job.salary_max)}`;
  }

  if (job.salary_min != null) {
    return `FROM ${curr}${fmtNum(job.salary_min)}`;
  }

  if (job.salary_max != null) {
    return `UP TO ${curr}${fmtNum(job.salary_max)}`;
  }

  return "—";
}

/**
 * Format years of experience into compact string.
 */
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
