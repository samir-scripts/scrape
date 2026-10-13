// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const preview = (spec: WorkflowSpec): WorkflowPreview => ({ spec, jobs: [], fetched: 3, matched: 0, rejected: 3, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: "2026-10-14T13:00:00Z" });
describe("one workflow draft", () => {
  it("creates URL → preview → save with zero matches, preserves cents and does not run implicitly", async () => {
    const user = userEvent.setup(); const save = vi.spyOn(api, "saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
    const run = vi.spyOn(api, "runSourceNow");
    vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => preview(spec));
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await user.type(screen.getByRole("textbox", { name: "Source URL or board identifier" }), "https://boards.greenhouse.io/acme");
    await user.click(screen.getByRole("button", { name: "Detect source" }));
    await screen.findByRole("textbox", { name: "Title keywords (any)" });
    await user.type(screen.getByRole("textbox", { name: "Title keywords (any)" }), "Engineer");
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/3 fetched/);
    await user.click(screen.getByRole("button", { name: "Continue to schedule" }));
    // Schedule editing invalidates the old preview.
    await user.selectOptions(screen.getByRole("combobox", { name: "Schedule" }), "manual");
    expect((screen.getByRole("button", { name: "Save workflow" }) as HTMLButtonElement).disabled).toBe(true);
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/No eligible postings/);
    await user.click(screen.getByRole("button", { name: "Save workflow" }));
    await screen.findByText(/Saving has not started a scrape/);
    expect(save.mock.calls[0][0].schedule?.cadence).toBe("manual");
    expect(save.mock.calls[0][0].filters?.matching?.keywords?.any).toEqual(["Engineer"]);
    expect(run).not.toHaveBeenCalled();
  });
  it("preserves hidden fields across simple, advanced, YAML and ignores a cancelled stale preview", async () => {
    const user = userEvent.setup(); let finish!: (result: WorkflowPreview) => void;
    const source = { ...SAMPLE_SWE_SOURCE, filters: { matching: { keywords: { any: [], all: ["Rust"], exclude: ["Senior"], fields: ["description"] }, compensation: { min_minor: 5025, max_minor: 6075, interval: "hour", currency: "USD", include_unknown: false } } } };
    const mock = vi.spyOn(api, "previewWorkflow").mockImplementation(spec => new Promise(resolve => { finish = resolve; expect(spec.filters?.matching?.compensation?.min_minor).toBe(5025); }));
    const cancel = vi.spyOn(api, "cancelWorkflowPreview").mockResolvedValue(true);
    render(<AddSourceModal isOpen initialSource={source} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Advanced" }));
    await user.click(screen.getByRole("button", { name: "Edit YAML" }));
    expect((screen.getByRole("textbox", { name: "Workflow YAML" }) as HTMLTextAreaElement).value).toContain("5025");
    await user.click(screen.getByRole("button", { name: "Apply YAML to form" }));
    await screen.findByRole("textbox", { name: "Minimum per hour" });
    expect((screen.getByRole("textbox", { name: "Minimum per hour" }) as HTMLInputElement).value).toBe("50.25");
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await user.type(screen.getByRole("textbox", { name: "Workflow name" }), " revised");
    expect(cancel).toHaveBeenCalled();
    finish(preview(mock.mock.calls[0][0]));
    await waitFor(() => expect(screen.queryByText(/3 fetched/)).toBeNull());
  });
});
