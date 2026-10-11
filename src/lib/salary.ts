/**
 * Centralized salary & compensation conversion utilities.
 * Defines standard work hours and conversion math between hourly rates and annual salaries.
 */

export const ANNUAL_WORK_HOURS = 2080; // 40 hrs/week * 52 weeks
export const HOURLY_SALARY_THRESHOLD = 1000;
export const MAX_ANNUAL_SALARY = 500000;
export const STEP_ANNUAL_SALARY = 5000;
export const MAX_HOURLY_RATE = 250;
export const STEP_HOURLY_RATE = 5;

/**
 * Normalizes a given salary number into an hourly rate.
 * If the value is already an hourly rate (< 1000), returns it as-is.
 * Otherwise divides by 2080 annual work hours and rounds.
 */
export function toHourlyRate(salary: number | null | undefined): number | null {
  if (salary == null) return null;
  return salary < HOURLY_SALARY_THRESHOLD
    ? salary
    : Math.round(salary / ANNUAL_WORK_HOURS);
}

/**
 * Normalizes a given compensation number into an annualized salary.
 * If the value is an hourly rate (< 1000), multiplies by 2080 hours.
 * Otherwise returns the annualized salary as-is.
 */
export function toAnnualSalary(salary: number | null | undefined): number | null {
  if (salary == null) return null;
  return salary < HOURLY_SALARY_THRESHOLD
    ? Math.round(salary * ANNUAL_WORK_HOURS)
    : Math.round(salary);
}

/**
 * Parses user input in salary fields, handling abbreviations like "$150k" or raw numbers.
 */
export function parseSalaryInput(raw: string): number | null {
  const trimmed = raw.trim().toLowerCase().replace(/[$,]/g, "");
  if (!trimmed) return null;
  if (trimmed.endsWith("k")) {
    const num = parseFloat(trimmed.slice(0, -1));
    return isNaN(num) ? null : Math.round(num * 1000);
  }
  const num = parseFloat(trimmed);
  if (isNaN(num)) return null;
  // If user typed e.g. 150, interpret as 150k if <= 500
  if (num > 0 && num <= 500) {
    return Math.round(num * 1000);
  }
  return Math.round(num);
}

/**
 * Parses user input in hourly rate fields.
 */
export function parseHourlyInput(raw: string): number | null {
  const trimmed = raw.trim().toLowerCase().replace(/[$,/hr]/g, "");
  if (!trimmed) return null;
  const num = parseFloat(trimmed);
  if (isNaN(num) || num < 0) return null;
  return Math.round(num);
}

/**
 * Formats a salary or hourly rate for display with commas.
 */
export function formatSalaryNumber(num: number | null | undefined): string {
  if (num == null) return "";
  return num.toLocaleString();
}
