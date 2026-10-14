import { RemoveJobButton } from "../../ui/RemoveJobButton";
import type { JobSort } from "../../../bindings/JobSort";
import { useState } from "react";
import type { Job } from "../../../bindings/Job";
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
import {
  formatSalary,
  formatExperience,
  formatWorkAuth,
  triStateToLabel,
} from "../../../lib/filter";

interface TrackedListProps {
  onRepositorySort?: (sort: JobSort) => void;
  repositorySort?: JobSort | null;
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onOpenUrl: (url: string) => void;
  onToggleFavorite?: (jobId: string) => void;
  onDelete?: (job: Job) => void;
}

export type SortField =
  "company" | "title" | "status" | "salary" | "date_added" | "favorited" | "status_changed_at";

export function compareTrackedJobs(
  a: Job,
  b: Job,
  sortField: SortField,
  sortAsc: boolean,
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
  } else if (sortField === "status_changed_at") {
    cmp = a.status_changed_at.localeCompare(b.status_changed_at);
  } else {
    cmp = a.date_added.localeCompare(b.date_added);
  }
  return sortAsc ? cmp : -cmp;
}

const extraColumns = [
  "Description",
  "Experience",
  "New graduate",
  "Sponsorship",
  "Work authorization",
  "Clearance",
  "Discovered",
  "Source",
] as const;
export function TrackedList({
  jobs,
  selectedJobId,
  onSelectJob,
  onOpenUrl,
  onToggleFavorite,
  onDelete,
  onRepositorySort,
  repositorySort,
}: TrackedListProps) {
  const [localSortField, setSortField] = useState<SortField>("status_changed_at");
  const [localSortAsc, setSortAsc] = useState(false);
  const [columns, setColumns] = useState<string[]>(() => {
    try {
      const value = JSON.parse(
        localStorage.getItem("scrape:tracked:columns") || "[]",
      );
      return Array.isArray(value)
        ? value.filter((c) => extraColumns.includes(c))
        : [];
    } catch {
      return [];
    }
  });
  const changeColumns = (next: string[]) => {
    setColumns(next);
    try {
      localStorage.setItem("scrape:tracked:columns", JSON.stringify(next));
    } catch {}
  };
  const sortField = repositorySort
    ? (repositorySort.replace(/_(asc|desc)$/, "") as SortField)
    : localSortField;
  const sortAsc = repositorySort
    ? repositorySort.endsWith("_asc")
    : localSortAsc;
  const handleSort = (field: SortField) => {
    const ascending = sortField === field ? !sortAsc : true;
    if (onRepositorySort)
      onRepositorySort(`${field}_${ascending ? "asc" : "desc"}` as JobSort);
    setSortField(field);
    setSortAsc(ascending);
  };
  const sortedJobs = onRepositorySort
    ? jobs
    : [...jobs].sort((a, b) => compareTrackedJobs(a, b, sortField, sortAsc));
  const sortButton = (label: string, field: SortField) => (
    <button
      type="button"
      className="cursor-pointer min-h-8 text-left"
      onClick={() => handleSort(field)}
    >
      {label} {sortField === field ? (sortAsc ? "↑" : "↓") : ""}
    </button>
  );
  const extra = (job: Job, column: string) => {
    switch (column) {
      case "Description":
        return job.description_text || "Unknown";
      case "Experience":
        return formatExperience(job);
      case "New graduate":
        return triStateToLabel(job.is_new_grad);
      case "Sponsorship":
        return triStateToLabel(job.sponsorship);
      case "Work authorization":
        return formatWorkAuth(job);
      case "Clearance":
        return triStateToLabel(job.clearance);
      case "Discovered":
        return job.date_added.slice(0, 10);
      case "Source":
        return job.source_id || "Unknown";
    }
  };
  return (
    <div className="tracked-scroll flex-1 min-h-0 overflow-auto p-4 bg-[var(--color-paper)]">
      <details className="mb-3">
        <summary className="cursor-pointer text-sm min-h-8">Columns</summary>
        <div className="flex flex-wrap gap-4 py-3">
          {extraColumns.map((column) => (
            <label className="flex items-center gap-2 text-sm" key={column}>
              <input
                type="checkbox"
                checked={columns.includes(column)}
                onChange={(e) =>
                  changeColumns(
                    e.target.checked
                      ? [...columns, column]
                      : columns.filter((c) => c !== column),
                  )
                }
              />
              {column}
            </label>
          ))}
          <button
            type="button"
            className="underline cursor-pointer"
            onClick={() => changeColumns([])}
          >
            Reset columns
          </button>
        </div>
      </details>
      {!jobs.length ? (
        <EmptyState
          title="NO TRACKED APPLICATIONS"
          description="Applications moved to Applied, Interviewing, Offer, or Rejected appear here."
        />
      ) : (
        <Table className="tracked-table text-sm">
          <TableHead>
            <TableRow>
              <TableHeader className="w-[35%] min-w-[240px]">
                {sortButton("Job", "title")}
              </TableHeader>
              <TableHeader>{sortButton("Stage", "status")}</TableHeader>
              <TableHeader className="tracked-secondary">Location</TableHeader>
              <TableHeader className="tracked-secondary">
                {sortButton("Pay", "salary")}
              </TableHeader>
              <TableHeader className="tracked-secondary">
                {sortButton("Stage updated", "status_changed_at")}
              </TableHeader>
              {columns.map((column) => (
                <TableHeader key={column} className="min-w-[140px]">
                  {column}
                </TableHeader>
              ))}
              <TableHeader>Actions</TableHeader>
            </TableRow>
          </TableHead>
          <TableBody>
            {sortedJobs.map((job) => (
              <TableRow
                key={job.id}
                className={
                  job.id === selectedJobId ? "bg-[var(--color-paper-2)]" : ""
                }
              >
                <TableCell className="min-h-16">
                  <div className="flex items-start gap-2">
                    {onToggleFavorite && (
                      <FavoriteButton
                        favorited={job.favorited}
                        onToggle={() => onToggleFavorite(job.id)}
                      />
                    )}
                    <div className="min-w-0">
                      <button
                        type="button"
                        className="text-left normal-case font-bold underline cursor-pointer text-sm leading-snug"
                        aria-label={`View ${job.title} at ${job.company}`}
                        onClick={() => onSelectJob(job)}
                      >
                        {job.title}
                      </button>
                      <p className="text-sm mt-1 text-[var(--color-ink-mute)]">
                        {job.company}
                      </p>
                      <p className="tracked-mobile-meta mt-1 text-xs">
                        {job.location || "Location unknown"} ·{" "}
                        {formatSalary(job)}
                      </p>
                    </div>
                  </div>
                </TableCell>
                <TableCell>
                  <Tag status={job.status} />
                </TableCell>
                <TableCell className="tracked-secondary text-sm">
                  {job.location || "Location unknown"}
                  {job.workplace_type !== "unknown" && (
                    <p className="text-xs text-[var(--color-ink-mute)]">
                      {job.workplace_type === "onsite"
                        ? "On-site"
                        : job.workplace_type}
                    </p>
                  )}
                </TableCell>
                <TableCell className="tracked-secondary text-sm">
                  {formatSalary(job)}
                </TableCell>
                <TableCell className="tracked-secondary text-sm">
                  {job.status_changed_at.slice(0, 10)}
                </TableCell>
                {columns.map((column) => (
                  <TableCell key={column} className="text-sm max-w-[260px]">
                    <p className="line-clamp-3">{extra(job, column)}</p>
                  </TableCell>
                ))}
                <TableCell>
                  <div className="flex gap-2">
                    <button
                      type="button"
                      aria-label={`Open ${job.title} posting`}
                      title="Open posting"
                      className="min-h-8 min-w-8 cursor-pointer"
                      onClick={() => onOpenUrl(job.url)}
                    >
                      <svg
                        aria-hidden="true"
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        strokeWidth="2"
                      >
                        <path d="M14 3h7v7M21 3 10 14M10 3H3v18h18v-7" />
                      </svg>
                    </button>
                    {onDelete && (
                      <RemoveJobButton
                        title={job.title}
                        company={job.company}
                        onClick={() => onDelete(job)}
                      />
                    )}
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
    </div>
  );
}
