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

interface TrackedListProps {
  jobs: Job[];
  selectedJobId: string | null;
  onSelectJob: (job: Job) => void;
  onOpenUrl: (url: string) => void;
}

export function TrackedList({
  jobs,
  selectedJobId,
  onSelectJob,
  onOpenUrl,
}: TrackedListProps) {
  const [sortField, setSortField] = useState<"company" | "title" | "status" | "date_added">("date_added");
  const [sortAsc, setSortAsc] = useState(false);

  const handleSort = (field: "company" | "title" | "status" | "date_added") => {
    if (sortField === field) {
      setSortAsc(!sortAsc);
    } else {
      setSortField(field);
      setSortAsc(true);
    }
  };

  const sortedJobs = [...jobs].sort((a, b) => {
    let cmp = 0;
    if (sortField === "company") {
      cmp = a.company.localeCompare(b.company);
    } else if (sortField === "title") {
      cmp = a.title.localeCompare(b.title);
    } else if (sortField === "status") {
      cmp = a.status.localeCompare(b.status);
    } else {
      cmp = a.date_added.localeCompare(b.date_added);
    }
    return sortAsc ? cmp : -cmp;
  });

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
            <TableHeader onClick={() => handleSort("company")} className="cursor-pointer">
              COMPANY {sortField === "company" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader onClick={() => handleSort("title")} className="cursor-pointer">
              JOB TITLE {sortField === "title" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader onClick={() => handleSort("status")} className="cursor-pointer">
              STAGE {sortField === "status" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader>LOCATION</TableHeader>
            <TableHeader onClick={() => handleSort("date_added")} className="cursor-pointer">
              ADDED {sortField === "date_added" ? (sortAsc ? "▲" : "▼") : ""}
            </TableHeader>
            <TableHeader>ACTIONS</TableHeader>
          </TableRow>
        </TableHead>
        <TableBody>
          {sortedJobs.map((job) => {
            const isSelected = job.id === selectedJobId;

            return (
              <TableRow
                key={job.id}
                onClick={() => onSelectJob(job)}
                className={`cursor-pointer ${
                  isSelected ? "bg-[var(--color-paper-2)] font-bold" : ""
                }`}
              >
                <TableCell className="font-bold uppercase">{job.company}</TableCell>
                <TableCell>{job.title}</TableCell>
                <TableCell>
                  <Tag status={job.status} />
                </TableCell>
                <TableCell className="text-[var(--color-ink-mute)]">
                  {job.location || "REMOTE"}
                </TableCell>
                <TableCell className="text-[var(--color-ink-mute)]">
                  {job.date_added.slice(0, 10)}
                </TableCell>
                <TableCell>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      onOpenUrl(job.url);
                    }}
                    className="px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] hover:bg-[var(--color-ink)] hover:text-[var(--color-paper)] text-[10px] font-mono uppercase cursor-pointer"
                  >
                    OPEN ↗
                  </button>
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
    </div>
  );
}
