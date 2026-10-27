// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent, { type UserEvent } from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { SourcesSettings } from "../components/views/settings/SourcesSettings";
import { ToastContainer } from "../components/ui/Toast";
import { api, SAMPLE_ENGINEERING_SOURCE, DEFAULT_SETTINGS } from "../lib/api";
import { workflowSpecToYaml, sourceToWorkflowSpec } from "../lib/yamlWorkflow";
import { useToastStore } from "../lib/toast";
const spec = { ...sourceToWorkflowSpec(SAMPLE_ENGINEERING_SOURCE), metadata: { name: "Imported board", enabled: true }, filters: { include_keywords: [], exclude_keywords: [], locations: [] }, schedule: { cadence: "manual" as const, timezone: "UTC" } };
function file(text: string, name = 'workflow.yaml') { const result = new File([text], name, { type: 'text/yaml' }); Object.defineProperty(result, 'text', { value: async () => text }); return result; }
async function requestWorkflowDeletion(user: UserEvent, source = SAMPLE_ENGINEERING_SOURCE) {
 await user.click(await screen.findByRole('button', { name: `More actions for ${source.name}` }));
 await user.click(screen.getByRole('menuitem', { name: 'Delete workflow' }));
 return screen.findByRole('dialog', { name: 'Delete workflow' });
}
beforeAll(() => { HTMLDialogElement.prototype.showModal = function() { this.setAttribute('open',''); }; HTMLDialogElement.prototype.close = function() { this.removeAttribute('open'); }; });
afterEach(() => { cleanup(); vi.restoreAllMocks(); useToastStore.getState().clearAll(); });
it('imports only after validation, confirms replacement, preserves the old draft on failure, and reselects the same file', async () => {
 const save = vi.spyOn(api, 'saveWorkflow');
 render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
 const user = userEvent.setup(); await user.clear(screen.getByLabelText('Workflow name')); await user.type(screen.getByLabelText('Workflow name'), 'Keep my changes');
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
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_ENGINEERING_SOURCE]);
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
 expect(screen.getByLabelText("Any keywords")).toBeTruthy();
 expect(screen.queryByText(/Global keyword defaults/)).toBeNull();
});
it('confirms against edits made while file validation is in flight and ignores an empty file selection', async () => {
 let finish!: (value: typeof spec) => void;
 const validation = vi.spyOn(api, 'validateWorkflow').mockImplementation(() => new Promise(resolve => { finish = resolve as typeof finish; }));
 render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
 const user = userEvent.setup(), input = screen.getByLabelText('Workflow YAML file');
 await user.upload(input, []); expect(validation).not.toHaveBeenCalled();
 await user.upload(input, file(workflowSpecToYaml(spec)));
 await user.clear(screen.getByLabelText('Workflow name'));
 await user.type(screen.getByLabelText('Workflow name'), 'Edited during validation');
 finish(spec);
 await user.click(await screen.findByRole("button", { name: "Keep editing" }));
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Edited during validation');
});
it('shows cancellation while Run all is pending and refreshes after controlled completion', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_ENGINEERING_SOURCE]);
 let finish!: (value: Awaited<ReturnType<typeof api.runScrapeNow>>) => void;
 vi.spyOn(api, 'runScrapeNow').mockImplementation(() => new Promise(resolve => { finish = resolve; }));
 const cancel = vi.spyOn(api, 'cancelScrape').mockResolvedValue(true);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup(); await user.click(await screen.findByRole('button', { name: 'Run all' }));
 await user.click(await screen.findByRole('button', { name: 'Cancel run' })); expect(cancel).toHaveBeenCalledTimes(1);
 finish({ id: 'cancelled-fixture', status: 'cancelled', trigger: 'manual', started_at: '2026-10-15', finished_at: '2026-10-15', jobs_found: 1n, jobs_new: 1n, error: 'Cancelled after partial fixture result', source_id: null });
 await waitFor(() => expect(screen.queryByRole('button', { name: 'Cancel run' })).toBeNull());
 expect(useToastStore.getState().toasts.some(toast => toast.type === 'error')).toBe(false);
 expect(useToastStore.getState().toasts.some(toast => toast.title === 'RUN CANCELLED')).toBe(true);
});

