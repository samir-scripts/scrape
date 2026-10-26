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
  expect(screen.getByText("Preview breakdown").closest("details")?.open).toBe(true);
  await userEvent.click(screen.getByText("Preview breakdown"));
  expect(screen.getByText("Preview breakdown").closest("details")?.open).toBe(false);
  await userEvent.click(screen.getByText("Preview breakdown"));
  expect(screen.getByText(/Time limit reached/)).toBeTruthy();
  expect(screen.getByText(/Fictional public HTML source/)).toBeTruthy();
});

function result(overrides: Partial<WorkflowPreview> = {}): WorkflowPreview {
  return {
    spec: SAMPLE_WORKFLOW_TEMPLATES[0].spec,
    branding: [],
    jobs: [],
    fetched: 0,
    matched: 0,
    rejected: 0,
    blocked: 0,
    invalid: 0,
    sampled: false,
    errors: [],
    next_run_at: null,
    ...overrides,
  };
}

it("explains zero extracted candidates without claiming the source has no jobs", () => {
  const preview = result();
  render(<WorkflowPreviewResults preview={preview} draft={preview.spec} setError={() => {}} />);
  expect(screen.getByText(/No posting candidates were extracted from the visited pages/)).toBeTruthy();
  expect(screen.getByText(/This does not establish that no jobs exist/)).toBeTruthy();
  expect(screen.getByText(/A preview with zero matches can be saved/)).toBeTruthy();
  expect(screen.getByText("Preview breakdown").closest("details")?.open).toBe(true);
});

it("explains matching rejections and retains the actual reason breakdown", () => {
  const preview = result({
    fetched: 2,
    rejected: 2,
    sources: [{
      source_id: "company",
      limit: 10,
      examined: 2,
      matched: 0,
      accepted: 0,
      blocked: 0,
      rejected: 2,
      duplicates: 0,
      invalid: 0,
      unresolved: 0,
      limit_reached: false,
      provider_total: null,
      enrichment_warnings: 0,
      termination: "exhausted",
      outcome: "success",
      rejections: [{ reason: "posting_date", count: 2 }],
      rejection_examples: [],
    }],
  });
  render(<WorkflowPreviewResults preview={preview} draft={preview.spec} setError={() => {}} />);
  expect(screen.getByText(/2 examined candidates did not satisfy the configured criteria/)).toBeTruthy();
  expect(screen.getByText("Posting date: 2")).toBeTruthy();
  expect(screen.getByText("0 eligible matches")).toBeTruthy();
});

it("explains when every eligible posting was blocked", () => {
  const preview = result({ fetched: 2, matched: 2, blocked: 2 });
  render(<WorkflowPreviewResults preview={preview} draft={preview.spec} setError={() => {}} />);
  expect(screen.getByText(/All eligible postings were blocked/)).toBeTruthy();
  expect(screen.getByText("0 eligible matches")).toBeTruthy();
});

it("prioritizes actual errors over empty or unresolved explanations", () => {
  const preview = result({ errors: ["Source could not complete usable processing."] });
  render(<WorkflowPreviewResults preview={preview} draft={preview.spec} setError={() => {}} />);
  expect(screen.getByRole("alert").textContent).toBe("Source could not complete usable processing.");
  expect(screen.getByText(/Preview could not establish a usable result:/)).toBeTruthy();
  expect(screen.queryByText(/No posting candidates were extracted/)).toBeNull();
  expect(screen.queryByText(/A preview with zero matches can be saved/)).toBeNull();
});

it("explains unresolved evidence without presenting unresolved candidates as eligible", () => {
  const preview = result({
    fetched: 1,
    sources: [{
      source_id: "undated",
      limit: 10,
      examined: 1,
      matched: 0,
      accepted: 0,
      blocked: 0,
      rejected: 0,
      duplicates: 0,
      invalid: 0,
      unresolved: 1,
      limit_reached: false,
      provider_total: null,
      enrichment_warnings: 0,
      termination: "exhausted",
      outcome: "partial",
      rejections: [],
      rejection_examples: [],
    }],
  });
  render(<WorkflowPreviewResults preview={preview} draft={preview.spec} setError={() => {}} />);
  expect(screen.getByText(/1 examined candidates still need required eligibility evidence/)).toBeTruthy();
  expect(screen.getByText("0 eligible matches")).toBeTruthy();
  expect(screen.queryByText(/A preview with zero matches can be saved/)).toBeNull();
});
