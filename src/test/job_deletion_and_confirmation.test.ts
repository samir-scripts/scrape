import { describe, it, expect } from "vitest";
import React from "react";
import { renderToStaticMarkup } from "react-dom/server";
import type { Job } from "../bindings/Job";
import { api, DEFAULT_SETTINGS, __seedDevJobs, __resetDevData } from "../lib/api";
import { DeleteJobModal } from "../components/ui/DeleteJobModal";
import { JobDetail } from "../components/views/discovered/JobDetail";
import { JobCard } from "../components/views/discovered/JobCard";
import { DiscoveredTable } from "../components/views/discovered/DiscoveredTable";
import { JobDetailDrawer } from "../components/views/tracked/JobDetailDrawer";
import { TrackedList } from "../components/views/tracked/TrackedList";
import { KanbanColumn } from "../components/views/tracked/KanbanColumn";
import { SystemSettings } from "../components/views/settings/SystemSettings";

const sampleJob: Job = {
  id: "test-job-del-1",
  dedupe_key: "gh:testco:del1",
  source_id: "source-1",
  external_id: "ext-1",
  title: "Principal Security Architect",
  company: "CyberArmor",
  role_type: "full_time",
  location: "San Francisco, CA",
  url: "https://job-boards.greenhouse.io/cyberarmor/jobs/1",
  date_posted: "2026-10-10",
  date_added: "2026-10-10T00:00:00Z",
  last_seen_at: "2026-10-10T00:00:00Z",
  status: "scraped",
  status_changed_at: "2026-10-10T00:00:00Z",
  notes: "Urgent opening",
  salary_min: 220000,
  salary_max: 280000,
  salary_currency: "USD",
  salary_interval: "year",
  salary_raw: "$220,000 - $280,000",
  salary_source: "structured",
  workplace_type: "hybrid",
  sponsorship: "no",
  sponsorship_evidence: "No visa support",
  work_auth: "citizen_only",
  work_auth_evidence: "Must be US Citizen",
  clearance: "yes",
  clearance_level: "top_secret",
  clearance_evidence: "Active Top Secret clearance required",
  experience_min: 8,
  experience_max: 12,
  experience_raw: "8-12 years",
  is_new_grad: "no",
  experience_evidence: "8+ years in cloud security",
  description_text: "Seeking a Principal Security Architect to lead our cloud security team.",
  company_icon_url: null,
  favorited: false,
  favorited_at: null,
  enrichment_version: 1,
  locations: [],
};

