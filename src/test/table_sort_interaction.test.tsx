// @vitest-environment jsdom
import { afterEach, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { DiscoveredTable } from "../components/views/discovered/DiscoveredTable";
import { generateScrapedJobsForSource, SAMPLE_ENGINEERING_SOURCE } from "../lib/api";

afterEach(cleanup);
it("sorts named SVG and text headers by keyboard and announces the direction", async () => {
  const sample = generateScrapedJobsForSource(SAMPLE_ENGINEERING_SOURCE)[0];
  render(<DiscoveredTable jobs={[
    { ...sample, id: "b", title: "B engineer", favorited: true },
    { ...sample, id: "a", title: "A engineer", favorited: false },
  ]} selectedJobId={null} onSelectJob={vi.fn()} onUpdateStatus={vi.fn()} onOpenUrl={vi.fn()} />);
  const user = userEvent.setup();
  const title = screen.getByRole("button", { name: "Sort by job title" });
  title.focus(); await user.keyboard("{Enter}");
  expect(screen.getAllByRole("row")[1].textContent).toContain("A engineer");
  expect(title.closest("th")?.getAttribute("aria-sort")).toBe("ascending");
  const favorite = screen.getByRole("button", { name: "Sort by favorites" });
  favorite.focus(); await user.keyboard("{Enter}"); await user.keyboard(" ");
  expect(screen.getAllByRole("row")[1].textContent).toContain("B engineer");
  expect(favorite.closest("th")?.getAttribute("aria-sort")).toBe("descending");
});
