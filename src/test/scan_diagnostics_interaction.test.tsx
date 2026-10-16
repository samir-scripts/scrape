// @vitest-environment jsdom
import { afterEach, expect, it } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { ScanDiagnostics } from "../components/views/settings/ScanDiagnostics";
import type { SourceScanStats } from "../bindings/SourceScanStats";
afterEach(cleanup);
it("explains primary rejection counts and explicitly overlapping examples through controls", async () => {
  const source: SourceScanStats = {
    source_id: "Authored diagnostic fixture", limit: 20, examined: 438, matched: 2,
    accepted: 2, blocked: 0, rejected: 436, duplicates: 0, invalid: 0,
    limit_reached: false, provider_total: null, termination: "exhausted",
    rejections: [{ reason: "legacy_role_type", count: 436 }],
    rejection_examples: [{ title: "Software Engineer", primary: "legacy_role_type", overlapping: ["legacy_role_type", "geography"] }],
  };
  render(<ScanDiagnostics sources={[source]} />);
  expect(screen.getByText(/438 examined · 2 accepted · 436 matching rejections/)).toBeTruthy();
  expect(screen.getByText("Imported role restriction: 436")).toBeTruthy();
  expect(screen.queryByText(/Provider-reported total/)).toBeNull();
  await userEvent.click(screen.getByText("Why representative postings were rejected"));
  expect(screen.getByText(/Primary categories above count each rejection once/)).toBeTruthy();
  expect(screen.getByText(/Software Engineer: Imported role restriction. Failed conditions: Imported role restriction, Country or region/)).toBeTruthy();
});
