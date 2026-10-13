import { afterEach, describe, expect, it, vi } from "vitest";
import { api } from "../lib/api";
import { isDemoMode, needsDesktop } from "../lib/runtime";

afterEach(() => { vi.unstubAllEnvs(); vi.unstubAllGlobals(); });

describe("explicit browser demonstration", () => {
  it("rejects browser data operations unless the user opts into demonstration", async () => {
    vi.stubEnv("MODE", "production");
    vi.stubGlobal("window", { location: { search: "" } });
    expect(needsDesktop()).toBe(true);
    expect(isDemoMode()).toBe(false);
    await expect(api.listJobs()).rejects.toThrow("requires the desktop application");
    await expect(api.runScrapeNow()).rejects.toThrow("requires the desktop application");
  });
  it("permits explicit demo operations and keeps the mode identifiable", async () => {
    vi.stubEnv("MODE", "production");
    vi.stubGlobal("window", { location: { search: "?demo=1" } });
    expect(isDemoMode()).toBe(true);
    expect(needsDesktop()).toBe(false);
    await expect(api.listJobs()).resolves.toHaveProperty("items");
  });
});
