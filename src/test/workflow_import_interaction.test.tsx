// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { SourcesSettings } from "../components/views/settings/SourcesSettings";
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
 const confirm = vi.spyOn(window, 'confirm').mockReturnValueOnce(false).mockReturnValue(true);
 const input = screen.getByLabelText('Workflow YAML file');
 const yaml = file(workflowSpecToYaml(spec));
 await user.upload(input, yaml); await waitFor(() => expect(confirm).toHaveBeenCalledTimes(1));
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Keep my changes');
 await user.upload(input, yaml); await waitFor(() => expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Imported board'));
 expect((input as HTMLInputElement).value).toBe('');
 await user.upload(input, file('version: 99\nmetadata: {name: bad}\nextractor: {kind: greenhouse, url: acme}'));
 await screen.findByText(/Unsupported workflow version/);
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Imported board'); expect(save).not.toHaveBeenCalled();
});
it('registry upload opens an unsaved draft and Run all remains available while automatic scheduling is paused', async () => {
 vi.spyOn(api, 'listSources').mockResolvedValue([SAMPLE_SWE_SOURCE]);
 const run = vi.spyOn(api, 'runScrapeNow').mockRejectedValueOnce(new Error('worker failed')).mockResolvedValue({} as never);
 const save = vi.spyOn(api, 'saveWorkflow'); const queryClient = new QueryClient();
 render(<QueryClientProvider client={queryClient}><SourcesSettings settings={{ ...DEFAULT_SETTINGS, scraper_enabled: false }} /></QueryClientProvider>);
 const user = userEvent.setup(); await user.click(await screen.findByRole('button', { name: 'Run all' }));
 await waitFor(() => expect(useToastStore.getState().toasts.some(toast => toast.title === 'RUN ALL FAILED')).toBe(true));
 await user.click(screen.getByRole('button', { name: 'Run all' })); await waitFor(() => expect(run).toHaveBeenCalledTimes(2));
 await user.upload(screen.getByRole('button', { name: 'Import workflow YAML' }).parentElement!.querySelector('input')!, file(workflowSpecToYaml(spec)));
 await screen.findByRole('dialog', { name: 'Create workflow' });
 expect((screen.getByLabelText('Workflow name') as HTMLInputElement).value).toBe('Imported board'); expect(save).not.toHaveBeenCalled();
 await user.click(screen.getByRole('button', { name: 'Continue to Matching' }));
 expect(screen.getAllByText(/No restriction \(explicit override\)/).length).toBeGreaterThan(0);
});
