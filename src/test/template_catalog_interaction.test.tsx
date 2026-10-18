// @vitest-environment jsdom
import { useState } from "react";
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkflowSections } from "../components/views/settings/WorkflowSections";
import { SAMPLE_WORKFLOW_TEMPLATES, workflowSpecToYaml, validateWorkflowYaml } from "../lib/yamlWorkflow";
import type { WorkflowSpec } from "../bindings/WorkflowSpec";

afterEach(cleanup);
function Harness() {
  const [draft, change] = useState<WorkflowSpec>(structuredClone(SAMPLE_WORKFLOW_TEMPLATES[0].spec));
  return <>
    <WorkflowSections draft={draft} change={change} advanced={false} section="Source" />
    <WorkflowSections draft={draft} change={change} advanced={false} section="Matching" />
    <output data-testid="draft">{workflowSpecToYaml(draft)}</output>
  </>;
}
it("loads a canonical template and explicitly combines known and unknown evidence choices", async () => {
  const user = userEvent.setup();
  render(<Harness />);
  await user.click(screen.getByRole("button", { name: "Choose sample workflow template" }));
  const template = SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === "sep-midwest-consulting")!;
  await user.click(screen.getByRole("option", { name: template.name }));
  expect(validateWorkflowYaml(screen.getByTestId("draft").textContent!).parsedSpec).toEqual(template.spec);
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
it("labels bundled script setup and keeps every executable source disabled", async () => {
  const user = userEvent.setup();
  render(<Harness />);
  await user.click(screen.getByRole("button", { name: "Choose sample workflow template" }));
  const template = SAMPLE_WORKFLOW_TEMPLATES.find(template => template.id === "streaming-cloud-script")!;
  await user.click(screen.getByRole("option", { name: template.name }));
  expect(screen.getByText(template.description)).toBeTruthy();
  const draft = validateWorkflowYaml(screen.getByTestId("draft").textContent!).parsedSpec!;
  expect(draft.metadata.enabled).toBe(false);
  expect(draft.sources.every(source => !source.enabled)).toBe(true);
  expect(screen.getAllByRole("checkbox", { name: /^Enable (Netflix|Amazon)$/ }).every(input => !(input as HTMLInputElement).checked)).toBe(true);
});
