// @vitest-environment jsdom
// Labelled mocked IPC/UI consumer tests, not native provider acceptance.
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "../App";
import { api, DEFAULT_SETTINGS, generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";
import { useUIStore } from "../lib/store";
import { useToastStore } from "../lib/toast";
import type { ScrapeRun } from "../bindings/ScrapeRun";
import { SourcesSettings } from "../components/views/settings/SourcesSettings";
import { SAMPLE_WORKFLOW_TEMPLATES, workflowSpecToCreateInput } from "../lib/yamlWorkflow";

afterEach(() => {
  cleanup(); vi.restoreAllMocks(); useToastStore.getState().clearAll();
  useUIStore.getState().resetFilterSpec();
  useUIStore.setState({ isScanning: false, runningSourceIds: [], selectedJobId: null, drawerJobId: null });
});

it.each(["success", "no-match", "persistence-failure"] as const)("mocked IPC Load → Run → diagnostics → Discovered: %s", async scenario => {
  const source = { ...SAMPLE_SWE_SOURCE, enabled: true, needs_review: false };
  const existing = { ...generateScrapedJobsForSource(source)[0], id: "retained", title: "Retained engineer", status: "viewed" as const };
  const committed = { ...existing, id: "committed", title: "Committed engineer" };
  let seeded = false, completed = false;
  let resolveCompletion!: (value: ScrapeRun) => void;
  const completionPromise = new Promise<ScrapeRun>(resolve => {
    resolveCompletion = resolve;
  });
  const run: ScrapeRun = {
    id: "authored-ui-run", source_id: source.id, trigger: "single_source",
    status: scenario === "persistence-failure" ? "partial" : "success",
    started_at: "2026-10-19T10:00:00Z", finished_at: "2026-10-19T10:00:01Z",
    jobs_found: 3n, jobs_new: scenario === "no-match" ? 0n : 1n,
    error: scenario === "persistence-failure" ? "SQLite persistence failed: disk full; one posting saved" : null,
    counts: { fetched: 3, rejected: scenario === "no-match" ? 3 : 1, blocked: 0,
      inserted: scenario === "no-match" ? 0 : 1, updated: scenario === "success" ? 1 : 0,
      failed: scenario === "persistence-failure" ? 1 : 0, retention_removed: 0,
      sources: [{ source_id: source.id, limit: 20, examined: 3, matched: scenario === "no-match" ? 0 : 2,
        accepted: scenario === "no-match" ? 0 : 2, blocked: 0, rejected: scenario === "no-match" ? 3 : 1,
        duplicates: 0, invalid: 0, unresolved: 0, limit_reached: false, provider_total: null,
        enrichment_warnings: 0, termination: "exhausted", outcome: "success", rejections: [], rejection_examples: [] }],
    },
  };
  vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
  vi.spyOn(api, "listSources").mockImplementation(async () => seeded ? [source] : []);
  const seed = vi.spyOn(api, "seedAllSampleWorkflows").mockImplementation(async () => {
    seeded = true; return { created: [source], already_present: [], needs_review: [] };
  });
  vi.spyOn(api, "listSourceRuns").mockImplementation(async () => completed ? [run] : []);
  vi.spyOn(api, "getScrapeLogs").mockResolvedValue([]);
  const start = vi.spyOn(api, "runSourceNow").mockImplementation(() => completionPromise);
  const listing = vi.spyOn(api, "listJobs").mockImplementation(async filter => {
    const items = completed && scenario !== "no-match" ? [existing, committed] : [existing];
    return { items: filter?.limit === 1 ? [] : items, total: BigInt(items.length), limit: BigInt(filter?.limit || 50), offset: 0n };
  });
  useUIStore.setState({ view: "discovered", selectedJobId: null, drawerJobId: null });
  useUIStore.getState().resetFilterSpec();
  useUIStore.getState().setFilterSpec(previous => ({ ...previous, search: "engineer", statuses: ["viewed"] }));
  const filters = structuredClone(useUIStore.getState().filterSpec);
  const client = new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } });
  render(<QueryClientProvider client={client}><App /></QueryClientProvider>);
  const user = userEvent.setup();
  await screen.findByRole("button", { name: /^View Retained engineer / });
  await user.click(screen.getByRole("button", { name: "SCRAPER" }));
  await user.click(await screen.findByRole("button", { name: "Load test workflows" }));
  expect(seed).toHaveBeenCalledTimes(1);
  const row = await screen.findByRole("button", { name: `Edit workflow ${source.name}` });
  await user.click(within(row.closest("tr")!).getByRole("button", { name: "RUN" }));
  await waitFor(() => expect(start).toHaveBeenCalledExactlyOnceWith(source.id));
  expect(screen.getByText(/WORKFLOW OBSERVABILITY & EXECUTION LOGS/)).toBeTruthy();
  expect(screen.queryByRole("button", { name: "RUN" })).toBeNull();
  completed = true; resolveCompletion(run);
  await screen.findByText(new RegExp(`3 examined · ${scenario === "no-match" ? 0 : 2} accepted`));
  if (scenario === "persistence-failure") {
    expect(await screen.findByText(/DETAIL: SQLite persistence failed: disk full/)).toBeTruthy();
    expect(useToastStore.getState().toasts.some(toast => toast.message.includes("disk full"))).toBe(true);
  }
  await user.click(screen.getByRole("button", { name: /^DISCOVERED/ }));
  await screen.findByRole("button", { name: /^View Retained engineer / });
  if (scenario === "no-match") expect(screen.queryByRole("button", { name: /^View Committed engineer / })).toBeNull();
  else await screen.findByRole("button", { name: /^View Committed engineer / });
  expect(useUIStore.getState().filterSpec).toEqual(filters);
  expect(listing.mock.calls.filter(([filter]) => filter?.limit !== 1).length).toBeGreaterThan(1);
  expect(screen.getByRole("textbox", { name: "Search jobs" })).toHaveProperty("value", "engineer");
  client.clear();
});

