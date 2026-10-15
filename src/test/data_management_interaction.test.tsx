// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider, QueryObserver } from "@tanstack/react-query";
const native = vi.hoisted(() => ({ enabled: true, invoke: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ isTauri: () => native.enabled, invoke: native.invoke }));
import { DataManagementSettings } from "../components/views/settings/DataManagementSettings";
import { api } from "../lib/api";
import { applyDataCleanup } from "../lib/dataCleanup";
import { DEFAULT_FILTER_SPEC, useUIStore } from "../lib/store";
import type { CleanupMode } from "../bindings/CleanupMode";
import type { CleanupPreview } from "../bindings/CleanupPreview";
import type { CleanupResult } from "../bindings/CleanupResult";

beforeAll(() => {
  const storage = Object.create({
    getItem(this: Record<string, string>, key: string) { return Object.hasOwn(this, key) ? this[key] : null; },
    setItem(this: Record<string, string>, key: string, value: string) { this[key] = String(value); },
    removeItem(this: Record<string, string>, key: string) { delete this[key]; },
    clear(this: Record<string, string>) { for (const key of Object.keys(this)) delete this[key]; },
  });
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: storage });
  Object.defineProperty(window, "localStorage", { configurable: true, value: storage });
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); native.enabled = true; localStorage.clear(); });
const counts = (mode: CleanupMode): CleanupPreview => ({ mode, revision: "revision", deleted_records: 7, deleted_postings: 5, preserved_records: 2, preserved_postings: 1, deleted_notes: 3, deleted_history: 8, workflows: 2, blocks: 4 });
const result = (mode: CleanupMode): CleanupResult => ({ counts: counts(mode), backup_path: `/temporary/${mode}.db`, scheduling_paused: true, warnings: [] });
function renderSettings() {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false }, mutations: { retry: false } } });
  render(<QueryClientProvider client={client}><DataManagementSettings /></QueryClientProvider>);
  return client;
}
function previewStub() { return vi.spyOn(api, "previewDataCleanup").mockImplementation(mode => Promise.resolve(counts(mode))); }
async function open(user: ReturnType<typeof userEvent.setup>, name = "Review all jobs") {
  await user.click(screen.getByRole("button", { name }));
  await screen.findByLabelText("Cleanup counts");
  return screen.getByRole("dialog");
}
describe("confirmed bulk cleanup", () => {
  it("requires desktop and never fabricates a successful browser cleanup", async () => {
    native.enabled = false; renderSettings();
    expect(screen.getByRole("status").textContent).toContain("cannot clear the desktop database");
    expect((screen.getByRole("button", { name: "Review all jobs" }) as HTMLButtonElement).disabled).toBe(true);
    await expect(api.executeDataCleanup({ preview: counts("all_jobs"), confirmation: "", pause_after: true })).rejects.toThrow("desktop app only");
    expect(native.invoke).not.toHaveBeenCalled();
  });
  it("focuses Cancel, restores focus, and does nothing on Enter or Escape", async () => {
    previewStub(); const execute = vi.spyOn(api, "executeDataCleanup");renderSettings();const user=userEvent.setup();
    await open(user);
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Cancel" }));
    expect((screen.getByRole("checkbox") as HTMLInputElement).checked).toBe(true);
    expect(screen.getByText("Delete 5 postings (7 saved records).")).toBeTruthy();
    await user.keyboard("{Enter}");
    expect(document.activeElement).toBe(screen.getByRole("button", { name: "Review all jobs" }));
    await open(user); await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).toBeNull();expect(execute).not.toHaveBeenCalled();
  });
  it.each([['Review all jobs','all_jobs'],['Review keep favorites','except_favorites'],['Review keep tracked','except_tracked']] as const)("executes %s using reviewed counts and chosen pause", async (name,mode) => {
    previewStub(); const execute=vi.spyOn(api,"executeDataCleanup").mockResolvedValue(result(mode));
    const client=renderSettings();client.setQueryData(['jobs'], { items: ['stale'] });client.setQueryData(['metrics'], { total: 100 });
    useUIStore.setState({selectedJobId:'old',drawerJobId:'old',selectedJobIds:['old'],undoStack:[{jobId:'old',previousStatus:'viewed'}]});
    const user=userEvent.setup(); await open(user,name);await user.click(screen.getByRole('checkbox'));await user.click(screen.getByRole('button',{name:'Permanently delete'}));
    await screen.findByText(/Recovery backup:/);
    expect(execute).toHaveBeenCalledExactlyOnceWith({preview: counts(mode),pause_after:false,confirmation:''});
    expect(client.getQueryData(['jobs'])).toBeUndefined();expect(client.getQueryData(['metrics'])).toBeUndefined();
    expect(useUIStore.getState().selectedJobId).toBeNull();expect(useUIStore.getState().undoStack).toEqual([]);
  });
  it("requires exact RESET, disables duplicate submissions, and clears scoped preferences", async () => {
    previewStub();let resolve!: (data: CleanupResult)=>void;
    const execute=vi.spyOn(api,'executeDataCleanup').mockImplementation(()=>new Promise(done=>{resolve=done;}));
    renderSettings();localStorage.setItem('scrape:layout:discovered_split_width','42');localStorage.setItem('unrelated','keep');
    useUIStore.setState({filterSpec:{search:'old'},scraperMode:'advanced'});
    const user=userEvent.setup(); await open(user,'Review reset');const submit=screen.getByRole('button',{name:'Permanently delete'}) as HTMLButtonElement;
    expect(submit.disabled).toBe(true); await user.type(screen.getByLabelText('Type RESET to confirm'),'reset');expect(submit.disabled).toBe(true);
    await user.clear(screen.getByLabelText('Type RESET to confirm'));await user.type(screen.getByLabelText('Type RESET to confirm'),'RESET');expect(submit.disabled).toBe(false);
    await user.click(submit);expect((screen.getByRole('button',{name:'Cancel'}) as HTMLButtonElement).disabled).toBe(true);
    await user.keyboard('{Escape}'); expect(screen.getByRole('dialog')).toBeTruthy();
    resolve(result('reset_all'));await screen.findByText(/Recovery backup:/);
    expect(execute).toHaveBeenCalledTimes(1);expect(localStorage.getItem('scrape:layout:discovered_split_width')).toBeNull();expect(localStorage.getItem('unrelated')).toBe('keep');
    expect(useUIStore.getState().filterSpec).toEqual(DEFAULT_FILTER_SPEC);expect(useUIStore.getState().scraperMode).toBe('simple');
  });
  it("shows backup/persistence failures without deleting caches and requires renewed review", async () => {
    const preview=previewStub();vi.spyOn(api,'executeDataCleanup').mockRejectedValue(new Error('Recovery backup failed. No data was deleted: disk is full'));
    const client=renderSettings();client.setQueryData(['jobs'],{items:['keep']});const user=userEvent.setup();await open(user);
    await user.click(screen.getByRole('button',{name:'Permanently delete'}));await screen.findByRole('alert');
    expect(client.getQueryData(['jobs'])).toEqual({items:['keep']});expect((screen.getByRole('button',{name:'Permanently delete'}) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole('button',{name:'Refresh cleanup counts'}));await screen.findByLabelText('Cleanup counts');expect(preview).toHaveBeenCalledTimes(2);
    expect((screen.getByRole('button',{name:'Permanently delete'}) as HTMLButtonElement).disabled).toBe(false);
  });
  it("refreshes settings without clearing them and losing the recovery receipt", async () => {
    const client = renderSettings();
    client.setQueryData(["settings"], { scraper_enabled: true });
    const observer = new QueryObserver(client, { queryKey: ["settings"], queryFn: async () => ({ scraper_enabled: false }) });
    const seen: unknown[] = [];
    const unsubscribe = observer.subscribe(state => seen.push(state.data));
    await applyDataCleanup(client, result("all_jobs"));
    expect(seen.length).toBeGreaterThan(0);
    expect(seen.every(value => value !== undefined)).toBe(true);
    expect(client.getQueryData(["settings"])).toEqual({ scraper_enabled: false });
    unsubscribe();
  });
  it("ignores a cancelled preview response and deduplicates event/response invalidation", async () => {
    let resolve!: (data: CleanupPreview)=>void;vi.spyOn(api,'previewDataCleanup').mockImplementation(()=>new Promise(done=>{resolve=done;}));
    const client=renderSettings();const user=userEvent.setup();await user.click(screen.getByRole('button',{name:'Review all jobs'}));await user.click(screen.getByRole('button',{name:'Cancel'}));
    resolve(counts('all_jobs'));await waitFor(()=>expect(screen.queryByRole('dialog')).toBeNull());
    const revision=useUIStore.getState().dataRevision;
    await applyDataCleanup(client,result('all_jobs'));await applyDataCleanup(client,result('all_jobs'));
    expect(useUIStore.getState().dataRevision).toBe(revision+1);
  });
});
