/**
 * Centralized salary & compensation conversion utilities.
 * Defines standard work hours and conversion math between hourly rates and annual salaries.
 */

export const ANNUAL_WORK_HOURS = 2080; // 40 hrs/week * 52 weeks
export const MAX_ANNUAL_SALARY = 500000;
export const STEP_ANNUAL_SALARY = 5000;
export const MAX_HOURLY_RATE = 250;
export const STEP_HOURLY_RATE = 5;

/** Explicit analytics conversions; callers provide the source unit. */
export function toHourlyRate(annual: number | null | undefined): number | null {
  return annual == null ? null : annual / ANNUAL_WORK_HOURS;
}
export function toAnnualSalary(hourly: number | null | undefined): number | null {
  return hourly == null ? null : hourly * ANNUAL_WORK_HOURS;
}
export function parseSalaryInput(raw: string): number | null {
  const normalized = raw.trim().replace(/^\$\s*/, "").replaceAll(",", "");
  const match = /^(\d+(?:\.\d{1,2})?)\s*([kK])?$/.exec(normalized);
  if (!match) return null;
  const result = Number(match[1]) * (match[2] ? 1000 : 1);
  return Number.isFinite(result) && result >= 0 ? Math.round(result * 100) / 100 : null;
}
export function parseHourlyInput(raw: string): number | null {
  const normalized = raw.trim().replace(/^\$\s*/, "").replace(/\s*\/\s*(?:hr|hour)$/i, "");
  if (!/^\d+(?:\.\d{1,2})?$/.test(normalized)) return null;
  const result = Number(normalized);
  return Number.isFinite(result) && result >= 0 ? result : null;
}

/**
 * Formats a salary or hourly rate for display with commas.
 */
export function formatSalaryNumber(num: number | null | undefined): string {
  if (num == null) return "";
  return num.toLocaleString();
}