it('consolidates contextual actions and retains the workflow dialog after a readable native delete failure', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_ENGINEERING_SOURCE]);
 const remove = vi.spyOn(api, 'deleteSource').mockRejectedValue({ kind: 'database', message: 'database unavailable' });
 const queryClient = new QueryClient();
 render(<QueryClientProvider client={queryClient}><SourcesSettings settings={DEFAULT_SETTINGS} /><ToastContainer /></QueryClientProvider>);
 const user = userEvent.setup();
 const menu = await screen.findByRole('button', { name: `More actions for ${SAMPLE_ENGINEERING_SOURCE.name}` });
 expect(menu.querySelector('svg')).toBeTruthy();
 await user.click(menu);
 const labels = await screen.findAllByRole('menuitem');
 expect(labels.findIndex(button => button.textContent === 'Edit workflow')).toBeLessThan(labels.findIndex(button => button.textContent === 'Preview matches'));
 expect(screen.getByRole('menuitem', { name: 'View run history' })).toBeTruthy();
 await user.click(screen.getByRole('menuitem', { name: 'Delete workflow' }));
 const dialog = await screen.findByRole('dialog', { name: 'Delete workflow' });
 expect(within(dialog).getByText(SAMPLE_ENGINEERING_SOURCE.name)).toBeTruthy();
 expect(within(dialog).getByText(/Saved jobs, tracking, notes and run history will be kept/)).toBeTruthy();
 expect(remove).not.toHaveBeenCalled();
 const cancel = within(dialog).getByRole('button', { name: 'Cancel' }) as HTMLButtonElement;
 await waitFor(() => expect(document.activeElement).toBe(cancel));
 await user.click(within(dialog).getByRole('button', { name: 'Delete workflow' }));
 await waitFor(() => expect(remove).toHaveBeenCalledWith(SAMPLE_ENGINEERING_SOURCE.id));
 expect(remove).toHaveBeenCalledTimes(1);
 const alert = await screen.findByRole('alert');
 expect(alert.textContent).toContain('WORKFLOW DELETE FAILED');
 expect(alert.textContent).toContain('database unavailable');
 expect(alert.textContent).not.toContain('[object Object]');
 expect(screen.getByRole('dialog', { name: 'Delete workflow' })).toBe(dialog);
 expect(screen.getByRole('button', { name: `Edit workflow ${SAMPLE_ENGINEERING_SOURCE.name}` })).toBeTruthy();
 expect(cancel.disabled).toBe(false);
 expect((within(dialog).getByRole('button', { name: 'Delete workflow' }) as HTMLButtonElement).disabled).toBe(false);
 await user.click(cancel);
 await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Delete workflow' })).toBeNull());
 await waitFor(() => expect(document.activeElement).toBe(menu));
});

it('focuses Cancel after the menu frame and dismisses without deleting or refreshing', async () => {
 const list = vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_ENGINEERING_SOURCE]);
 const remove = vi.spyOn(api, 'deleteSource').mockResolvedValue(undefined);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup();
 const menu = await screen.findByRole('button', { name: `More actions for ${SAMPLE_ENGINEERING_SOURCE.name}` });
 for (const dismissal of ['cancel', 'escape', 'header', 'backdrop'] as const) {
  const dialog = await requestWorkflowDeletion(user);
  const cancel = within(dialog).getByRole('button', { name: 'Cancel' });
  await waitFor(() => expect(document.activeElement).toBe(cancel));
  expect(remove).not.toHaveBeenCalled();
  if (dismissal === 'cancel') await user.click(cancel);
  else if (dismissal === 'escape') await user.keyboard('{Escape}');
  else if (dismissal === 'header') await user.click(within(dialog).getByRole('button', { name: 'Close dialog' }));
  else await user.click(dialog);
  await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Delete workflow' })).toBeNull());
  await waitFor(() => expect(document.activeElement).toBe(menu));
 }
 expect(remove).not.toHaveBeenCalled();
 expect(list).toHaveBeenCalledTimes(1);
 expect(screen.getByRole('button', { name: `Edit workflow ${SAMPLE_ENGINEERING_SOURCE.name}` })).toBeTruthy();
});

