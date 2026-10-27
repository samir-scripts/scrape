import { Icon } from "../../ui/Icon";
import { RemoveJobButton } from "../../ui/RemoveJobButton";
import { useState } from "react";
import type { Job } from "../../../bindings/Job";
import type { JobStatus } from "../../../bindings/JobStatus";
import {
  Table,
  TableHead,
  TableHeader,
  TableBody,
  TableRow,
  TableCell,
} from "../../ui/Table";
import { Tag } from "../../ui/Tag";
import { EmptyState } from "../../ui/EmptyState";
import { FavoriteButton } from "../../ui/FavoriteButton";
import { CompanyLogo } from "../../ui/CompanyLogo";
import {
  formatSalary,
  formatExperience,
  formatWorkAuth,
  triStateToLabel,
} from "../../../lib/filter";
import { formatJobLocation } from "../../../lib/jobLocationLabel";

interface DiscoveredTableProps {
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onUpdateStatus: (jobId: string, status: JobStatus) => void;
  onOpenUrl?: (url: string) => void;
  onToggleFavorite?: (jobId: string) => void;
  onDelete?: (job: Job) => void;
}

export type DiscoveredSortField =
  | "company"
  | "title"
  | "status"
  | "salary"
  | "date_added"
  | "favorited";

export function compareDiscoveredJobs(
  a: Job,
  b: Job,
  sortField: DiscoveredSortField,
  sortAsc: boolean
): number {
  let cmp = 0;
  if (sortField === "favorited") {
    cmp = (a.favorited ? 1 : 0) - (b.favorited ? 1 : 0);
  } else if (sortField === "company") {
    cmp = a.company.localeCompare(b.company);
  } else if (sortField === "title") {
    cmp = a.title.localeCompare(b.title);
  } else if (sortField === "status") {
    cmp = a.status.localeCompare(b.status);
  } else if (sortField === "salary") {
    const aSal = a.salary_min ?? a.salary_max ?? -1;
    const bSal = b.salary_min ?? b.salary_max ?? -1;
    cmp = aSal - bSal;
  } else {
    cmp = a.date_added.localeCompare(b.date_added);
  }
  return sortAsc ? cmp : -cmp;
}

