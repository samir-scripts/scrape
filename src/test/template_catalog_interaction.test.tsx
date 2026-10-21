// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkflowSections } from "../components/views/settings/WorkflowSections";
import { SAMPLE_WORKFLOW_TEMPLATES, workflowSpecToYaml, validateWorkflowYaml } from "../lib/yamlWorkflow";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";
import { AddSourceModal } from "../components/views/settings/AddSourceModal";

afterEach(cleanup);
function Harness() {
  const [draft, change] = useState<WorkflowSpec>(structuredClone(SAMPLE_WORKFLOW_TEMPLATES[0].spec));
  return <>
    <WorkflowSections draft={draft} change={change} advanced={false} section="Source" />
    <WorkflowSections draft={draft} change={change} advanced={false} section="Matching" />
    <output data-testid="draft">{workflowSpecToYaml(draft)}</output>
  </>;
}
it("explicitly combines known and unknown evidence choices without mutating the catalog", async () => {
  const user = userEvent.setup();
  render(<Harness />);
  const template = SAMPLE_WORKFLOW_TEMPLATES[0];
  await user.click(screen.getByRole("checkbox", { name: "Full-time" }));
  await user.click(screen.getByRole("checkbox", { name: "Unknown employment" }));
  await user.click(screen.getByRole("checkbox", { name: "Remote" }));
  await user.click(screen.getByRole("checkbox", { name: "Unknown workplace" }));
  const spec = validateWorkflowYaml(screen.getByTestId("draft").textContent!).parsedSpec!;
  expect(spec.filters?.matching?.role_types).toEqual(["full_time", "unknown"]);
  expect(spec.filters?.matching?.workplaces).toEqual(["remote", "unknown"]);
  expect(spec.filters?.matching?.location_include_unknown).toBe(false);
  // A new draft must never mutate the canonical data shared by other editors.
  expect(template.spec.filters?.matching?.role_types).toEqual([]);
});
it("keeps example loading in Advanced and requires whole-draft replacement confirmation", async () => {
  HTMLDialogElement.prototype.showModal = function () { this.setAttribute("open", ""); };
  HTMLDialogElement.prototype.close = function () { this.removeAttribute("open"); };
  const user = userEvent.setup();
  render(<AddSourceModal isOpen onClose={() => {}} onSourceSaved={() => {}} />);
  await user.type(screen.getByRole("textbox", { name: "Workflow name" }), "Keep my edits");
  expect(screen.queryByRole("button", { name: "Load example workflow" })).toBeNull();
  await user.click(screen.getByRole("button", { name: "Advanced settings" }));
  await user.click(screen.getByRole("button", { name: "Load example workflow" }));
  const template = SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === "streaming-cloud-script")!;
  await user.click(screen.getByRole("option", { name: template.name }));
  expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe("Keep my edits");
  await user.click(screen.getByRole("button", { name: "Keep editing" }));
  expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe("Keep my edits");
  await user.click(screen.getByRole("button", { name: "Load example workflow" }));
  await user.click(screen.getByRole("option", { name: template.name }));
  await user.click(screen.getByRole("button", { name: "Replace draft" }));
  expect((screen.getByRole("textbox", { name: "Workflow name" }) as HTMLInputElement).value).toBe(template.name);
  expect(screen.getAllByRole("checkbox", { name: /^Enable (Netflix|Amazon)$/ }).every(input => !(input as HTMLInputElement).checked)).toBe(true);
  expect((screen.getByRole("button", { name: "Save disabled for review" }) as HTMLButtonElement).disabled).toBe(false);
});
