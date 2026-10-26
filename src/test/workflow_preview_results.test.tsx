// @vitest-environment jsdom
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { WorkflowPreviewResults } from "../components/views/settings/WorkflowPreviewResults";
import { SAMPLE_WORKFLOW_TEMPLATES } from "../lib/yamlWorkflow";
import type { WorkflowPreview } from "../bindings/WorkflowPreview";

afterEach(cleanup);

it("shows time-limited Preview coverage without claiming zero unresolved candidates", async () => {
  const spec = SAMPLE_WORKFLOW_TEMPLATES[0].spec;
  const preview: WorkflowPreview = {
    spec,
    jobs: [],
    branding: [],
    fetched: 0,
    matched: 0,
    rejected: 0,
    blocked: 0,
    invalid: 0,
    sampled: true,
    errors: [],
    warnings: [],
    next_run_at: null,
    sources: [{
      source_id: "Fictional public HTML source",
      limit: 10,
      examined: 0,
      matched: 0,
      accepted: 0,
      blocked: 0,
      rejected: 0,
      duplicates: 0,
      invalid: 0,
      unresolved: 0,
      limit_reached: false,
      provider_total: null,
      enrichment_warnings: 0,
      termination: "time_limit",
      outcome: "partial",
      rejections: [],
      rejection_examples: [],
    }],
  };
  render(<WorkflowPreviewResults preview={preview} draft={spec} setError={() => {}} />);

  expect(screen.getByText(/Preview stopped at its page, time, request, or result limit/)).toBeTruthy();
  expect(screen.getByText(/Preview-only limits do not change saved source limits/)).toBeTruthy();
  expect(screen.queryByRole("alert")).toBeNull();
  expect(screen.queryByText(/0 candidates need eligibility evidence/)).toBeNull();
  await userEvent.click(screen.getByText("Preview breakdown"));
  expect(screen.getByText(/Time limit reached/)).toBeTruthy();
  expect(screen.getByText(/Fictional public HTML source/)).toBeTruthy();
});
