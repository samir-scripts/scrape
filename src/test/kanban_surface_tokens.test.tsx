// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { cleanup, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { SystemSettings } from "../components/views/settings/SystemSettings";
import { KanbanBoard } from "../components/views/tracked/KanbanBoard";
import { KanbanColumn } from "../components/views/tracked/KanbanColumn";
import { KanbanCard, KanbanCardPreview } from "../components/views/tracked/KanbanCard";
import { api, DEFAULT_SETTINGS } from "../lib/api";
import { useUIStore } from "../lib/store";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "../App";
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

  it("renders KanbanColumn using raised header surface, structural 1px border, and concise 'No jobs on this page.' empty state", () => {
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

    // Header surface should be raised for clear contrast against column/board
    const header = container.querySelector("header");
    expect(header).toBeTruthy();
    expect(header?.className).toContain("bg-[var(--color-surface-raised)]");
    expect(header?.className).toContain("border-b");
    expect(header?.className).toContain("border-[var(--color-ink-mute)]");

    // Header should contain ink-bordered stage swatch
    const swatch = header?.querySelector("span[aria-hidden='true']");
    expect(swatch).toBeTruthy();
    expect(swatch?.className).toContain("border-[var(--color-ink)]");

    // Header count shows 0 loaded
    expect(screen.getByText("0 loaded")).toBeTruthy();

    // Empty state should be concise "No jobs on this page."
    expect(screen.getByText("No jobs on this page.")).toBeTruthy();
    expect(
      screen.queryByText(/No jobs loaded in this stage\. Check other pages or move an application here\./i)
    ).toBeNull();
  });

  it("renders KanbanCard with semantic card surface, interviewing accent, and drag preview with shared Tag", () => {
    const { container } = render(
      <KanbanCard
        job={{ ...sampleJob, status: "interviewing" }}
        title={<span>{sampleJob.title}</span>}
      />
    );

    const card = container.querySelector("article");
    expect(card).toBeTruthy();
    expect(card?.className).toContain("bg-[var(--color-kanban-card)]");
    expect(card?.style.borderLeftColor).toBe("var(--color-stage-accent-interviewing)");

    // Card preview should use shared Tag component
    cleanup();
    const { container: previewContainer } = render(<KanbanCardPreview job={sampleJob} />);
    const previewCard = previewContainer.querySelector("article");
    expect(previewCard?.className).toContain("bg-[var(--color-kanban-card)]");
    // Shared Tag renders uppercase label in a font-mono tag with border
    expect(screen.getByText("APPLIED")).toBeTruthy();
    expect(screen.getByText("Move to")).toBeTruthy();
  });

  it("hides JobPaginator on Tracked Kanban board and displays it on Tracked List", async () => {
    useUIStore.setState({ view: "tracked", trackedViewMode: "kanban" });
    vi.spyOn(api, "getSettings").mockResolvedValue({ ...DEFAULT_SETTINGS, app_auto_update: false });
    vi.spyOn(api, "listJobs").mockImplementation(async (filter) => {
      if (filter?.limit === 1) return { items: [], total: 0n, limit: 1n, offset: 0n };
      return { items: [], total: 0n, limit: 200n, offset: 0n };
    });
    vi.spyOn(api, "countUnreadNotifications").mockResolvedValue(0);
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = userEvent.setup();

    render(
      <QueryClientProvider client={client}>
        <App />
      </QueryClientProvider>
    );

    // Wait for boot sequence to finish and Kanban board to appear
    await screen.findByRole("region", { name: "APPLIED stage" });

    // In Kanban mode, the bottom JobPaginator navigation should NOT be present
    expect(screen.queryByRole("navigation", { name: "Job pagination" })).toBeNull();
    expect(screen.queryByText(/0-0 OF 0/i)).toBeNull();

    // Switch to List mode via SubNav button
    const listBtn = screen.getByRole("button", { name: "LIST" });
    await user.click(listBtn);

    // In List mode, JobPaginator should be mounted
    expect(await screen.findByRole("navigation", { name: "Job pagination" })).toBeTruthy();
  });
});
