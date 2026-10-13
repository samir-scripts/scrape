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
