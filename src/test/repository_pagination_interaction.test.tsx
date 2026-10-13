// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "../App";
import { api, DEFAULT_SETTINGS, generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";
import { useUIStore } from "../lib/store";
import type { Page } from "../bindings/Page";
import type { Job } from "../bindings/Job";

afterEach(() => { cleanup(); vi.restoreAllMocks(); useUIStore.getState().resetFilterSpec(); });
describe("repository pages and response ordering", () => {
  it("resets the page on a new search and prevents an older response from replacing newer results", async () => {
    useUIStore.getState().setView("discovered"); useUIStore.getState().resetFilterSpec();
    const fixture = { ...generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0], status: "viewed" as const };
    vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
    let resolveOld: ((page: Page<Job>) => void) | undefined;
    const listing = vi.spyOn(api, "listJobs").mockImplementation(async (filter) => {
      if (filter?.limit === 1) return { items: [], total: 2105n, limit: 1n, offset: 0n };
      if (filter?.search === "old") return new Promise<Page<Job>>((resolve) => { resolveOld = resolve; });
      return { items: [{ ...fixture, id: filter?.search === "new" ? "new" : "page", title: filter?.search === "new" ? "Newest result" : "Page result" }], total: 2105n, limit: 50n, offset: BigInt(filter?.offset || 0) };
    });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(<QueryClientProvider client={client}><App /></QueryClientProvider>);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Next page" }));
    await waitFor(() => expect(listing.mock.calls.some(([filter]) => filter?.offset === 50 && filter.limit === 50)).toBe(true));
    const search = screen.getByPlaceholderText("SEARCH (TITLE, CO, LOC)...");
    await user.type(search, "old");
    await waitFor(() => expect(resolveOld).toBeDefined());
    await user.clear(search); await user.type(search, "new");
    expect(await screen.findByRole("button", { name: /^View Newest result/ })).toBeTruthy();
    expect(listing.mock.calls.some(([filter]) => filter?.search === "new" && filter.offset === 0)).toBe(true);
    resolveOld!({ items: [{ ...fixture, id: "old", title: "Outdated result" }], total: 2105n, limit: 50n, offset: 0n });
    await waitFor(() => expect(screen.queryByRole("button", { name: /^View Outdated result/ })).toBeNull());
    expect(screen.getByRole("button", { name: /^View Newest result/ })).toBeTruthy();
  });
});
