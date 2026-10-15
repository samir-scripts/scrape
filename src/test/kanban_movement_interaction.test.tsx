// @vitest-environment jsdom
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { KanbanBoard } from "../components/views/tracked/KanbanBoard";
import { recordedJobs } from "../lib/recordedJobs";
beforeEach(() => { Object.defineProperty(HTMLElement.prototype, "scrollIntoView", { configurable: true, value: vi.fn() }); });
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const job = { ...recordedJobs()[0], status: "applied" as const, id: "moving" };
it("offers a named stage control without opening details and ignores a same-stage choice", async () => {
 const move = vi.fn(), open = vi.fn();
 render(<KanbanBoard jobs={[job]} selectedJobId={job.id} onSelectJob={open} onDropJob={move} />);
 const control = screen.getByRole("button", { name: `Move ${job.title} to` });
 await userEvent.click(control); await userEvent.click(await screen.findByRole("option", { name: "interviewing" }));
 expect(move).toHaveBeenCalledWith(job.id, "interviewing"); expect(open).not.toHaveBeenCalled();
 await userEvent.click(control); await userEvent.click(await screen.findByRole("option", { name: "applied" })); expect(move).toHaveBeenCalledTimes(1);
 expect(screen.getByRole("button", { name: `Drag ${job.title} to another stage` }).closest("article")?.draggable).toBe(false);
});
it("moves from the focused handle using keyboard and cancels without changing stage", async () => {
 const move = vi.fn(); render(<KanbanBoard jobs={[job]} selectedJobId={null} onSelectJob={vi.fn()} onDropJob={move} />);
 const handle = screen.getByRole("button", { name: `Drag ${job.title} to another stage` });
 handle.focus(); const user = userEvent.setup();
 await user.keyboard(" {ArrowRight}{Escape}"); expect(move).not.toHaveBeenCalled();
 await user.keyboard(" {ArrowRight}{Enter}"); expect(move).toHaveBeenCalledExactlyOnceWith(job.id, "interviewing");
});
it("keeps the card in its saved column after failed IPC and retries through the visible action", async () => {
 const { QueryClient, QueryClientProvider } = await import('@tanstack/react-query');
 const { default: App } = await import('../App');
 const { api, DEFAULT_SETTINGS } = await import('../lib/api');
 const { useUIStore } = await import('../lib/store');
 const { useToastStore } = await import('../lib/toast');
 useUIStore.setState({ view: 'tracked', trackedViewMode: 'kanban', selectedJobId: null, drawerJobId: null }); useUIStore.getState().resetFilterSpec();
 let saved = { ...job };
 vi.spyOn(api, 'getSettings').mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
 vi.spyOn(api, 'listJobs').mockImplementation(async filter => ({ items: [saved], total: 1n, limit: 50n, offset: BigInt(filter?.offset || 0) }));
 vi.spyOn(api, 'updateJobStatus').mockRejectedValueOnce(new Error('fixture disk full')).mockImplementation(async (_, status) => { saved = { ...saved, status } as typeof saved; return saved; });
 const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
 render(<QueryClientProvider client={client}><App /></QueryClientProvider>);
 const user = userEvent.setup(); await user.click(await screen.findByRole('button', { name: `Move ${job.title} to` })); await user.click(await screen.findByRole('option', { name: 'interviewing' }));
 const retry = await screen.findByRole('button', { name: 'Retry stage change' });
 expect(screen.getByRole('region', { name: 'APPLIED stage' }).querySelector('article')).toBeTruthy();
 expect(screen.getByRole('region', { name: 'INTERVIEWING stage' }).querySelector('article')).toBeNull();
 await user.click(retry);
 await screen.findByText('1 loaded', { selector: '[aria-label="INTERVIEWING stage"] header span' });
 expect(saved.status).toBe('interviewing');
 useToastStore.getState().clearAll();
});
