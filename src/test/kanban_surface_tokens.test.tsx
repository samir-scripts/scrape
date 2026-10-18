// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SystemSettings } from "../components/views/settings/SystemSettings";
import { KanbanBoard } from "../components/views/tracked/KanbanBoard";
import { KanbanColumn } from "../components/views/tracked/KanbanColumn";
import { KanbanCard, KanbanCardPreview } from "../components/views/tracked/KanbanCard";
import { DEFAULT_SETTINGS } from "../lib/api";
import { recordedJobs } from "../lib/recordedJobs";
import { useState } from "react";
import type { Job } from "../bindings/Job";

beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
    configurable: true,
    value: vi.fn(),
  });
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

const sampleJob: Job = {
  ...recordedJobs()[0],
  id: "test-card-1",
  status: "applied",
  title: "Frontend Engineer",
  company: "Demo Corp",
};

describe("Section 8: Regional Settings and Kanban Surface Contracts", () => {
  it("provides themed Time format select with 12-hour and 24-hour options", async () => {
    const updates: [string, unknown][] = [];
    function Harness() {
      const [settings, setSettings] = useState(DEFAULT_SETTINGS);
      return (
        <SystemSettings
          settings={settings}
          onUpdateSetting={(key, value) => {
            updates.push([key, value]);
            setSettings((current) => ({ ...current, system_time_format: String(value) }));
          }}
        />
      );
    }
    render(<Harness />);
    const user = userEvent.setup();

    // The time format control should be a themed Select button, not a radio group
    expect(screen.queryByRole("radiogroup", { name: "Time format" })).toBeNull();
    const timeButton = screen.getByRole("button", { name: "Time format" });
    expect(timeButton).toBeTruthy();
    expect(timeButton.textContent).toContain("12-hour · 3:25 PM");

    // Open and switch to 24-hour
    await user.click(timeButton);
    const option24h = await screen.findByRole("option", { name: "24-hour · 15:25" });
    await user.click(option24h);

    expect(updates).toContainEqual(["system.time_format", "24h"]);
    expect(screen.getByRole("button", { name: "Time format" }).textContent).toContain("24-hour · 15:25");

    // Switch back to 12-hour
    await user.click(screen.getByRole("button", { name: "Time format" }));
    const option12h = await screen.findByRole("option", { name: "12-hour · 3:25 PM" });
    await user.click(option12h);

    expect(updates).toContainEqual(["system.time_format", "12h"]);
    expect(screen.getByRole("button", { name: "Time format" }).textContent).toContain("12-hour · 3:25 PM");
  });

  it("renders KanbanBoard using explicit semantic board surface and removes repeated page-scope paragraph", () => {
    const { container } = render(
      <KanbanBoard
        jobs={[sampleJob]}
        selectedJobId={sampleJob.id}
        onSelectJob={vi.fn()}
        onDropJob={vi.fn()}
      />
    );

    // Board container should use the semantic board token
    const boardContainer = container.firstElementChild as HTMLElement;
    expect(boardContainer.className).toContain("bg-[var(--color-kanban-board)]");

    // Repeated page-scope paragraph should be removed
    expect(
      screen.queryByText(/Showing applications on this page\. Stage counts describe loaded jobs\./i)
    ).toBeNull();
  });

  it("renders KanbanColumn using semantic column surface and concise 'No jobs on this page.' empty state", () => {
    const { container } = render(
      <KanbanColumn
        status="interviewing"
        columnIndex={1}
        jobs={[]}
        selectedJobId={null}
        draggingJobId={null}
        onSelectJob={vi.fn()}
        onDropJob={vi.fn()}
        dropTarget={null}
        handleProps={() => ({})}
      />
    );

    const section = container.querySelector("section");
    expect(section).toBeTruthy();
    expect(section?.className).toContain("bg-[var(--color-kanban-column)]");

    // Header count shows 0 loaded
    expect(screen.getByText("0 loaded")).toBeTruthy();

    // Empty state should be concise "No jobs on this page."
    expect(screen.getByText("No jobs on this page.")).toBeTruthy();
    expect(
      screen.queryByText(/No jobs loaded in this stage\. Check other pages or move an application here\./i)
    ).toBeNull();
  });

  it("renders KanbanCard with semantic card surface and drag preview with shared Tag", () => {
    const { container } = render(
      <KanbanCard
        job={sampleJob}
        title={<span>{sampleJob.title}</span>}
      />
    );

    const card = container.querySelector("article");
    expect(card).toBeTruthy();
    expect(card?.className).toContain("bg-[var(--color-kanban-card)]");

    // Card preview should use shared Tag component
    cleanup();
    const { container: previewContainer } = render(<KanbanCardPreview job={sampleJob} />);
    const previewCard = previewContainer.querySelector("article");
    expect(previewCard?.className).toContain("bg-[var(--color-kanban-card)]");
    // Shared Tag renders uppercase label in a font-mono tag with border
    expect(screen.getByText("APPLIED")).toBeTruthy();
    expect(screen.getByText("Move to")).toBeTruthy();
  });
});
