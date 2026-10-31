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
  target_template_ids: ["software-engineering-it-us-48h"],
  candidates: [
    { id: "recognized", name: "Recognized old example", fingerprint: "known-hash", selected_by_default: true, reason: "Exact shipped example" },
    { id: "custom", name: "Custom copy", fingerprint: "custom-hash", selected_by_default: false, reason: "Modified workflow; preserved unless selected" },
  ],
};
const starterPreview: WorkflowReplacementPreview = {
  ...preview,
  target_template_ids: ["software-engineering-last-12-hours", "cherry-hill-fast-food-last-month", "remote-usd-over-60000"],
  candidates: [{ id: "starter-old", name: "Old starter example", fingerprint: "starter-hash", selected_by_default: true, reason: "Exact shipped starter example" }],
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
  await waitFor(() => expect(replace).toHaveBeenCalledWith([{ id: "recognized", fingerprint: "known-hash" }], "test_workflow"));
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
  ], "test_workflow"));
  expect(await within(dialog).findByRole("alert")).toHaveProperty("textContent", expect.stringContaining("Workflow changed while replacing"));
});

it("defaults to the disabled test workflow and never runs a crawl during replacement", async () => {
  const user = userEvent.setup(); mount();
  const request = vi.spyOn(api, "previewTestWorkflowReplacement").mockResolvedValue(preview);
  const run = vi.spyOn(api, "runSourceNow");
  const replace = vi.spyOn(api, "replaceTestWorkflows").mockResolvedValue({
    created: [], removed_ids: ["recognized"], preserved_ids: ["custom"], backup_path: null, setup_required: [],
  });
  await user.click(screen.getByRole("button", { name: "Replace test workflows" }));
  const dialog = await screen.findByRole("dialog", { name: "Replace test workflows" });
  expect(request).toHaveBeenCalledWith("test_workflow");
  expect(within(dialog).getByRole("button", { name: "Test workflow (test-workflows/)" }).getAttribute("aria-pressed")).toBe("true");
  expect(within(dialog).getByText(/It is loaded disabled; enable it and choose Run explicitly/)).toBeTruthy();
  await user.click(within(dialog).getByRole("button", { name: /Replace selected workflows/ }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith([{ id: "recognized", fingerprint: "known-hash" }], "test_workflow"));
  expect(run).not.toHaveBeenCalled();
});

it("switches target and selection together only after a successful preview", async () => {
  const user = userEvent.setup(); mount();
  let resolve!: (value: WorkflowReplacementPreview) => void;
  const pending = new Promise<WorkflowReplacementPreview>(done => { resolve = done; });
  const request = vi.spyOn(api, "previewTestWorkflowReplacement").mockResolvedValueOnce(preview).mockReturnValueOnce(pending);
  const replace = vi.spyOn(api, "replaceTestWorkflows").mockResolvedValue({
    created: [], removed_ids: ["starter-old"], preserved_ids: [], backup_path: null, setup_required: [],
  });
  await user.click(screen.getByRole("button", { name: "Replace test workflows" }));
  const dialog = await screen.findByRole("dialog", { name: "Replace test workflows" });
  await user.click(within(dialog).getByRole("button", { name: "Starter workflows" }));
  expect(request).toHaveBeenLastCalledWith("starter_workflows");
  expect(within(dialog).getByRole("button", { name: /Replace selected workflows/ })).toHaveProperty("disabled", true);
  expect(within(dialog).getByRole("button", { name: "Cancel" })).toHaveProperty("disabled", true);
  expect(within(dialog).getByRole("button", { name: "Test workflow (test-workflows/)" }).getAttribute("aria-pressed")).toBe("true");
  expect(within(dialog).getByRole("status").textContent).toContain("Loading");
  resolve(starterPreview);
  await waitFor(() => expect(within(dialog).getByRole("button", { name: "Starter workflows" }).getAttribute("aria-pressed")).toBe("true"));
  expect(within(dialog).queryByLabelText(/Recognized old example/)).toBeNull();
  expect(within(dialog).getByLabelText(/Old starter example/)).toHaveProperty("checked", true);
  await user.click(within(dialog).getByRole("button", { name: /Replace selected workflows/ }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith([{ id: "starter-old", fingerprint: "starter-hash" }], "starter_workflows"));
});

it("retains the successful target, preview and selection when switching fails", async () => {
  const user = userEvent.setup(); mount();
  vi.spyOn(api, "previewTestWorkflowReplacement").mockResolvedValueOnce(preview).mockRejectedValueOnce(new Error("Starter preview unavailable"));
  const replace = vi.spyOn(api, "replaceTestWorkflows").mockResolvedValue({
    created: [], removed_ids: ["recognized", "custom"], preserved_ids: [], backup_path: null, setup_required: [],
  });
  await user.click(screen.getByRole("button", { name: "Replace test workflows" }));
  const dialog = await screen.findByRole("dialog", { name: "Replace test workflows" });
  await user.click(within(dialog).getByText(/Other workflows/));
  await user.click(within(dialog).getByLabelText(/Custom copy/));
  await user.click(within(dialog).getByRole("button", { name: "Starter workflows" }));
  expect(await within(dialog).findByRole("alert")).toHaveProperty("textContent", expect.stringContaining("Starter preview unavailable"));
  expect(within(dialog).getByRole("button", { name: "Test workflow (test-workflows/)" }).getAttribute("aria-pressed")).toBe("true");
  expect(within(dialog).getByLabelText(/Custom copy/)).toHaveProperty("checked", true);
  await user.click(within(dialog).getByRole("button", { name: /Replace selected workflows/ }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith([
    { id: "recognized", fingerprint: "known-hash" }, { id: "custom", fingerprint: "custom-hash" },
  ], "test_workflow"));
});

it("describes an empty removal selection as adding missing workflows", async () => {
  const user = userEvent.setup(); mount();
  vi.spyOn(api, "previewTestWorkflowReplacement").mockResolvedValue({ ...preview, candidates: [] });
  const replace = vi.spyOn(api, "replaceTestWorkflows").mockResolvedValue({
    created: [], removed_ids: [], preserved_ids: [], backup_path: null, setup_required: [],
  });
  await user.click(screen.getByRole("button", { name: "Replace test workflows" }));
  await user.click(await screen.findByRole("button", { name: "Add missing test workflow" }));
  await waitFor(() => expect(replace).toHaveBeenCalledWith([], "test_workflow"));
});


it("exposes an initial preview failure without allowing replacement", async () => {
  const user = userEvent.setup(); mount();
  vi.spyOn(api, "previewTestWorkflowReplacement").mockRejectedValue(new Error("Preview unavailable"));
  const replace = vi.spyOn(api, "replaceTestWorkflows");
  await user.click(screen.getByRole("button", { name: "Replace test workflows" }));
  expect(await screen.findByRole("alert")).toHaveProperty("textContent", expect.stringContaining("Preview unavailable"));
  expect(screen.queryByRole("dialog", { name: "Replace test workflows" })).toBeNull();
  expect(replace).not.toHaveBeenCalled();
});
