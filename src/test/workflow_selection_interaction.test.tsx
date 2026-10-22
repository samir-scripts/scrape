// @vitest-environment jsdom
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api } from "../lib/api";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const preview = (spec: WorkflowSpec): WorkflowPreview => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: "2026-10-21T13:00:00Z" });

describe("workflow role selection draft boundary", () => {
  it("keeps role search draft-neutral and preserves a successful preview", async () => {
    const user = userEvent.setup();
    vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => preview(spec));
    render(<AddSourceModal isOpen onClose={vi.fn()} onSourceSaved={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Continue to Criteria" }));
    await user.click(screen.getByRole("button", { name: "Continue to Sources" }));
    await user.click(screen.getByRole("button", { name: "Continue to Schedule" }));
    await user.click(screen.getByRole("button", { name: "Preview matches" }));
    await screen.findByText(/0 examined/);
    const save = screen.getByRole("button", { name: "Save workflow" });
    expect((save as HTMLButtonElement).disabled).toBe(false);

    const steps = screen.getByRole("navigation", { name: "Workflow steps" });
    await user.click(within(steps).getByRole("button", { name: "Roles" }));
    const roles = screen.getByRole("combobox", { name: "Target roles" });
    await user.type(roles, "Software Engineer");
    await user.click(within(steps).getByRole("button", { name: "Schedule" }));
    expect((save as HTMLButtonElement).disabled).toBe(false);
  });
});
