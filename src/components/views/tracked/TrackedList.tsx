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
import { CompanyLogo } from "../../ui/CompanyLogo";
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

export type SortField = "company" | "title" | "status" | "salary" | "date_added" | "favorited";

export function compareTrackedJobs(
  a: Job,
  b: Job,
  sortField: SortField,
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
  const [localSortField, setSortField] = useState<SortField>("date_added");
  const [localSortAsc, setSortAsc] = useState(false);

  const sortField = repositorySort ? repositorySort.replace(/_(asc|desc)$/, "") as SortField : localSortField;
  const sortAsc = repositorySort ? repositorySort.endsWith("_asc") : localSortAsc;
  const handleSort = (field: SortField) => {
    if (onRepositorySort) {
      const ascending = sortField === field ? !sortAsc : true;
      onRepositorySort(`${field}_${ascending ? "asc" : "desc"}` as JobSort);
    }
    if (sortField === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortField(field);
      setSortAsc(true);
    }
  };

  const sortedJobs = onRepositorySort ? jobs : [...jobs].sort((a, b) =>
    compareTrackedJobs(a, b, sortField, sortAsc)
  );

  if (jobs.length === 0) {
    return (
      <div className="p-8 flex items-center justify-center">
        <EmptyState
          title="NO TRACKED APPLICATIONS"
          description="APPLICATIONS MOVED TO APPLIED, INTERVIEWING, OFFER, OR REJECTED APPEAR HERE."
        />
      </div>
    );
  }

  return (
    <div className="flex-1 overflow-y-auto p-4 bg-[var(--color-paper)]">
      <Table>
        <TableHead>
          <TableRow>
            <TableHeader onClick={() => handleSort("favorited")} className="cursor-pointer w-8 text-center" title="Sort by Favorited">
              ★ {sortField === "favorited" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader onClick={() => handleSort("company")} className="cursor-pointer">
              COMPANY {sortField === "company" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader onClick={() => handleSort("title")} className="cursor-pointer">
              JOB TITLE {sortField === "title" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader className="min-w-[200px]">DESCRIPTION & RELEVANT INFO</TableHeader>
            <TableHeader onClick={() => handleSort("status")} className="cursor-pointer">
              STAGE {sortField === "status" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader onClick={() => handleSort("salary")} className="cursor-pointer">
              SALARY {sortField === "salary" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader>LOCATION</TableHeader>
            <TableHeader>YOE</TableHeader>
            <TableHeader>NEW GRAD</TableHeader>
            <TableHeader>SPONSORSHIP</TableHeader>
            <TableHeader>GREEN CARD</TableHeader>
            <TableHeader>CLEARANCE</TableHeader>
            <TableHeader onClick={() => handleSort("date_added")} className="cursor-pointer">
              ADDED {sortField === "date_added" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader>ACTIONS</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {sortedJobs.map((job) => {
            const isSelected = job.id === selectedJobId;
            const descSnippet = job.description_text
              ? job.description_text.replace(/\s+/g, " ").trim().slice(0, 100) +
                (job.description_text.length > 100 ? "…" : "")
              : "—";

            return (
              <TableRow
                key={job.id}
                onClick={() => onSelectJob(job)}
                className={`cursor-pointer ${
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
                    <CompanyLogo
                      company={job.company}
                      iconUrl={job.company_icon_url}
                      size="sm"
                    />
                    <span>{job.company}</span>
                  </div>
                </TableCell>
                <TableCell>{job.title}</TableCell>
                <TableCell
                  className="text-xs font-mono text-[var(--color-ink-mute)] max-w-xs truncate select-text"
                  title={job.description_text || ""}
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
                  {job.location ||
                    (job.workplace_type === "onsite"
                      ? "IN-OFFICE"
                      : job.workplace_type === "hybrid"
                      ? "HYBRID"
                      : "REMOTE")}
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
                <TableCell className="text-[var(--color-ink-mute)] whitespace-nowrap">
                  {job.date_added.slice(0, 10)}
                </TableCell>
                <TableCell>
                  <div className="flex items-center gap-1.5">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenUrl(job.url);
                      }}
                      className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] text-xs font-mono uppercase cursor-pointer"
                    >
                      OPEN ↗
                    </button>
                    {onDelete && (
                      <button
                        onClick={(e) => {
                          e.stopPropagation();
                          onDelete(job);
                        }}
                        className="px-1.5 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-cherry)] hover:text-white text-xs font-mono font-bold uppercase cursor-pointer"
                        title="Delete application record"
                      >
                        DEL
                      </button>
                    )}
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