describe("Job Deletion & Confirmation Dialog Engine", () => {
  describe("DeleteJobModal Markup & Brutalist Design", () => {
    it("renders confirmation prompt, job details, and tombstone notice when isOpen is true", () => {
      const html = renderToStaticMarkup(
        React.createElement(DeleteJobModal, {
          isOpen: true,
          job: sampleJob,
          onClose: () => {},
          onConfirm: () => {},
        })
      );

      expect(html).toContain("Are you sure you want to delete?");
      expect(html).toContain("Principal Security Architect");
      expect(html).toContain("CyberArmor");
      expect(html).toContain("CONFIRM DELETION // DANGER");
      expect(html).toContain(
        "THIS POSTING WILL BE TOMBSTONED AND EXCLUDED FROM ALL FUTURE SCRAPES"
      );
      expect(html).toContain("DON&#x27;T ASK AGAIN");
      expect(html).toContain("[DEL] DELETE");
      expect(html).toContain("[ESC] CANCEL");
      expect(html).toContain("bg-[var(--color-cherry)]");
    });

    it("renders nothing when isOpen is false", () => {
      const html = renderToStaticMarkup(
        React.createElement(DeleteJobModal, {
          isOpen: false,
          job: sampleJob,
          onClose: () => {},
          onConfirm: () => {},
        })
      );

      expect(html).toBe("");
    });
  });

  describe("API deleteJob & Dev Storage Integration", () => {
    it("deletes a job from local dev store and returns true", async () => {
      __seedDevJobs([sampleJob]);
      const jobsBefore = await api.listJobs();
      expect(jobsBefore.items.length).toBeGreaterThan(0);

      const targetId = jobsBefore.items[0].id;
      const deleted = await api.deleteJob(targetId);
      expect(deleted).toBe(true);

      const jobsAfter = await api.listJobs();
      expect(jobsAfter.items.some((j) => j.id === targetId)).toBe(false);
    });

    it("returns false when attempting to delete a non-existent job ID", async () => {
      const deleted = await api.deleteJob("non-existent-job-id-9999");
      expect(deleted).toBe(false);
    });

    it("prevents deleted job from resurrecting during workflow execution", async () => {
      __seedDevJobs([{ ...sampleJob, id: "job-to-delete", dedupe_key: "cloudflare:8199958" }]);
      const deleted = await api.deleteJob("job-to-delete");
      expect(deleted).toBe(true);

      // Trigger workflow run - candidate with dedupe_key "cloudflare:8199958" should be ignored
      await api.runSourceNow("sample-swe-internships-1");
      const list = await api.listJobs();
      expect(list.items.some((j) => j.dedupe_key === "cloudflare:8199958")).toBe(false);
    });
  });

  describe("Settings Model & Confirmation Toggle", () => {
    it("verifies DEFAULT_SETTINGS has app_confirm_job_deletion set to true", () => {
      expect(DEFAULT_SETTINGS.app_confirm_job_deletion).toBe(true);
    });

    it("allows updating app.confirm_job_deletion setting to false and back to true", async () => {
      const updatedOff = await api.updateSetting(
        "app.confirm_job_deletion",
        false
      );
      expect(updatedOff.app_confirm_job_deletion).toBe(false);

      const updatedOn = await api.updateSetting(
        "app.confirm_job_deletion",
        true
      );
      expect(updatedOn.app_confirm_job_deletion).toBe(true);
    });

    it("renders CONFIRM BEFORE DELETING JOBS toggle in SystemSettings", () => {
      const html = renderToStaticMarkup(
        React.createElement(SystemSettings, {
          settings: { ...DEFAULT_SETTINGS, app_confirm_job_deletion: true },
          onUpdateSetting: () => {},
        })
      );

      expect(html).toContain("CONFIRM BEFORE DELETING JOBS");
      expect(html).toContain(
        "PROMPT FOR CONFIRMATION BEFORE DELETING JOBS FROM DISCOVERED OR TRACKED"
      );
    });
  });

  describe("Discovered View Deletion Trigger Elements", () => {
    it("renders DELETE POSTING button in JobDetail triage toolbar when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobDetail, {
          job: sampleJob,
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
          onOpenUrl: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain("DELETE POSTING");
      expect(html).toContain("DEL");
      expect(html).toContain("text-[var(--color-cherry)]");
    });

    it("omits DELETE POSTING button in JobDetail when onDelete is not provided", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobDetail, {
          job: sampleJob,
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
          onOpenUrl: () => {},
        })
      );

      expect(html).not.toContain("DELETE POSTING");
    });

    it("renders a named SVG delete button in JobCard header cluster when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: sampleJob,
          onDelete: () => {},
        })
      );

      expect(html).toContain(`aria-label="Delete ${sampleJob.title} at ${sampleJob.company}"`);
      expect(html).toContain('aria-hidden="true"');
      expect(html).toContain("min-w-8 min-h-8");
    });

    it("renders DEL button in DiscoveredTable row actions when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(DiscoveredTable, {
          jobs: [sampleJob],
          selectedJobId: null,
          onSelectJob: () => {},
          onUpdateStatus: () => {},
          onOpenUrl: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain("DEL");
      expect(html).toContain("Delete posting");
    });
  });

  describe("Tracked View Deletion Trigger Elements", () => {
    it("renders DELETE APPLICATION RECORD in JobDetailDrawer when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobDetailDrawer, {
          isOpen: true,
          job: sampleJob,
          onClose: () => {},
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
          onOpenUrl: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain("DELETE APPLICATION RECORD");
      expect(html).toContain("border-[var(--color-cherry)]");
    });

    it("renders DEL button in TrackedList table action cell when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(TrackedList, {
          jobs: [sampleJob],
          selectedJobId: null,
          onSelectJob: () => {},
          onOpenUrl: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain("DEL");
      expect(html).toContain("Delete application record");
    });

    it("renders DEL button in KanbanColumn card bottom row when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(KanbanColumn, {
          status: "applied",
          columnIndex: 0,
          jobs: [{ ...sampleJob, status: "applied" }],
          selectedJobId: null,
          draggingJobId: null,
          onSelectJob: () => {},
          onDropJob: () => {},
          onDragStartJob: () => {},
          onDragEndJob: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain("DEL");
      expect(html).toContain("Delete application record");
    });
  });
});
