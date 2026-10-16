// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { SourcesSettings } from "../components/views/settings/SourcesSettings";
import { ToastContainer } from "../components/ui/Toast";
import { api, SAMPLE_SWE_SOURCE, DEFAULT_SETTINGS } from "../lib/api";
import { workflowSpecToYaml, sourceToWorkflowSpec } from "../lib/yamlWorkflow";
import { useToastStore } from "../lib/toast";
const spec = { ...sourceToWorkflowSpec(SAMPLE_SWE_SOURCE), metadata: { name: "Imported board", enabled: true }, filters: { include_keywords: [], exclude_keywords: [], locations: [] }, schedule: { cadence: "manual" as const, timezone: "UTC" } };
function file(text: string, name = 'workflow.yaml') { const result = new File([text], name, { type: 'text/yaml' }); Object.defineProperty(result, 'text', { value: async () => text }); return result; }
beforeAll(() => { HTMLDialogElement.prototype.showModal = function() { this.setAttribute('open',''); }; HTMLDialogElement.prototype.close = function() { this.removeAttribute('open'); }; });
afterEach(() => { cleanup(); vi.restoreAllMocks(); useToastStore.getState().clearAll(); });
it('imports only after validation, confirms replacement, preserves the old draft on failure, and reselects the same file', async () => {
 const save = vi.spyOn(api, 'saveWorkflow');
 render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
 const user = userEvent.setup(); await user.type(screen.getByLabelText('Workflow name'), 'Keep my changes');
 const input = screen.getByLabelText('Workflow YAML file');
 const yaml = file(workflowSpecToYaml(spec));
 await user.upload(input, yaml); await user.click(await screen.findByRole("button", { name: "Keep editing" }));
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Keep my changes');
 await user.upload(input, yaml); await user.click(await screen.findByRole("button", { name: "Replace draft" })); await waitFor(() => expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Imported board'));
 expect((input as HTMLInputElement).value).toBe('');
 await user.upload(input, file('version: 99\nmetadata: {name: bad}\nextractor: {kind: greenhouse, url: acme}'));
 await screen.findByText(/Unsupported workflow version/);
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Imported board'); expect(save).not.toHaveBeenCalled();
});
it('registry upload opens an unsaved draft and Run all remains available while scheduled runs are paused in Settings', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_SWE_SOURCE]);
 const run = vi.spyOn(api, 'runScrapeNow').mockRejectedValueOnce(new Error('worker failed')).mockResolvedValue({ status: 'partial', error: 'One workflow failed' } as never);
 const save = vi.spyOn(api, 'saveWorkflow'); const queryClient = new QueryClient();
 render(<QueryClientProvider client={queryClient}><SourcesSettings settings={{ ...DEFAULT_SETTINGS, scraper_enabled: false }} /></QueryClientProvider>);
 expect(screen.queryByRole('switch', { name: 'Automatic scraping enabled' })).toBeNull();
 const user = userEvent.setup(); await user.click(await screen.findByRole('button', { name: 'Run all' }));
 await waitFor(() => expect(useToastStore.getState().toasts.some(toast => toast.title === 'RUN ALL DID NOT COMPLETE')).toBe(true));
 await user.click(screen.getByRole('button', { name: 'Run all' })); await waitFor(() => expect(run).toHaveBeenCalledTimes(2));
 await waitFor(() => expect(useToastStore.getState().toasts.some(toast => toast.message.includes('One workflow failed'))).toBe(true));
 await user.upload(screen.getByRole('button', { name: 'Import workflow YAML' }).parentElement!.querySelector('input')!, file(workflowSpecToYaml(spec)));
 await screen.findByRole('dialog', { name: 'Create workflow' });
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Imported board'); expect(save).not.toHaveBeenCalled();
 await user.click(screen.getByRole('button', { name: 'Continue to Matching' }));
 expect(screen.getByLabelText("Title keywords (any)")).toBeTruthy();
 expect(screen.queryByText(/Global keyword defaults/)).toBeNull();
});
it('confirms against edits made while file validation is in flight and ignores an empty file selection', async () => {
 let finish!: (value: typeof spec) => void;
 const validation = vi.spyOn(api, 'validateWorkflow').mockImplementation(() => new Promise(resolve => { finish = resolve as typeof finish; }));
 render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
 const user = userEvent.setup(), input = screen.getByLabelText('Workflow YAML file');
 await user.upload(input, []); expect(validation).not.toHaveBeenCalled();
 await user.upload(input, file(workflowSpecToYaml(spec)));
 await user.type(screen.getByLabelText('Workflow name'), 'Edited during validation');
 finish(spec);
 await user.click(await screen.findByRole("button", { name: "Keep editing" }));
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Edited during validation');
});
it('shows cancellation while Run all is pending and refreshes after controlled completion', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_SWE_SOURCE]);
 let finish!: (value: Awaited<ReturnType<typeof api.runScrapeNow>>) => void;
 vi.spyOn(api, 'runScrapeNow').mockImplementation(() => new Promise(resolve => { finish = resolve; }));
 const cancel = vi.spyOn(api, 'cancelScrape').mockResolvedValue(true);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup(); await user.click(await screen.findByRole('button', { name: 'Run all' }));
 await user.click(await screen.findByRole('button', { name: 'Cancel run' })); expect(cancel).toHaveBeenCalledTimes(1);
 finish({ id: 'cancelled-fixture', status: 'partial', trigger: 'manual', started_at: '2026-10-15', finished_at: '2026-10-15', jobs_found: 1n, jobs_new: 1n, error: 'Cancelled after partial fixture result', source_id: null });
 await waitFor(() => expect(screen.queryByRole('button', { name: 'Cancel run' })).toBeNull());
});

