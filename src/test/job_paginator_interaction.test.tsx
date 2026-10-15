// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { JobPaginator } from "../components/jobs/JobPaginator";
import { pagination } from "../lib/pagination";

afterEach(cleanup);
it.each([
  [0, 1, 0, 0, 0], [1, 1, 1, 1, 1], [50, 1, 1, 50, 1], [51, 2, 51, 51, 2], [2105, 43, 2101, 2105, 43], [50, 8, 1, 50, 1],
])("announces the correct range for %i jobs on requested page %i", (total, requested, start, end, count) => {
  const model = pagination(total, requested, 50);
  expect([model.start, model.end, model.pageCount]).toEqual([start, end, count]);
  render(<JobPaginator total={total} page={requested} pageSize={50} onPageChange={vi.fn()} />);
  expect(screen.getByRole("status").textContent).toBe(`${start}–${end} of ${total}`);
  if (!total) expect((screen.getByRole("textbox", { name: "Page number" }) as HTMLInputElement).disabled).toBe(true);
});
it("keeps invalid page drafts editable and focused, Escape restores, and first/last jump directly", async () => {
  const change = vi.fn();
  function Harness() {
    const [page, setPage] = useState(2);
    return <JobPaginator page={page} pageSize={50} total={2105} onPageChange={value => { change(value); setPage(value); }} />;
  }
  render(<Harness />); const user = userEvent.setup();
  const input = screen.getByRole("textbox", { name: "Page number" }) as HTMLInputElement;
  for (const invalid of ["", "0", "44", "-1", "2.5", "1e2", "9007199254740992"]) {
    await user.clear(input); if (invalid) await user.type(input, invalid);
    await user.keyboard("{Enter}");
    expect(screen.getByRole("alert").textContent).toContain("1 to 43");
    expect(document.activeElement).toBe(input);
    expect(change).not.toHaveBeenCalled();
  }
  await user.keyboard("{Escape}"); expect(input.value).toBe("2");
  expect(screen.queryByRole("alert")).toBeNull();
  await user.clear(input); await user.type(input, "43{Enter}");
  expect(change).toHaveBeenLastCalledWith(43);
  expect((screen.getByRole("button", { name: "Next page" }) as HTMLButtonElement).disabled).toBe(true);
  await user.click(screen.getByRole("button", { name: "First page" }));
  expect(change).toHaveBeenLastCalledWith(1);
  await user.click(screen.getByRole("button", { name: "Last page" }));
  expect(change).toHaveBeenLastCalledWith(43);
});
it("allows another direct jump while a known page is loading", async () => {
  const change = vi.fn();
  render(<JobPaginator page={3} pageSize={50} total={2105} busy onPageChange={change} />);
  const user = userEvent.setup();
  await user.clear(screen.getByRole("textbox", { name: "Page number" }));
  await user.type(screen.getByRole("textbox", { name: "Page number" }), "8{Enter}");
  expect(change).toHaveBeenCalledWith(8);
  expect(screen.getByRole("status").textContent).toBe("Loading page 3…");
});
