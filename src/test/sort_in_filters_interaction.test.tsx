// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "../App";
import { api, DEFAULT_SETTINGS, generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";
import { countActiveFilters } from "../lib/filter";
import { useUIStore } from "../lib/store";

afterEach(() => { cleanup(); vi.restoreAllMocks(); useUIStore.getState().resetFilterSpec(); useUIStore.getState().setFilterPanelOpen(false); });

it("sorts inside Filters & sort, resets pagination, and keeps sort when filters clear", async () => {
 useUIStore.setState({ view: "discovered", selectedJobId: null, drawerJobId: null });
 useUIStore.getState().resetFilterSpec(); useUIStore.getState().setFilterSpec({ search: "fixture", sort: null });
 const fixture = { ...generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0], status: "viewed" as const };
 vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
 const listing = vi.spyOn(api, "listJobs").mockImplementation(async filter => {
  if (filter?.limit === 1) return { items: [], total: 120n, limit: 1n, offset: 0n };
  return { items: [{ ...fixture, id: `row-${filter?.offset}`, title: `Fixture ${filter?.offset}` }], total: 120n, limit: 50n, offset: BigInt(filter?.offset || 0) };
 });
 render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><App /></QueryClientProvider>);
 const user = userEvent.setup();
 await screen.findByRole("button", { name: /^View Fixture 0 / });
 const toolbar = screen.getByRole("toolbar", { name: "Job controls" });
 const entry = screen.getByRole("button", { name: /^FILTERS & SORT \(1\)/ });
 expect(toolbar.querySelector('[aria-label="Sort jobs"]')).toBeNull();
 expect(countActiveFilters({ search: "fixture", sort: "title_asc" })).toBe(1);
 await user.click(await screen.findByRole("button", { name: "Next page" }));
 await screen.findByRole("button", { name: /^View Fixture 50 / });
 await user.click(entry);
 screen.getByRole("dialog", { name: "Filters and sort" });
 await user.click(screen.getByRole("button", { name: "Sort jobs" }));
 await user.click(await screen.findByRole("option", { name: "Title A to Z" }));
 const pageInput = screen.getByRole("textbox", { name: "Page number" }) as HTMLInputElement;
 await waitFor(() => expect(pageInput.value).toBe("1"));
 expect(listing.mock.calls.at(-1)?.[0]).toMatchObject({ sort: "title_asc", offset: 0 });
 await user.click(screen.getAllByRole("button", { name: /CLEAR FILTERS/ })[0]);
 expect(useUIStore.getState().filterSpec.search).toBeNull();
 expect(useUIStore.getState().filterSpec.sort).toBe("title_asc");
 expect(screen.getByRole("button", { name: /^FILTERS & SORT/ }).textContent).not.toContain("(1)");
 expect(screen.getByText("SORT: Title A to Z")).toBeTruthy();
 await user.click(screen.getByRole("button", { name: "Reset sort" }));
 expect(useUIStore.getState().filterSpec.sort).toBeNull();
 await waitFor(() => expect(listing.mock.calls.at(-1)?.[0]?.sort).toBe("date_added_desc"));
 expect(screen.queryByText("SORT: Title A to Z")).toBeNull();
});
