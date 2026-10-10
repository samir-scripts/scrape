import { describe, it, expect } from "vitest";
import { toggleTriStateSelection } from "../components/filters/TriStateFilter";
import type { TriState } from "../bindings/TriState";

describe("TriState Filter Logic - toggleTriStateSelection", () => {
  it("initializes single selection from null or empty array", () => {
    expect(toggleTriStateSelection(null, "yes")).toEqual(["yes"]);
    expect(toggleTriStateSelection([], "no")).toEqual(["no"]);
    expect(toggleTriStateSelection(null, "unknown")).toEqual(["unknown"]);
  });

  it("accumulates multi-select values", () => {
    const step1 = toggleTriStateSelection(null, "yes");
    expect(step1).toEqual(["yes"]);

    const step2 = toggleTriStateSelection(step1, "no");
    expect(step2).toEqual(["yes", "no"]);
  });

  it("removes an already selected item", () => {
    const current: TriState[] = ["yes", "no"];
    const next = toggleTriStateSelection(current, "yes");
    expect(next).toEqual(["no"]);
  });

  it("collapses to null when the last active item is deselected", () => {
    const current: TriState[] = ["yes"];
    const next = toggleTriStateSelection(current, "yes");
    expect(next).toBeNull();
  });

  it("collapses to null when all 3 states are selected (equivalent to unrestricted ANY)", () => {
    const current: TriState[] = ["yes", "no"];
    const next = toggleTriStateSelection(current, "unknown");
    // Selecting all three options means no filter constraint, so it resets to null
    expect(next).toBeNull();
  });
});
