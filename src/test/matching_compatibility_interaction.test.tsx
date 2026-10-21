// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import { emptyRules } from "../components/views/settings/WorkflowSections";
import { sourceToWorkflowSpec, validateWorkflowYaml, workflowSpecToYaml } from "../lib/yamlWorkflow";
import type { WorkflowFilters } from "../bindings/WorkflowFilters";

beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
const filters: WorkflowFilters = {
  include_keywords: ["Software"], exclude_keywords: ["Staff"], locations: ["Remote", "Austin"], role_types: ["internship"],
  query: "title LIKE '%Engineer%' OR title LIKE '%Developer%' OR title LIKE '%Intern%'",
  policy: { company: "%Fixture%" }, compensation: { min_minor: 5000, max_minor: null, interval: "hour", currency: "USD", include_unknown: true },
  matching: { ...emptyRules(), experience_match_mode: "advertised_range_overlap", role_types: ["full_time"], keywords: { any: ["Engineer"], all: ["Software"], exclude: ["Senior"], fields: ["description"] } },
};
function setup() {
  vi.spyOn(api, "describeWorkflowFilters").mockResolvedValue([
    { field: "include_keywords", explanation: "Rust explanation: title OR location; AND structured groups." },
    { field: "locations", explanation: "Rust explanation: original location terms use OR." },
    { field: "role_types", explanation: "Rust explanation: both role groups must match." },
  ]);
  vi.spyOn(api, "validateWorkflow").mockImplementation(async spec => spec as ReturnType<typeof sourceToWorkflowSpec>);
  const preview = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  const save = vi.spyOn(api, "saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
  const initial = { ...SAMPLE_SWE_SOURCE, filters };
  render(<AddSourceModal isOpen initialSource={initial} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  return { preview, save };
}
it("keeps every compatibility restriction visible and intact through Preview, YAML, Save and reopen conversion", async () => {
  const user = userEvent.setup(); const { preview, save } = setup();
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  expect(screen.queryByRole("button", { name: "Migrate to structured rules" })).toBeNull();
  expect(screen.getByRole("textbox", { name: "Expert query" }).getAttribute("value")).toBe(filters.query);
  expect(screen.getByRole("textbox", { name: "Expert policy (JSON)" })).toBeTruthy();
  expect(screen.getByText("Legacy role types: internship")).toBeTruthy();
  expect(screen.getByText(/Legacy compensation:/)).toBeTruthy();
  await screen.findByText("Rust explanation: original location terms use OR.");
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
  expect(preview.mock.calls[0][0].filters).toEqual(filters);
  const encoded = workflowSpecToYaml(preview.mock.calls[0][0]);
  expect(validateWorkflowYaml(encoded).parsedSpec?.filters).toEqual(filters);
  await user.click(screen.getByRole("button", { name: "Continue to Schedule" }));
  await user.click(screen.getByRole("button", { name: "Review workflow" }));
  await user.click(screen.getByRole("button", { name: "Save workflow" }));
  await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
  expect(save.mock.calls[0][0].filters).toEqual(filters);
  expect(sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters }).filters).toEqual(filters);
});
it("explicit edits invalidate Preview, retain the other groups, and commit multiple unfinished compatibility inputs", async () => {
  const user = userEvent.setup(); const { preview } = setup();
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
  await user.click(screen.getByRole("button", { name: "Remove legacy role-type restriction" }));
  expect(screen.queryByText("Legacy role types: internship")).toBeNull();
  fireEvent.change(screen.getByRole("textbox", { name: "Legacy include keywords" }), { target: { value: "Developer" } });
  fireEvent.change(screen.getByRole("textbox", { name: "Legacy location terms" }), { target: { value: "London" } });
  fireEvent.change(screen.getByRole("textbox", { name: "Expert policy (JSON)" }), { target: { value: '{"company":"%Edited%"}' } });
  fireEvent.change(screen.getByRole("textbox", { name: "Expert query" }), { target: { value: "title LIKE '%Software%'" } });
  fireEvent.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(preview).toHaveBeenCalledTimes(2));
  expect(preview.mock.calls[1][0].filters).toEqual({ ...filters, role_types: [], include_keywords: ["Software", "Developer"], locations: ["Remote", "Austin", "London"], policy: { company: "%Edited%" }, query: "title LIKE '%Software%'" });
});
it("allows ordinary typing in new expert fields and forwards malformed policy drafts for authoritative rejection", async () => {
  const user = userEvent.setup();
  vi.spyOn(api, "describeWorkflowFilters").mockResolvedValue([]);
  vi.spyOn(api, "validateWorkflow").mockImplementation(async spec => spec as ReturnType<typeof sourceToWorkflowSpec>);
  const preview = vi.spyOn(api, "previewWorkflow").mockRejectedValue(new Error("workflow.filters.policy: invalid policy expression"));
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  await user.click(screen.getByRole("button", { name: "Advanced settings" }));
  const query = screen.getByRole("textbox", { name: "Expert query" });
  await user.type(query, "title LIKE '%Engineer%'");
  expect((query as HTMLInputElement).value).toBe("title LIKE '%Engineer%'");
  expect(document.activeElement).toBe(query);
  const policy = screen.getByRole("textbox", { name: "Expert policy (JSON)" });
  fireEvent.change(policy, { target: { value: '{"title":' } });
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  await screen.findByText(/invalid policy expression/);
  expect(preview.mock.calls[0][0].filters?.policy).toBe('{"title":');
  expect(preview.mock.calls[0][0].filters?.query).toBe("title LIKE '%Engineer%'");
});
