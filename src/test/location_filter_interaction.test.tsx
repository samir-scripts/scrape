// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { useState } from "react";
import { LocationFilter } from "../components/filters/LocationFilter";
import { api } from "../lib/api";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe("workplace and eligible geography controls", () => {
  it("keeps Remote and United States as independent active filters", async () => {
    vi.spyOn(api, "listLocationOptions").mockResolvedValue({
      countries: [],
      regions: [],
    });

    function Harness() {
      const [filter, setFilter] = useState<{
        countries: string[] | null;
        regions: string[] | null;
        remote: string | null;
        includeUnknown: boolean;
      }>({
        countries: null,
        regions: null,
        remote: "all",
        includeUnknown: false,
      });
      return (
        <>
          <LocationFilter {...filter} onChange={setFilter} />
          <output data-testid="active-filter">{JSON.stringify(filter)}</output>
        </>
      );
    }

    render(<Harness />);
    fireEvent.click(screen.getByRole("button", { name: "REMOTE ONLY" }));
    fireEvent.change(screen.getAllByRole("combobox")[0], { target: { value: "US" } });

    expect(JSON.parse(screen.getByTestId("active-filter").textContent || "{}")).toEqual({
      countries: ["US"],
      regions: null,
      remote: "remote",
      includeUnknown: false,
    });
  });
});
