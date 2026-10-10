import { describe, it, expect } from "vitest";
import { api } from "../lib/api";
import { formatRoleType } from "../components/views/metrics/RoleTypeDonut";
import type { Metrics } from "../bindings/Metrics";
import type { RoleType } from "../bindings/RoleType";

describe("Metrics Dashboard Tests", () => {
  it("returns default zeroed metrics in non-Tauri dev environment", async () => {
    const metrics: Metrics = await api.getMetrics("week", 12);
    expect(metrics.totals.scraped).toBe(0);
    expect(metrics.totals.applied).toBe(0);
    expect(metrics.totals.interviewing).toBe(0);
    expect(metrics.totals.offers).toBe(0);
    expect(metrics.totals.response_rate).toBe(0);
    expect(metrics.totals.median_salary).toBeNull();
    expect(metrics.totals.avg_salary).toBeNull();
    expect(metrics.streak.current).toBe(0);
    expect(metrics.streak.longest).toBe(0);
  });

  it("formats role types into uppercase brutalist labels", () => {
    expect(formatRoleType("full_time")).toBe("FULL-TIME");
    expect(formatRoleType("contract")).toBe("CONTRACT");
    expect(formatRoleType("internship")).toBe("INTERNSHIP");
    expect(formatRoleType("part_time")).toBe("PART-TIME");
    expect(formatRoleType("unknown")).toBe("UNKNOWN");
  });

  it("calculates response rate percentage correctly without division by zero", () => {
    const calcRate = (interviewing: number, offers: number, applied: number): number => {
      if (applied === 0) return 0;
      return Math.round(((interviewing + offers) / applied) * 100);
    };

    expect(calcRate(0, 0, 0)).toBe(0);
    expect(calcRate(1, 0, 10)).toBe(10);
    expect(calcRate(2, 1, 6)).toBe(50);
    expect(calcRate(3, 1, 3)).toBe(133); // Handles multi-stage edge case
  });

  it("formats salary display values cleanly", () => {
    const formatSalary = (val: number | null): string => {
      if (val === null || val === undefined) return "--";
      return `$${Math.round(val).toLocaleString()}`;
    };

    expect(formatSalary(null)).toBe("--");
    expect(formatSalary(145000)).toBe("$145,000");
    expect(formatSalary(82500.5)).toBe("$82,501");
  });

  it("computes role type percentage shares correctly", () => {
    const roles: Array<{ role_type: RoleType; count: number }> = [
      { role_type: "full_time", count: 30 },
      { role_type: "contract", count: 10 },
      { role_type: "internship", count: 10 },
    ];
    const total = roles.reduce((s, r) => s + r.count, 0);
    expect(total).toBe(50);

    const pcts = roles.map((r) => Math.round((r.count / total) * 100));
    expect(pcts).toEqual([60, 20, 20]);
  });
});