it.each(['deleted', 'surviving'] as const)('deletes the captured workflow, refreshes the list and preserves inspection with the %s workflow selected', async inspectedWorkflow => {
 const remaining = { ...SAMPLE_ENGINEERING_SOURCE, id: 'remaining-workflow', name: 'Remaining workflow' };
 const list = vi.spyOn(api, 'listSources').mockResolvedValueOnce([SAMPLE_ENGINEERING_SOURCE, remaining]).mockResolvedValue([remaining]);
 const runs = vi.spyOn(api, 'listSourceRuns').mockResolvedValue([]);
 const remove = vi.spyOn(api, 'deleteSource').mockResolvedValue(undefined);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup();
 const inspected = inspectedWorkflow === 'deleted' ? SAMPLE_ENGINEERING_SOURCE : remaining;
 await user.click(await screen.findByRole('button', { name: `More actions for ${inspected.name}` }));
 await user.click(screen.getByRole('menuitem', { name: 'View run history' }));
 expect(await screen.findByText(`${inspected.name} // WORKFLOW OBSERVABILITY & EXECUTION LOGS`)).toBeTruthy();
 const dialog = await requestWorkflowDeletion(user);
 expect(remove).not.toHaveBeenCalled();
 await user.click(within(dialog).getByRole('button', { name: 'Delete workflow' }));
 await waitFor(() => expect(list).toHaveBeenCalledTimes(2));
 await waitFor(() => expect(screen.queryByRole('button', { name: `Edit workflow ${SAMPLE_ENGINEERING_SOURCE.name}` })).toBeNull());
 expect(remove).toHaveBeenCalledTimes(1);
 expect(remove).toHaveBeenCalledWith(SAMPLE_ENGINEERING_SOURCE.id);
 expect(screen.queryByRole('dialog', { name: 'Delete workflow' })).toBeNull();
 expect(screen.getByRole('button', { name: `Edit workflow ${remaining.name}` })).toBeTruthy();
 expect(await screen.findByText(`${remaining.name} // WORKFLOW OBSERVABILITY & EXECUTION LOGS`)).toBeTruthy();
 expect(runs).toHaveBeenCalledWith(remaining.id, 10);
 expect(useToastStore.getState().toasts.some(toast => toast.message === `Deleted workflow ${SAMPLE_ENGINEERING_SOURCE.name}`)).toBe(true);
 await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Add workflow' })));
});

