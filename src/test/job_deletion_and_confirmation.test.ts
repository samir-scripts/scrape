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

      expect(html).toContain("Remove saved job");
      expect(html).toContain(sampleJob.title);
      expect(html).toContain(sampleJob.company);
      expect(html).toContain("Keep this job out of future scrapes");
      expect(html).toContain("remain recoverable");
      expect(html).toContain("Remove job");
      expect(html).toContain("Cancel");
      expect(html).toContain("var(--color-danger)");
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

    it("only explicit blocking prevents rediscovery, and restore preserves saved state", async () => {
      const fixture = { ...sampleJob, posting_identity: "fixture-posting" };
      __seedDevJobs([fixture]);
      const removed = await api.removeJob(fixture.id, true);
      expect((await api.listJobs()).total).toBe(0n);
      expect((await api.listRemovedPostings("", "blocked")).items[0].identity).toBe(removed.identity);
      await api.setPostingBlocked(removed.identity, false);
      expect((await api.listJobs()).total).toBe(0n);
      await api.restorePosting(removed.identity);
      const restored = (await api.listJobs()).items[0];
      expect(restored.notes).toBe(sampleJob.notes);
      expect(restored.status).toBe(sampleJob.status);
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

      expect(html).toContain("Job removal always asks");
      expect(html).not.toContain("CONFIRM BEFORE DELETING JOBS");
    });
  });

  describe("Discovered View Deletion Trigger Elements", () => {
    it("renders REMOVE JOB button in JobDetail triage toolbar when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobDetail, {
          job: sampleJob,
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
          onOpenUrl: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain("aria-label=\"Remove ");
      expect(html).toContain(`Remove ${sampleJob.title} at ${sampleJob.company}`);
      expect(html).toContain("text-[var(--color-danger)]");
    });

    it("omits REMOVE JOB button in JobDetail when onDelete is not provided", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobDetail, {
          job: sampleJob,
          onUpdateStatus: () => {},
          onUpdateNotes: () => {},
          onOpenUrl: () => {},
        })
      );

      expect(html).not.toContain(`aria-label="Remove ${sampleJob.title} at ${sampleJob.company}"`);
    });

    it("renders a named SVG delete button in JobCard header cluster when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(JobCard, {
          job: sampleJob,
          onDelete: () => {},
        })
      );

      expect(html).toContain(`aria-label="Remove ${sampleJob.title} at ${sampleJob.company}"`);
      expect(html).toContain('aria-hidden="true"');
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

      expect(html).toContain(`Remove ${sampleJob.title} at ${sampleJob.company}`);
      expect(html).toContain("Remove saved job");
    });
  });

  describe("Tracked View Deletion Trigger Elements", () => {
    it("renders REMOVE JOB in JobDetailDrawer when onDelete is supplied", () => {
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

      expect(html).toContain("aria-label=\"Remove ");
      expect(html).toContain("border-current");
    });

    it("renders an accessible removal icon in TrackedList table action cell when onDelete is supplied", () => {
      const html = renderToStaticMarkup(
        React.createElement(TrackedList, {
          jobs: [sampleJob],
          selectedJobId: null,
          onSelectJob: () => {},
          onOpenUrl: () => {},
          onDelete: () => {},
        })
      );

      expect(html).toContain(`Remove ${sampleJob.title} at ${sampleJob.company}`);
      expect(html).toContain("Remove saved job");
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

      expect(html).toContain(`Remove ${sampleJob.title} at ${sampleJob.company}`);
      expect(html).toContain("Remove saved job");
    });
  });
});
