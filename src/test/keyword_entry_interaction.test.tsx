// @vitest-environment jsdom
import { afterEach, beforeAll, expect, it, vi } from "vitest";
import { cleanup, render, screen, fireEvent, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";
import { api, SAMPLE_SWE_SOURCE } from "../lib/api";
import { workflowSpecToYaml, validateWorkflowYaml, sourceToWorkflowSpec } from "../lib/yamlWorkflow";
import { emptyRules } from "../components/views/settings/WorkflowSections";
beforeAll(() => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });
it("commits a typed comma and preserves the next keyword", async () => {
  const user = userEvent.setup();
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  const input = screen.getByRole("textbox", { name: "Keywords" });
  await user.type(input, "Engineer,Developer{Enter}");
  expect(screen.getByRole("button", { name: "Remove Engineer from Keywords" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remove Developer from Keywords" })).toBeTruthy();
});

it("pastes phrases, deduplicates without changing first spelling, and requires focus before Backspace deletion", async () => {
  const user = userEvent.setup();
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  const input = screen.getByRole("textbox", { name: "Keywords" }) as HTMLInputElement;
  await user.click(input);
  await user.paste("Engineer, engineer,Machine Learning\nDeveloper\n\n");
  expect(screen.getAllByRole("button", { name: /^Remove .* from Keywords$/ })).toHaveLength(3);
  expect(screen.getByRole("button", { name: "Remove Machine Learning from Keywords" })).toBeTruthy();
  await user.type(input, "Rust");
  await user.click(screen.getByRole("button", { name: "Add keyword to Keywords" }));
  await user.keyboard("{Backspace}");
  const last = screen.getByRole("button", { name: "Remove Rust from Keywords" });
  expect(document.activeElement).toBe(last);
  expect(screen.getAllByRole("button", { name: /^Remove .* from Keywords$/ })).toHaveLength(4);
  await user.keyboard("{Backspace}");
  expect(screen.queryByRole("button", { name: "Remove Rust from Keywords" })).toBeNull();
  await waitFor(() => expect(document.activeElement).toBe(screen.getByRole("button", { name: "Remove Developer from Keywords" })));
  await user.keyboard("{Delete}");
  expect(screen.queryByRole("button", { name: "Remove Developer from Keywords" })).toBeNull();
});

it("keeps IME composition intact before committing the completed phrase", async () => {
  const user = userEvent.setup();
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  const input = screen.getByRole("textbox", { name: "Keywords" });
  fireEvent.compositionStart(input);
  fireEvent.change(input, { target: { value: "機械,学習" } });
  fireEvent.keyDown(input, { key: "Enter", isComposing: true, keyCode: 229 });
  expect(screen.queryByRole("button", { name: /^Remove .* from Keywords$/ })).toBeNull();
  expect((input as HTMLInputElement).value).toBe("機械,学習");
  fireEvent.change(input, { target: { value: "機械学習" } });
  fireEvent.compositionEnd(input);
  await user.click(screen.getByRole("button", { name: "Add keyword to Keywords" }));
  expect(screen.getByRole("button", { name: "Remove 機械学習 from Keywords" })).toBeTruthy();
});

it("flushes unfinished Any and excluded terms before preview, persists All mode, and round-trips YAML", async () => {
  const user = userEvent.setup();
  const extraction = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  const save = vi.spyOn(api, "saveWorkflow").mockResolvedValue(SAMPLE_SWE_SOURCE);
  render(<AddSourceModal isOpen initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  fireEvent.change(screen.getByRole("textbox", { name: "Keywords" }), { target: { value: "Software" } });
  fireEvent.change(screen.getByRole("textbox", { name: "Exclude keywords" }), { target: { value: "Senior" } });
  // No blur: the action itself must synchronously commit both independent drafts.
  fireEvent.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(extraction).toHaveBeenCalledTimes(1));
  expect(extraction.mock.calls[0][0].filters?.matching?.keywords).toEqual({ any: ["Software"], all: [], exclude: ["Senior"], fields: ["title"] });
  await user.click(screen.getByRole("button", { name: "Match keywords" }));
  await user.click(screen.getByRole("option", { name: "All keywords" }));
  await user.type(screen.getByRole("textbox", { name: "Keywords" }), "Engineer");
  fireEvent.click(screen.getByRole("button", { name: "Continue to Schedule" }));
  await user.click(screen.getByRole("button", { name: "Review workflow" }));
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(extraction).toHaveBeenCalledTimes(2));
  const rules = extraction.mock.calls[1][0].filters?.matching?.keywords;
  expect(rules).toEqual({ any: [], all: ["Software", "Engineer"], exclude: ["Senior"], fields: ["title"] });
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
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  expect(screen.getByRole("textbox", { name: "Any keywords" })).toBeTruthy();
  expect(screen.getByRole("textbox", { name: "All keywords" })).toBeTruthy();
  expect(screen.queryByRole("button", { name: "Match keywords" })).toBeNull();
  expect((screen.getByRole("checkbox", { name: "description" }) as HTMLInputElement).checked).toBe(true);
  await user.type(screen.getByRole("textbox", { name: "Any keywords" }), "Scientist");
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(extraction).toHaveBeenCalledTimes(1));
  expect(extraction.mock.calls[0][0].filters?.matching?.keywords).toEqual({ any: ["Engineer", "Developer", "Scientist"], all: ["Machine Learning"], exclude: ["Senior"], fields: ["description"] });
  for (const term of ["Engineer", "Developer", "Scientist"]) await user.click(screen.getByRole("button", { name: `Remove ${term} from Any keywords` }));
  expect(screen.getByRole("textbox", { name: "Any keywords" })).toBeTruthy();
  expect(screen.getByRole("button", { name: "Remove Machine Learning from All keywords" })).toBeTruthy();

});

it("commits unfinished text before opening YAML and clears old drafts when replacing a workflow", async () => {
  const user = userEvent.setup();
  const props = { isOpen: true, onClose: vi.fn(), onSourceSaved: vi.fn() };
  const view = render(<AddSourceModal {...props} initialSource={{ ...SAMPLE_SWE_SOURCE, filters: {} }} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  await user.type(screen.getByRole("textbox", { name: "Keywords" }), "Machine Learning");
  await user.click(screen.getByRole("button", { name: "Advanced settings" }));
  await user.click(screen.getByRole("button", { name: "Edit YAML" }));
  expect((screen.getByRole("textbox", { name: "Workflow YAML" }) as HTMLTextAreaElement).value).toContain("Machine Learning");
  await user.click(screen.getByRole("button", { name: "Apply YAML to form" }));
  await screen.findByRole("textbox", { name: "Keywords" });
  await user.type(screen.getByRole("textbox", { name: "Keywords" }), "Unfinished old draft");
  view.rerender(<AddSourceModal {...props} initialSource={{ ...SAMPLE_SWE_SOURCE, id: "replacement", filters: {} }} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  expect((screen.getByRole("textbox", { name: "Keywords" }) as HTMLInputElement).value).toBe("");
  expect(screen.queryByRole("button", { name: /^Remove .* from Keywords$/ })).toBeNull();
});

it("retains imported All mode after removing its last term", async () => {
  const user = userEvent.setup();
  const spec = sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters: {} });
  spec.filters = { matching: { ...emptyRules(), keywords: { any: [], all: ["Engineer"], exclude: [], fields: ["title"] } } };
  render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  await user.click(screen.getByRole("button", { name: "Remove Engineer from Keywords" }));
  expect(screen.getByRole("button", { name: "Match keywords" }).textContent).toBe("All keywords");
  await user.type(screen.getByRole("textbox", { name: "Keywords" }), "Developer{Enter}");
  expect(screen.getByRole("button", { name: "Remove Developer from Keywords" })).toBeTruthy();
});

it("preserves the exact effective spelling of imported terms during navigation and preview", async () => {
  const user = userEvent.setup();
  const spec = sourceToWorkflowSpec({ ...SAMPLE_SWE_SOURCE, filters: {} });
  spec.filters = { matching: { ...emptyRules(), keywords: { any: [" Engineer "], all: [], exclude: [], fields: ["title"] } } };
  const preview = vi.spyOn(api, "previewWorkflow").mockImplementation(async spec => ({ spec, branding: [], jobs: [], fetched: 0, matched: 0, rejected: 0, invalid: 0, blocked: 0, sampled: false, errors: [], next_run_at: null }));
  render(<AddSourceModal isOpen initialSpec={spec} onClose={vi.fn()} onSourceSaved={vi.fn()} />);
  await user.click(screen.getByRole("button", { name: "Continue to Matching" }));
  await user.click(screen.getByRole("button", { name: "Continue to Schedule" }));
  await user.click(screen.getByRole("button", { name: "Review workflow" }));
  await user.click(screen.getByRole("button", { name: "Preview matches" }));
  await waitFor(() => expect(preview).toHaveBeenCalledTimes(1));
  expect(preview.mock.calls[0][0].filters?.matching?.keywords?.any).toEqual([" Engineer "]);
});
