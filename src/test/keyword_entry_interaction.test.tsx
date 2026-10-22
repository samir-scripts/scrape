// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import type { UserEvent } from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import { workflowSpecToYaml, validateWorkflowYaml, sourceToWorkflowSpec } from "../lib/yamlWorkflow";
import { emptyRules } from "../components/views/settings/WorkflowSections";
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
async function goToSchedule(user: UserEvent) {
  for (const name of ["Continue to Criteria", "Continue to Sources", "Continue to Schedule"]) {
    const button = screen.queryByRole("button", { name });
    if (button) await user.click(button);
  }
}
async function previewFromSchedule(user: UserEvent) {
  await goToSchedule(user);
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
}
async function backToRoles(user: UserEvent) {
  for (let step = 0; step < 3; step++) await user.click(screen.getByRole("button", { name: "Back" }));
}
it("commits a typed comma and preserves the next keyword", async () => {
  const user = userEvent.setup();
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  ;
  const input = screen.getByRole("combobox", { name: "Any keywords" });
  await user.type(input, "Engineer,Developer{Enter}");
  expect(screen.getByRole("button", { name: "Remove Engineer from Any keywords" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remove Developer from Any keywords" })).toBeTruthy();
});

it("pastes phrases, deduplicates without changing first spelling, and requires focus before Backspace deletion", async () => {
  const user = userEvent.setup();
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  ;
  const input = screen.getByRole("combobox", { name: "Any keywords" }) as HTMLInputElement;
  await user.click(input);
  await user.paste("Engineer, engineer,Machine Learning\nDeveloper\n\n");
  expect(screen.getAllByRole("button", { name: /^Remove .* from Any keywords$/ })).toHaveLength(3);
  expect(screen.getByRole("button", { name: "Remove Machine Learning from Any keywords" })).toBeTruthy();
  await user.type(input, "Rust");
  await user.click(screen.getByRole("button", { name: "Add keyword to Any keywords" }));
  await user.keyboard("{Backspace}");
  const last = screen.getByRole("button", { name: "Remove Rust from Any keywords" });
  expect(document.activeElement).toBe(last);
  expect(screen.getAllByRole("button", { name: /^Remove .* from Any keywords$/ })).toHaveLength(4);
  await user.keyboard("{Backspace}");
  expect(screen.queryByRole("button", { name: "Remove Rust from Any keywords" })).toBeNull();
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Remove Developer from Any keywords" })));
  await user.keyboard("{Delete}");
  expect(screen.queryByRole("button", { name: "Remove Developer from Any keywords" })).toBeNull();
});

it("keeps IME composition intact before committing the completed phrase", async () => {
  const user = userEvent.setup();
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  ;
  const input = screen.getByRole("combobox", { name: "Any keywords" });
  fireEvent.compositionStart(input);
  fireEvent.change(input, { target: { value: "機械,学習" } });
  fireEvent.keyDown(input, { key: "Enter", isComposing: true, keyCode: 229 });
  expect(screen.queryByRole("button", { name: /^Remove .* from Any keywords$/ })).toBeNull();
  expect((input as HTMLInputElement).value).toBe("機械,学習");
  fireEvent.change(input, { target: { value: "機械学習" } });
  fireEvent.compositionEnd(input);
  await user.click(screen.getByRole("button", { name: "Add keyword to Any keywords" }));
  expect(screen.getByRole("button", { name: "Remove 機械学習 from Any keywords" })).toBeTruthy();
});

it("flushes unfinished keyword drafts on navigation without committing role/company search text", async () => {
  const user = userEvent.setup();
  const extraction = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  const save = vi.spyOn(api, "saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  const roles = screen.getByRole("combobox", { name: "Target roles" });
  fireEvent.change(roles, { target: { value: "Data Engineer" } });
  const companies = screen.getByRole("combobox", { name: "Companies (Optional)" });
  fireEvent.change(companies, { target: { value: "Acme" } });
  fireEvent.change(screen.getByRole("combobox", { name: "Any keywords" }), { target: { value: "Software" } });
  fireEvent.change(screen.getByRole("combobox", { name: "Exclude keywords" }), { target: { value: "Senior" } });
  // Navigation synchronously commits keyword drafts; a role/company search fragment is never a selection.
  fireEvent.click(screen.getByRole("button", { name: "Continue to Criteria" }));
  await previewFromSchedule(user);
  await waitFor(() => expect(extraction).toHaveBeenCalledTimes(1));
  expect(extraction.mock.calls[0][0].filters?.matching?.keywords).toEqual({ any: ["Software"], all: [], exclude: ["Senior"], fields: ["title"] });
  expect(extraction.mock.calls[0][0].filters?.matching?.target_roles).toEqual([]);
  expect(extraction.mock.calls[0][0].filters?.matching?.companies).toEqual([]);
  await backToRoles(user);
  fireEvent.change(screen.getByRole("combobox", { name: "All keywords" }), { target: { value: "Engineer" } });
  fireEvent.click(screen.getByRole("button", { name: "Continue to Criteria" }));
  expect(screen.queryByText(/examined/)).toBeNull();
  await previewFromSchedule(user);
  await waitFor(() => expect(extraction).toHaveBeenCalledTimes(2));
  const rules = extraction.mock.calls[1][0].filters?.matching?.keywords;
  expect(rules).toEqual({ any: ["Software"], all: ["Engineer"], exclude: ["Senior"], fields: ["title"] });
  const encoded = workflowSpecToYaml(extraction.mock.calls[1][0]);
  expect(validateWorkflowYaml(encoded).parsedSpec?.filters?.matching?.keywords).toEqual(rules);
  await user.click(screen.getByRole("button", { name: "Save workflow" }));
  await waitFor(() => expect(save).toHaveBeenCalledTimes(1));
  expect(save.mock.calls[0][0].filters?.matching?.keywords).toEqual(rules);
});

it("shows mixed imported groups and search fields without opening Advanced, preserving both groups", async () => {
  const user = userEvent.setup();
  const spec = sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters: {} });
  spec.filters = { matching: { ...emptyRules(), keywords: { any: ["Engineer", "Developer"], all: ["Machine Learning"], exclude: ["Senior"], fields: ["description"] } } };
  const extraction = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  ;
  expect(screen.getByRole("combobox", { name: "Any keywords" })).toBeTruthy();
  expect(screen.getByRole("combobox", { name: "All keywords" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Match keywords" })).toBeNull();
  expect((screen.getByRole("checkbox", { name: "description" }) as HTMLInputElement).checked).toBe(true);
  await user.type(screen.getByRole("combobox", { name: "Any keywords" }), "Scientist");
  await previewFromSchedule(user);
  await waitFor(() => expect(extraction).toHaveBeenCalledTimes(1));
  expect(extraction.mock.calls[0][0].filters?.matching?.keywords).toEqual({ any: ["Engineer", "Developer", "Scientist"], all: ["Machine Learning"], exclude: ["Senior"], fields: ["description"] });
  await backToRoles(user);
  for (const term of ["Engineer", "Developer", "Scientist"]) await user.click(screen.getByRole("button", { name: `Remove ${term} from Any keywords` }));
  expect(screen.getByRole("combobox", { name: "Any keywords" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remove Machine Learning from All keywords" })).toBeTruthy();

});

it("commits unfinished text before opening YAML and clears old drafts when replacing a workflow", async () => {
  const user = userEvent.setup();
  const props = { isOpen: true, onClose: vi.fn(), onSourceSaved: vi.fn() };
  const view = render(<AddSourceModal {...props} initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} />);
  ;
  await user.type(screen.getByRole("combobox", { name: "Any keywords" }), "Machine Learning");
  await user.click(screen.getByRole("button", { name: "Advanced settings" }));
  await user.click(screen.getByRole("button", { name: "Edit YAML" }));
  expect((screen.getByRole("textbox", { name: "Workflow YAML" }) as HTMLTextAreaElement).value).toContain("Machine Learning");
  await user.click(screen.getByRole("button", { name: "Apply YAML to form" }));
  await screen.findByRole("combobox", { name: "Any keywords" });
  await user.type(screen.getByRole("combobox", { name: "Any keywords" }), "Unfinished old draft");
  view.rerender(<AddSourceModal {...props} initialSource={{ ...SAMPLE_SWE_SOURCE, id: "replacement", filters: {} }} />);
  ;
  await waitFor(() => expect((screen.getByRole("combobox", { name: "Any keywords" }) as HTMLInputElement).value).toBe(""));
  expect(screen.queryByRole("button", { name: /^Remove .* from Any keywords$/ })).toBeNull();
});

it("retains imported All mode after removing its last term", async () => {
  const user = userEvent.setup();
  const spec = sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters: {} });
  spec.filters = { matching: { ...emptyRules(), keywords: { any: [], all: ["Engineer"], exclude: [], fields: ["title"] } } };
  render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  ;
  await user.click(screen.getByRole("button", { name: "Remove Engineer from All keywords" }));
  expect(screen.queryByRole("button", { name: "Match keywords" })).toBeNull();
  await user.type(screen.getByRole("combobox", { name: "All keywords" }), "Developer{Enter}");
  expect(screen.getByRole("button", { name: "Remove Developer from All keywords" })).toBeTruthy();
});

it("preserves the exact effective spelling of imported terms during navigation and preview", async () => {
  const user = userEvent.setup();
  const spec = sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters: {} });
  spec.filters = { matching: { ...emptyRules(), keywords: { any: [" Engineer "], all: [], exclude: [], fields: ["title"] } } };
  const preview = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  ;
  await goToSchedule(user);
  ;
  await previewFromSchedule(user);
  await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
  expect(preview.mock.calls[0][0].filters?.matching?.keywords?.any).toEqual([" Engineer "]);
});
it("completes contextual keyword suggestions without changing role filters or adding the typed prefix", async () => {
  const user = userEvent.setup();
  const spec = sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters: {} });
  spec.filters = { matching: { ...emptyRules(), target_roles: ["Software Engineer"], keywords: { any: ["Machine Learning"], all: [], exclude: [], fields: ["title"] } } };
  const preview = vi.spyOn(api, "previewWorkflow").mockImplementation(async input => ({ spec: input, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  const any = screen.getByRole("combobox", { name: "Any keywords" });
  await user.type(any, "software");
  await user.keyboard("{ArrowDown}{Enter}");
  expect(screen.getByRole("button", { name: "Remove Software Engineer from Any keywords" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Remove software from Any keywords" })).toBeNull();
  await previewFromSchedule(user);
  await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
  expect(preview.mock.calls[0][0].filters?.matching?.target_roles).toEqual(["Software Engineer"]);
  expect(preview.mock.calls[0][0].filters?.matching?.keywords?.any).toEqual(["Machine Learning", "Software Engineer"]);
});
