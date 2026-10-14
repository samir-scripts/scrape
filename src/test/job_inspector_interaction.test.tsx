// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { JobInspectorContent } from "../components/jobs/JobInspector";
import { TrackedList } from "../components/views/tracked/TrackedList";
import type { Job } from "../bindings/Job";
import type { JobDescription } from "../bindings/JobDescription";
import { recordedJobs } from "../lib/recordedJobs";
const job: Job = {
  ...recordedJobs()[0],
  id: "job-a",
  title: "Software Engineer with a long provider title",
  company: "Acme",
  url: "https://jobs.acme.test/detail?gh_jid=42",
  status: "applied",
  status_changed_at: "2026-10-12T12:00:00Z",
  date_added: "2026-10-01T12:00:00Z",
  notes: "Existing note",
  role_type: "full_time",
  workplace_type: "unknown",
  location: null,
  locations: [],
  compensation: [],
  description_text: "Actual extracted description",
  favorited: false,
};
beforeEach(() => {
  const values = new Map<string, string>();
  vi.stubGlobal("localStorage", {
    getItem: (key: string) => values.get(key) ?? null,
    setItem: (key: string, value: string) => values.set(key, value),
    clear: () => values.clear(),
  });
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
});
describe("shared job inspector", () => {
  it("renders structured descriptions and sends only safe links to the opener", async () => {
    const open = vi.fn();
    const description: JobDescription = {
      plain_text: "Role\n\nBuild systems\n• First\n• Second",
      truncated: false,
      blocks: [
        { kind: "heading", level: 2, children: [{ kind: "text", text: "Role" }] },
        {
          kind: "paragraph",
          children: [
            { kind: "text", text: "Build " },
            { kind: "link", text: "systems", url: "https://jobs.acme.test/guide" },
            { kind: "link", text: " unsafe", url: "javascript:alert(1)" },
          ],
        },
        {
          kind: "list",
          ordered: false,
          items: [[{ kind: "text", text: "First" }], [{ kind: "text", text: "Second" }]],
        },
      ],
    };
    render(
      <JobInspectorContent
        job={{ ...job, description }}
        onUpdateStatus={vi.fn()}
        onUpdateNotes={vi.fn()}
        onOpenUrl={open}
      />,
    );
    expect(screen.getByRole("heading", { name: "Role" })).toBeTruthy();
    expect(screen.getByText("First")).toBeTruthy();
    expect(screen.getByText("unsafe")).toBeTruthy();
    expect(screen.queryByRole("link", { name: /unsafe/ })).toBeNull();
    await userEvent.click(screen.getByRole("link", { name: "systems" }));
    expect(open).toHaveBeenCalledWith("https://jobs.acme.test/guide");
  });

  it("opens the stored destination, changes stage and removes through distinct controls", async () => {
    const user = userEvent.setup(),
      open = vi.fn(),
      status = vi.fn(),
      remove = vi.fn();
    render(
      <JobInspectorContent
        job={job}
        onUpdateStatus={status}
        onUpdateNotes={vi.fn()}
        onOpenUrl={open}
        onDelete={remove}
      />,
    );
    await user.click(screen.getByRole("button", { name: "Open posting" }));
    expect(open).toHaveBeenCalledWith(job.url);
    await user.selectOptions(
      screen.getByLabelText("Application stage"),
      "interviewing",
    );
    expect(status).toHaveBeenCalledWith(job.id, "interviewing");
    const trigger = screen.getByRole("button", {
      name: `Remove ${job.title} at ${job.company}`,
    });
    expect(trigger.textContent).toBe("");
    await user.click(trigger);
    expect(remove).toHaveBeenCalledWith(job);
    expect(open).toHaveBeenCalledTimes(1);
    expect(screen.getByText("Location unknown")).toBeTruthy();
    expect(screen.queryByText("REMOTE")).toBeNull();
  });
  it("reports note persistence failure, retries, and flushes the correct job on switching", async () => {
    const update = vi
      .fn()
      .mockRejectedValueOnce(new Error("disk full"))
      .mockResolvedValue(undefined);
    const props = {
      onUpdateStatus: vi.fn(),
      onUpdateNotes: update,
      onOpenUrl: vi.fn(),
    };
    const view = render(
      <JobInspectorContent key={job.id} job={job} {...props} />,
    );
    fireEvent.change(screen.getByLabelText("Notes"), {
      target: { value: "Revised note" },
    });
    await screen.findByText("Failed to save");
    await userEvent.click(
      screen.getByRole("button", { name: "Retry saving notes" }),
    );
    await screen.findByText("Saved");
    expect(update).toHaveBeenLastCalledWith(job.id, "Revised note");
    fireEvent.change(screen.getByLabelText("Notes"), {
      target: { value: "Pending note" },
    });
    view.rerender(
      <JobInspectorContent
        key="job-b"
        job={{ ...job, id: "job-b", notes: "Second job note" }}
        {...props}
      />,
    );
    await waitFor(() =>
      expect(update).toHaveBeenLastCalledWith(job.id, "Pending note"),
    );
    expect((screen.getByLabelText("Notes") as HTMLTextAreaElement).value).toBe(
      "Second job note",
    );
  });
  it("defaults to six columns and persists optional columns without losing detail access", async () => {
    const user = userEvent.setup();
    const props = {
      jobs: [job],
      selectedJobId: null,
      onSelectJob: vi.fn(),
      onOpenUrl: vi.fn(),
    };
    const view = render(<TrackedList {...props} />);
    expect(screen.getAllByRole("columnheader")).toHaveLength(6);
    await user.click(screen.getByText("Columns"));
    await user.click(screen.getByLabelText("Experience"));
    expect(screen.getAllByRole("columnheader")).toHaveLength(7);
    view.unmount();
    render(<TrackedList {...props} />);
    expect(screen.getAllByRole("columnheader")).toHaveLength(7);
    await user.click(screen.getByText("Columns"));
    await user.click(screen.getByRole("button", { name: "Reset columns" }));
    expect(screen.getAllByRole("columnheader")).toHaveLength(6);
    await user.click(
      screen.getByRole("button", {
        name: `View ${job.title} at ${job.company}`,
      }),
    );
    expect(props.onSelectJob).toHaveBeenCalledWith(job);
  });
});
