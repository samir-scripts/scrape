// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useHotkeys, useGlobalHotkeys } from "../hooks/useHotkeys";
import { JobCard } from "../components/views/discovered/JobCard";
import { generateScrapedJobsForSource, SAMPLE_SWE_SOURCE } from "../lib/api";

afterEach(cleanup);

describe("independent card keyboard controls", () => {
  it("opens the stored destination without activating details; delete/favorite also stay independent", async () => {
    const job = {
      ...generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0],
      location: "Remote",
      workplace_type: "remote" as const,
      locations: [
        {
          raw: "Available Locations: US",
          city: null,
          region_code: null,
          country_code: "US",
          is_remote: false,
        },
      ],
    };
    const details = vi.fn(), open = vi.fn(), remove = vi.fn(), favorite = vi.fn();
    const globalOpen = vi.fn();
    function Harness() {
      useGlobalHotkeys(); useHotkeys({ onOpenUrl: globalOpen });
      return <JobCard job={job} onClick={details} onOpenUrl={open} onDelete={remove} onToggleFavorite={favorite} />;
    }
    render(<Harness />);
    expect(screen.getByText("Remote · United States")).toBeTruthy();
    const user = userEvent.setup();
    screen.getByRole("button", { name: /^Open .* in browser$/ }).focus();
    await user.keyboard("{Enter}");
    expect(open).toHaveBeenCalledExactlyOnceWith(job.url);
    expect(details).not.toHaveBeenCalled();
    screen.getByRole("button", { name: /^Remove / }).focus();
    await user.keyboard(" ");
    expect(remove).toHaveBeenCalledExactlyOnceWith(job);
    expect(details).not.toHaveBeenCalled();
    screen.getByRole("button", { name: "Favorite posting" }).focus();
    await user.keyboard("{Enter}");
    expect(favorite).toHaveBeenCalledExactlyOnceWith(job.id);
    expect(details).not.toHaveBeenCalled();
    screen.getByRole("button", { name: /^View / }).focus();
    await user.keyboard("{Enter}");
    expect(details).toHaveBeenCalledTimes(1);
    expect(globalOpen).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /^Remove / }).querySelector("svg")).not.toBeNull();
  });

  it("renders an unread pip for newly scraped jobs, clears it when viewed, and avoids noisy header badges", () => {
    const baseJob = generateScrapedJobsForSource(SAMPLE_SWE_SOURCE)[0];
    const { rerender } = render(<JobCard job={{ ...baseJob, status: "scraped" }} />);
    expect(screen.getByTitle("New discovery")).toBeTruthy();
    expect(screen.queryByText("SCRAPED")).toBeNull();

    rerender(<JobCard job={{ ...baseJob, status: "viewed" }} />);
    expect(screen.queryByTitle("New discovery")).toBeNull();
    expect(screen.queryByText("VIEWED")).toBeNull();

    rerender(<JobCard job={{ ...baseJob, status: "applied" }} />);
    expect(screen.queryByTitle("New discovery")).toBeNull();
    expect(screen.getByText("APPLIED")).toBeTruthy();
  });
});
