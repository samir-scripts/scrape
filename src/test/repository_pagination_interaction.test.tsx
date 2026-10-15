// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "../App";
import { api, DEFAULT_SETTINGS, generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";
import { useToastStore } from "../lib/toast";
import { useUIStore } from "../lib/store";
import type { Page } from "../bindings/Page";
import type { Job } from "../bindings/Job";

afterEach(() => { cleanup(); vi.restoreAllMocks(); useToastStore.getState().clearAll(); useUIStore.getState().resetFilterSpec(); });
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
    await screen.findByRole("button", { name: /^View Page result/ });
    expect(screen.getByRole("textbox", { name: "Page number" })).toBeTruthy();
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
  it("jumps rapidly across pages, ignores an older page response, and never revives an old page on filter reset", async () => {
    useUIStore.setState({ view: "discovered", selectedJobId: null, drawerJobId: null }); useUIStore.getState().resetFilterSpec();
    const fixture = { ...generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0], status: "viewed" as const };
    vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
    let old: ((value: Page<Job>) => void) | undefined;
    const listing = vi.spyOn(api, "listJobs").mockImplementation(async filter => {
      if (filter?.limit === 1) return { items: [], total: 2105n, limit: 1n, offset: 0n };
      if (filter?.offset === 100) return new Promise(resolve => { old = resolve; });
      return { items: [{ ...fixture, id: String(filter?.offset), title: `Result ${filter?.offset}` }], total: 2105n, limit: 50n, offset: BigInt(filter?.offset || 0) };
    });
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><App /></QueryClientProvider>);
    await screen.findByRole("button", { name: /^View Result 0 / });
    const user = userEvent.setup(), input = screen.getByRole("textbox", { name: "Page number" });
    await user.clear(input); await user.type(input, "3{Enter}"); await waitFor(() => expect(old).toBeDefined());
    await user.clear(input); await user.type(input, "8{Enter}");
    await screen.findByRole("button", { name: /^View Result 350 / });
    old!({ items: [{ ...fixture, title: "Outdated third page" }], total: 2105n, limit: 50n, offset: 100n });
    await waitFor(() => expect(screen.queryByRole("button", { name: /^View Outdated/ })).toBeNull());
    const search = screen.getByRole("textbox", { name: "Search jobs" });
    await user.type(search, "new"); await screen.findByRole("button", { name: /^View Result 0 / });
    await user.clear(search);
    await waitFor(() => expect((input as HTMLInputElement).value).toBe("1"));
    expect(listing.mock.calls.at(-1)?.[0]?.offset).toBe(0);
    const toolbar = screen.getByRole("toolbar", { name: "Job controls" });
    expect(toolbar.querySelector('[aria-label="Sort jobs"]')).toBeNull();
    expect(toolbar.querySelector("button")?.textContent).toContain("FILTERS & SORT");
    expect(Boolean(screen.getByRole("navigation", { name: "Job pagination" }).compareDocumentPosition(toolbar) & Node.DOCUMENT_POSITION_PRECEDING)).toBe(true);
  });
  it("hands a pending note to its original job on page change, and offers retry after persistence failure", async () => {
    useUIStore.setState({ view: "discovered", selectedJobId: null, drawerJobId: null }); useUIStore.getState().resetFilterSpec();
    const fixture = { ...generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0], status: "viewed" as const };
    vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
    vi.spyOn(api, "listJobs").mockImplementation(async filter => ({ items: filter?.limit === 1 ? [] : [{ ...fixture, id: `job-${filter?.offset || 0}` }], total: 51n, limit: BigInt(filter?.limit || 50), offset: BigInt(filter?.offset || 0) }));
    const notes = vi.spyOn(api, "updateJobNotes").mockRejectedValueOnce(new Error("disk full")).mockResolvedValue(fixture);
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><App /></QueryClientProvider>);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: /^View / }));
    fireEvent.change(screen.getByLabelText("Notes"), { target: { value: "Draft before paging" } });
    await user.click(screen.getByRole("button", { name: "Next page" }));
    await waitFor(() => expect(notes).toHaveBeenCalledWith("job-0", "Draft before paging"));
    expect(screen.queryByRole("complementary", { name: "Job details" })).toBeNull();
    await user.click(await screen.findByRole("button", { name: "Retry saving notes" }));
    await waitFor(() => expect(notes).toHaveBeenCalledTimes(2));
    expect(notes).toHaveBeenLastCalledWith("job-0", "Draft before paging");
  });
  it("clamps the last page when a stage change reduces the total", async () => {
    useUIStore.setState({ view: "discovered", selectedJobId: null, drawerJobId: null }); useUIStore.getState().resetFilterSpec();
    const fixture = { ...generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0], status: "viewed" as const };
    let total = 51n;
    vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
    const listing = vi.spyOn(api, "listJobs").mockImplementation(async filter => ({ items: filter?.limit === 1 || (total === 50n && filter?.offset === 50) ? [] : [fixture], total, limit: BigInt(filter?.limit || 50), offset: BigInt(filter?.offset || 0) }));
    vi.spyOn(api, "updateJobStatus").mockImplementation(async () => { total = 50n; return { ...fixture, status: "applied" }; });
    render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><App /></QueryClientProvider>);
    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: "Next page" }));
    await user.click(await screen.findByRole("button", { name: /^View / }));
    await user.click(screen.getByRole("button", { name: "Mark as applied" }));
    await waitFor(() => expect((screen.getByRole("textbox", { name: "Page number" }) as HTMLInputElement).value).toBe("1"));
    expect(listing.mock.calls.some(([filter]) => filter?.offset === 50 && filter.limit === 50)).toBe(true);
    expect(listing.mock.calls.at(-1)?.[0]?.offset).toBe(0);
  });

});