export function DiscoveredTable({
  jobs,
  selectedJobId,
  onSelectJob,
  onUpdateStatus,
  onToggleFavorite,
  onDelete,
}: DiscoveredTableProps) {
  const [sortField, setSortField] = useState<DiscoveredSortField>("date_added");
  const [sortAsc, setSortAsc] = useState(false);

  const handleSort = (field: DiscoveredSortField) => {
    if (sortField === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortField(field);
      setSortAsc(true);
    }
  };

  const sortedJobs = [...jobs].sort((a, b) =>
    compareDiscoveredJobs(a, b, sortField, sortAsc)
  );

  const sortHeader = (field: DiscoveredSortField, label: string, className = "") => (
    <TableHeader className={className} aria-sort={sortField === field ? (sortAsc ? "ascending" : "descending") : "none"}>
      <button type="button" aria-label={`Sort by ${label.toLowerCase()}`} data-control-action="jobs.sort" onClick={() => handleSort(field)}
        className="inline-flex items-center gap-1 min-h-[var(--control-height-sm)] cursor-pointer">
        {field === "favorited" ? <Icon name="star" size="sm" /> : label.toUpperCase()}
        {sortField === field && <Icon name={sortAsc ? "up" : "down"} size="sm" />}
      </button>
    </TableHeader>
  );

  if (jobs.length === 0) {
    return (
      <div className="p-8 flex items-center justify-center">
        <EmptyState
          title="NO POSTINGS FOUND"
          description="ALL DISCOVERED JOBS HAVE BEEN TRIAGED OR FILTERED."
        />
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4 bg-[var(--color-paper)]">
      <Table>
        <TableHead>
          <TableRow>
            {sortHeader("favorited", "Favorites", "w-9 text-center")}
            {sortHeader("company", "Company", "min-w-[140px]")}
            {sortHeader("title", "Job title", "min-w-[180px]")}
            <TableHeader className="min-w-[240px]">
              DESCRIPTION & RELEVANT INFO
            </TableHeader>
            {sortHeader("status", "Status", "")}
            {sortHeader("salary", "Salary", "")}
            <TableHeader>LOCATION</TableHeader>
            <TableHeader>YOE</TableHeader>
            <TableHeader>NEW GRAD</TableHeader>
            <TableHeader>SPONSORSHIP</TableHeader>
            <TableHeader>GREEN CARD</TableHeader>
            <TableHeader>CLEARANCE</TableHeader>
            {sortHeader("date_added", "Added", "")}
            <TableHeader className="text-right">TRIAGE</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {sortedJobs.map((job) => {
            const isSelected = job.id === selectedJobId;
            const descSnippet = job.description_text
              ? job.description_text.replace(/\s+/g, " ").trim().slice(0, 110) +
                (job.description_text.length > 110 ? "…" : "")
              : "—";

            return (
              <TableRow
                key={job.id}
                className={`${
                  isSelected ? "bg-[var(--color-paper-2)] font-bold" : ""
                }`}
              >
                <TableCell className="text-center">
                  <FavoriteButton
                    favorited={job.favorited}
                    onToggle={() => onToggleFavorite?.(job.id)}
                    size="sm"
                  />
                </TableCell>
                <TableCell className="font-bold uppercase whitespace-nowrap">
                  <div className="flex items-center gap-2">
                    <CompanyLogo company={job.company} jobId={job.id} sourceId={job.source_id} logo={job.company_logo} size="sm" />
                    <span>{job.company}</span>
                  </div>
                </TableCell>
                <TableCell className="font-bold"><button type="button" className="normal-case text-left hover:underline" data-control-action="job.inspect" onClick={() => onSelectJob?.(job)}>{job.title}</button></TableCell>
                <TableCell
                  className="text-xs font-mono text-[var(--color-ink-mute)] max-w-xs truncate select-text"
                  title={job.description_text || "No description"}
                >
                  {descSnippet}
                </TableCell>
                <TableCell>
                  <Tag status={job.status} />
                </TableCell>
                <TableCell className="font-mono text-xs whitespace-nowrap">
                  {formatSalary(job)}
                </TableCell>
                <TableCell className="text-[var(--color-ink-mute)] whitespace-nowrap">
                  {formatJobLocation(job.location, job.workplace_type, job.locations)}
                </TableCell>
                <TableCell className="font-mono text-xs whitespace-nowrap">
                  {formatExperience(job)}
                </TableCell>
                <TableCell className="font-mono text-xs">
                  {triStateToLabel(job.is_new_grad)}
                </TableCell>
                <TableCell className="font-mono text-xs">
                  {triStateToLabel(job.sponsorship)}
                </TableCell>
                <TableCell className="font-mono text-xs whitespace-nowrap">
                  {formatWorkAuth(job)}
                </TableCell>
                <TableCell className="font-mono text-xs">
                  {triStateToLabel(job.clearance)}
                </TableCell>
                <TableCell className="text-[var(--color-ink-mute)] whitespace-nowrap font-mono text-xs">
                  {job.date_added.slice(0, 10)}
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-1">
                    <button data-control-action="job.applied.mark"
                      onClick={(e) => {
                        e.stopPropagation();
                        onUpdateStatus(job.id, "applied");
                      }}
                      className="px-1.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-lime)] text-[var(--color-accent-ink)] hover:opacity-90 text-xs font-mono font-bold uppercase cursor-pointer"
                      title="Apply [A]"
                    >
                      APPLY
                    </button>
                    <button data-control-action="job.ignored.mark"
                      onClick={(e) => {
                        e.stopPropagation();
                        onUpdateStatus(job.id, "ignored");
                      }}
                      className="px-1.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] text-xs font-mono uppercase cursor-pointer"
                      title="Ignore [X]"
                    >
                      IGN
                    </button>
                    {onDelete && <RemoveJobButton title={job.title} company={job.company} onClick={e => { e.stopPropagation(); onDelete(job); }} />}
                  </div>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