it('blocks duplicate deletion and every dismissal path until the pending deletion completes', async () => {
 const remaining = { ...SAMPLE_ENGINEERING_SOURCE, id: 'remaining-workflow', name: 'Remaining workflow' };
 const list = vi.spyOn(api, 'listSources').mockResolvedValueOnce([SAMPLE_ENGINEERING_SOURCE, remaining]).mockResolvedValue([remaining]);
 let finish!: () => void;
 const remove = vi.spyOn(api, 'deleteSource').mockImplementation(() => new Promise(resolve => { finish = resolve; }));
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup();
 const dialog = await requestWorkflowDeletion(user);
 const confirm = within(dialog).getByRole('button', { name: 'Delete workflow' }) as HTMLButtonElement;
 const cancel = within(dialog).getByRole('button', { name: 'Cancel' }) as HTMLButtonElement;
 await user.dblClick(confirm);
 expect(remove).toHaveBeenCalledTimes(1);
 expect(remove).toHaveBeenCalledWith(SAMPLE_ENGINEERING_SOURCE.id);
 expect(confirm.disabled).toBe(true);
 expect(cancel.disabled).toBe(true);
 expect(within(dialog).getByRole('status').textContent).toContain('Deleting workflow');
 await user.keyboard('{Escape}');
 await user.click(dialog);
 await user.click(within(dialog).getByRole('button', { name: 'Close dialog' }));
 const nativeCancel = new Event('cancel', { cancelable: true });
 fireEvent(dialog, nativeCancel);
 expect(nativeCancel.defaultPrevented).toBe(true);
 expect(screen.getByRole('dialog', { name: 'Delete workflow' })).toBe(dialog);
 expect(screen.getByRole('button', { name: `Edit workflow ${SAMPLE_ENGINEERING_SOURCE.name}` })).toBeTruthy();
 expect(remove).toHaveBeenCalledTimes(1);
 expect(list).toHaveBeenCalledTimes(1);
 await act(async () => { finish(); });
 await waitFor(() => expect(screen.queryByRole('dialog', { name: 'Delete workflow' })).toBeNull());
 await waitFor(() => expect(screen.queryByRole('button', { name: `Edit workflow ${SAMPLE_ENGINEERING_SOURCE.name}` })).toBeNull());
 expect(screen.getByRole('button', { name: `Edit workflow ${remaining.name}` })).toBeTruthy();
 expect(list).toHaveBeenCalledTimes(2);
 await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Add workflow' })));
});

it('closes after a successful delete and reports refresh failure through the existing load-error UI', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValueOnce([SAMPLE_ENGINEERING_SOURCE]).mockRejectedValue(new Error('refresh unavailable'));
 const remove = vi.spyOn(api, 'deleteSource').mockResolvedValue(undefined);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup();
 const dialog = await requestWorkflowDeletion(user);
 await user.click(within(dialog).getByRole('button', { name: 'Delete workflow' }));
 expect((await screen.findByRole('alert')).textContent).toContain('Could not load workflows: Error: refresh unavailable');
 expect(screen.getByRole('button', { name: 'Retry loading workflows' })).toBeTruthy();
 expect(screen.queryByRole('dialog', { name: 'Delete workflow' })).toBeNull();
 expect(remove).toHaveBeenCalledWith(SAMPLE_ENGINEERING_SOURCE.id);
 expect(useToastStore.getState().toasts.some(toast => toast.title === 'WORKFLOW DELETE FAILED')).toBe(false);
 await waitFor(() => expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Add workflow' })));
});

it('opens workflow configuration from its name and exposes preview and run history in More actions', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_ENGINEERING_SOURCE]);
 const preview = vi.spyOn(api, 'testSource').mockResolvedValue([]);
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 const user = userEvent.setup();
 await user.click(await screen.findByRole('button', { name: `Edit workflow ${SAMPLE_ENGINEERING_SOURCE.name}` }));
 expect(await screen.findByRole('dialog', { name: 'Edit workflow' })).toBeTruthy();
 cleanup();
 render(<QueryClientProvider client={new QueryClient()}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
 await user.click(await screen.findByRole('button', { name: `More actions for ${SAMPLE_ENGINEERING_SOURCE.name}` }));
 await user.click(screen.getByRole('menuitem', { name: 'Preview matches' }));
 await waitFor(() => expect(preview).toHaveBeenCalledWith(SAMPLE_ENGINEERING_SOURCE.id));
 await user.click(screen.getByRole('button', { name: `More actions for ${SAMPLE_ENGINEERING_SOURCE.name}` }));
 await user.click(screen.getByRole('menuitem', { name: 'View run history' }));
 expect(await screen.findByText(/WORKFLOW OBSERVABILITY & EXECUTION LOGS/)).toBeTruthy();
});

it('opens the latest run details from a failed workflow status', async () => {
 const failedSource = { ...SAMPLE_ENGINEERING_SOURCE, last_status: 'error' };
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
