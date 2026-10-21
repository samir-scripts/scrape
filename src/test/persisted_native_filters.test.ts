import { afterEach, expect, it, vi } from "vitest";
afterEach(() => { vi.unstubAllGlobals(); vi.resetModules(); });
it("persists native hourly cents and restores them without conversion after reload", async () => {
  const values = new Map<string, string>();
  vi.stubGlobal("window", { localStorage: { getItem: (key: string) => values.get(key) || null, setItem: (key: string, value: string) => values.set(key, value) } });
  vi.resetModules();
  const first = (await import("../lib/store")).useUIStore;
  first.getState().setFilterSpec({ salary_min: 50.25, salary_max: 60.75, salary_interval: "hour", salary_currency: "USD" });
  vi.resetModules();
  const restored = (await import("../lib/store")).useUIStore;
  expect(restored.getState().filterSpec.salary_min).toBe(50.25);
  expect(restored.getState().filterSpec.salary_interval).toBe("hour");
  restored.getState().resetFilterSpec();
  expect(JSON.parse(values.get("scrape:job_filters:v2")!).salary_interval).toBeNull();
});

it("restores legacy active experience rules as overlap without changing saved unknown choices", async () => {
  // Reload the store module to exercise its localStorage initialization boundary.
  const values = new Map<string, string>([["scrape:job_filters:v2", JSON.stringify({
    experience_min: 1, experience_max: 5, experience_include_unknown: true, new_grad: "only",
  })]]);
  vi.stubGlobal("window", { localStorage: {
    getItem: (key: string) => values.get(key) || null,
    setItem: (key: string, value: string) => values.set(key, value),
  } });
  vi.resetModules();
  const first = (await import("../lib/store")).useUIStore;
  expect(first.getState().filterSpec).toMatchObject({
    experience_min: 1, experience_max: 5, experience_match_mode: "advertised_range_overlap",
    experience_include_unknown: true, new_grad: "only",
  });
  first.getState().setFilterSpec({ experience_min: 3 });
  vi.resetModules();
  const reopened = (await import("../lib/store")).useUIStore;
  expect(reopened.getState().filterSpec).toMatchObject({
    experience_min: 3, experience_max: 5, experience_match_mode: "advertised_range_overlap",
    experience_include_unknown: true, new_grad: "only",
  });
  reopened.getState().setFilterSpec({ experience_match_mode: "required_minimum" });
  vi.resetModules();
  const converted = (await import("../lib/store")).useUIStore;
  expect(converted.getState().filterSpec).toMatchObject({
    experience_match_mode: "required_minimum", experience_include_unknown: true,
  });
  converted.getState().clearFilters();
  expect(converted.getState().filterSpec).toMatchObject({
    experience_min: null, experience_max: null, experience_match_mode: "required_minimum",
    experience_include_unknown: false,
  });
});