it("keeps executable workflow Run disabled when opening its history before setup/trust review", async () => {
  const input = workflowSpecToCreateInput(SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === "streaming-cloud-script")!.spec);
  const source = { ...SAMPLE_SWE_SOURCE, kind: input.kind, identifier: input.identifier, config: input.config, filters: input.filters, name: "Authored untrusted executable", enabled: false, needs_review: true, script_trusted: false };
  vi.spyOn(api, "listSources").mockResolvedValue([source]);
  vi.spyOn(api, "listSourceRuns").mockResolvedValue([]);
  const run = vi.spyOn(api, "runSourceNow");
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><SourcesSettings /></QueryClientProvider>);
  const user = userEvent.setup();
  const editor = await screen.findByRole("button", { name: `Edit workflow ${source.name}` });
  const row = within(editor.closest("tr")!);
  expect(row.getByText("NEEDS REVIEW")).toBeTruthy();
  expect(row.getByRole("button", { name: "RUN" })).toHaveProperty("disabled", true);
  await user.click(row.getByRole("button", { name: `More actions for ${source.name}` }));
  await user.click(screen.getByRole("menuitem", { name: "View run history" }));
  const dockRun = screen.getByRole("button", { name: "RUN WORKFLOW NOW" });
  expect(dockRun).toHaveProperty("disabled", true);
  await user.click(dockRun);
  expect(run).not.toHaveBeenCalled();
  client.clear();
});

it("keeps Run disabled and shows warning banner when workflow has all child sources disabled", async () => {
  const source = {
    ...SAMPLE_SWE_SOURCE,
    id: "google-apple-disabled",
    name: "Google & Apple — Static extraction example (Needs review)",
    enabled: true,
    needs_review: false,
    config: {
      __workflow_spec: {
        sources: [
          { id: "google", label: "Google", enabled: false },
          { id: "apple", label: "Apple", enabled: false },
        ],
      },
    },
  };
  vi.spyOn(api, "listSources").mockResolvedValue([source]);
  vi.spyOn(api, "listSourceRuns").mockResolvedValue([]);
  const run = vi.spyOn(api, "runSourceNow");
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  render(<QueryClientProvider client={client}><SourcesSettings /></QueryClientProvider>);
  const user = userEvent.setup();
  const editor = await screen.findByRole("button", { name: `Edit workflow ${source.name}` });
  const row = within(editor.closest("tr")!);
  expect(row.getByRole("button", { name: "RUN" })).toHaveProperty("disabled", true);
  await user.click(row.getByRole("button", { name: `More actions for ${source.name}` }));
  await user.click(screen.getByRole("menuitem", { name: "View run history" }));
  const dockRun = screen.getByRole("button", { name: "RUN WORKFLOW NOW" });
  expect(dockRun).toHaveProperty("disabled", true);
  expect(screen.getByRole("alert").textContent).toMatch(/Workflow or all sources disabled/i);
  await user.click(dockRun);
  expect(run).not.toHaveBeenCalled();
  client.clear();
});
