// @vitest-environment jsdom
import { afterEach, describe, expect, it } from "vitest";
import { cleanup, render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { MultiSelectCombobox } from "../components/ui/MultiSelectCombobox";

afterEach(cleanup);
function Harness({ options = [{ value: "Software Engineer", label: "Software Engineer", searchText: "15-1252 Computer occupations" }, { value: "Line Cook, Short Order", label: "Line Cook, Short Order" }, ...Array.from({ length: 55 }, (_, i) => ({ value: `Role ${String(i).padStart(2, "0")}`, label: `Role ${String(i).padStart(2, "0")}` }))] }: { options?: {value:string;label:string;searchText?:string}[] }) {
  const [value, setValue] = useState<string[]>([]);
  return <><MultiSelectCombobox label="Target roles" value={value} options={options} allowCustom onChange={setValue} /><output aria-label="Selection">{JSON.stringify(value)}</output></>;
}

describe("MultiSelectCombobox", () => {
  it("filters by all search tokens, chooses one literal, prevents normalized duplicates, and removes accessibly", async () => {
    const user = userEvent.setup(); render(<Harness />);
    const input = screen.getByRole("combobox", { name: "Target roles" });
    await user.type(input, "software 15-1252");
    await user.keyboard("{ArrowDown}{Enter}");
    expect(screen.getByLabelText("Selection").textContent).toBe('["Software Engineer"]');
    await user.click(screen.getByRole("button", { name: "Remove Software Engineer from Target roles" }));
    expect(screen.getByLabelText("Selection").textContent).toBe("[]");
  });

  it("keeps comma-bearing labels intact, offers explicit custom values, and paginates browsing", async () => {
    const user = userEvent.setup(); render(<Harness />);
    const input = screen.getByRole("combobox", { name: "Target roles" });
    await user.type(input, "Line Cook");
    expect(screen.getByRole("option", { name: "Line Cook, Short Order" })).toBeTruthy();
    await user.keyboard("{ArrowDown}{Enter}");
    expect(screen.getByLabelText("Selection").textContent).toBe('["Line Cook, Short Order"]');
    await user.clear(input); await user.type(input, "Cloud Platform Generalist");
    expect(screen.getByRole("option", { name: "Use “Cloud Platform Generalist”" })).toBeTruthy();
    await user.keyboard("{ArrowDown}{Enter}");
    expect(screen.getByLabelText("Selection").textContent).toBe('["Line Cook, Short Order","Cloud Platform Generalist"]');
    expect(input.getAttribute("aria-expanded")).toBe("false");
    await user.click(input);
    expect(screen.getAllByRole("status")[0].textContent?.replace(/\s+/g, " ")).toContain("Showing 50 of 57 results.");
    await user.click(screen.getByRole("button", { name: "Show more results" }));
    expect(within(screen.getByRole("listbox")).getAllByRole("option")).toHaveLength(57);
  });

  it("does not commit a search fragment on blur", async () => {
    const user = userEvent.setup(); render(<Harness options={[{ value: "Official Role", label: "Official Role" }]} />);
    const input = screen.getByRole("combobox", { name: "Target roles" });
    await user.type(input, "Official");
    await user.tab();
    expect(screen.getByLabelText("Selection").textContent).toBe("[]");
  });
});