it('consolidates contextual actions and reports delete failure from the menu', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_SWE_SOURCE]);
 const remove = vi.spyOn(api, 'deleteSource').mockRejectedValue(new Error('database unavailable'));
 const confirm = vi.spyOn(window, 'confirm').mockReturnValue(true);
 const queryClient = new QueryClient();
 render(<QueryClientProvider client={queryClient}><SourcesSettings settings={DEFAULT_SETTINGS} /><ToastContainer /></QueryClientProvider>);
 const user = userEvent.setup();
 const menu = await screen.findByRole('button', { name: `More actions for ${SAMPLE_SWE_SOURCE.name}` });
 expect(menu.querySelector('svg')).toBeTruthy();
 await user.click(menu);
 const labels = await screen.findAllByRole('menuitem');
 expect(labels.findIndex(button => button.textContent === 'Edit workflow')).toBeLessThan(labels.findIndex(button => button.textContent === 'Preview matches'));
 expect(screen.getByRole('menuitem', { name: 'View run history' })).toBeTruthy();
 await user.click(screen.getByRole('menuitem', { name: 'Delete workflow' }));
 await waitFor(() => expect(remove).toHaveBeenCalledWith(SAMPLE_SWE_SOURCE.id));
 expect(confirm).toHaveBeenCalledWith(`DELETE WORKFLOW ${SAMPLE_SWE_SOURCE.name.toUpperCase()}?`);
 expect((await screen.findByRole("alert")).textContent).toContain("database unavailable");
});

it('opens workflow configuration from its name and exposes preview and run history in More actions', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_SWE_SOURCE]);
 const preview = vi.spyOn(api, 'testSource').mockResolvedValue([]);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup();
 await user.click(await screen.findByRole('button', { name: `Edit workflow ${SAMPLE_SWE_SOURCE.name}` }));
 expect(await screen.findByRole('dialog', { name: 'Edit workflow' })).toBeTruthy();
 cleanup();
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 await user.click(await screen.findByRole('button', { name: `More actions for ${SAMPLE_SWE_SOURCE.name}` }));
 await user.click(screen.getByRole('menuitem', { name: 'Preview matches' }));
 await waitFor(() => expect(preview).toHaveBeenCalledWith(SAMPLE_SWE_SOURCE.id));
 await user.click(screen.getByRole('button', { name: `More actions for ${SAMPLE_SWE_SOURCE.name}` }));
 await user.click(screen.getByRole('menuitem', { name: 'View run history' }));
 expect(await screen.findByText(/WORKFLOW OBSERVABILITY & EXECUTION LOGS/)).toBeTruthy();
});

it('opens the latest run details from a failed workflow status', async () => {
 const failedSource = { ...SAMPLE_SWE_SOURCE, last_status: 'error' };
 vi.spyOn(api, 'listSources').mockResolvedValue([failedSource]);
 const listRuns = vi.spyOn(api, 'listSourceRuns').mockResolvedValue([{
  id: 'failed-run', trigger: 'manual', status: 'failed', started_at: '2026-10-15T10:00:00Z',
  finished_at: '2026-10-15T10:01:00Z', jobs_found: 0n, jobs_new: 0n, error: 'Fixture failure', source_id: failedSource.id,
 }]);
 vi.spyOn(api, 'getScrapeLogs').mockResolvedValue([]);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 await userEvent.setup().click(await screen.findByRole('button', { name: `View failed run for ${failedSource.name}` }));
 expect(await screen.findByText(/RUN failed-r/)).toBeTruthy();
 expect(listRuns).toHaveBeenCalledWith(failedSource.id, 10);
});
