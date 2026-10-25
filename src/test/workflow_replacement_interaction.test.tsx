// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { SourcesSettings } from "../components/views/settings/SourcesSettings";
import { api, DEFAULT_SETTINGS } from "../lib/api";
import type { WorkflowReplacementPreview } from "../bindings/WorkflowReplacementPreview";

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

const preview: WorkflowReplacementPreview = {
  target_template_ids: ["software-engineering-last-12-hours", "cherry-hill-fast-food-last-month", "remote-usd-over-60000"],
  candidates: [
    { id: "recognized", name: "Recognized old example", fingerprint: "known-hash", selected_by_default: true, reason: "Exact shipped example" },
    { id: "custom", name: "Custom copy", fingerprint: "custom-hash", selected_by_default: false, reason: "Modified workflow; preserved unless selected" },
  ],
};
function mount() {
  vi.spyOn(api, "listSources").mockResolvedValue([]);
  return render(<QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}><SourcesSettings settings={DEFAULT_SETTINGS} /></QueryClientProvider>);
}
async function open(user: ReturnType<typeof userEvent.setup>) {
  vi.spyOn(api, "previewTestWorkflowReplacement").mockResolvedValue(preview);
  await user.click(screen.getByRole("button", { name: "Replace test workflows" }));
  return screen.findByRole("dialog", { name: "Replace test workflows" });
}

it("previews and cancels without invoking replacement", async () => {
  const user = userEvent.setup(); mount();
  const replace = vi.spyOn(api, "replaceTestWorkflows");
  const dialog = await open(user);
  expect(within(dialog).getByLabelText(/Recognized old example/)).toHaveProperty("checked", true);
  expect(within(dialog).getByText(/Saved jobs and tracking are kept/)).toBeTruthy();
  await user.click(within(dialog).getByRole("button", { name: "Cancel" }));
  expect(replace).not.toHaveBeenCalled();
});

it("passes only the preview-selected id and fingerprint and shows the recovery backup", async () => {
  const user = userEvent.setup(); mount(); await open(user);
  const report = { created: [], removed_ids: ["recognized"], preserved_ids: ["custom"], backup_path: "/profile.db.pre-workflow-replacement-123.db", setup_required: [] };
  const replace = vi.spyOn(api, "replaceTestWorkflows").mockResolvedValue(report);
  await user.click(screen.getByRole("button", { name: /Replace selected workflows/ }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith([{ id: "recognized", fingerprint: "known-hash" }]));
  expect(await screen.findByText(report.backup_path)).toBeTruthy();
});

it("allows explicitly selecting an ambiguous workflow and retains the dialog on failure", async () => {
  const user = userEvent.setup(); mount(); await open(user);
  const replace = vi.spyOn(api, "replaceTestWorkflows").mockRejectedValue(new Error("Workflow changed while replacing"));
  const dialog = screen.getByRole("dialog", { name: "Replace test workflows" });
  await user.click(within(dialog).getByText(/Other workflows/));
  await user.click(within(dialog).getByLabelText(/Custom copy/));
  await user.click(within(dialog).getByRole("button", { name: /Replace selected workflows/ }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith([
    { id: "recognized", fingerprint: "known-hash" }, { id: "custom", fingerprint: "custom-hash" },
  ]));
  expect(await within(dialog).findByRole("alert")).toHaveProperty("textContent", expect.stringContaining("Workflow changed while replacing"));
});
