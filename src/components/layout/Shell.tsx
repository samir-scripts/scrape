import type { JobSort } from "../../bindings/JobSort";
import { recordedProvenance } from "../../lib/recordedJobs";
import { isDemoMode, isFixtureMode } from "../../lib/runtime";
import type { ReactNode } from "react";
import { TopNav } from "./TopNav";
import { SubNav } from "./SubNav";
import { Footer } from "./Footer";
import { ToastContainer } from "../ui/Toast";

interface ShellProps {
  discoveredCount?: number;
  trackedCount?: number;
  nextRunText?: string;
  footerStatus?: string;
  jobSort?: JobSort;
  onJobSortChange?: (sort: JobSort) => void;
  children: ReactNode;
}

export function Shell({
  discoveredCount,
  trackedCount,
  nextRunText,
  footerStatus,
  jobSort,
  onJobSortChange,
  children,
}: ShellProps) {
  return (
    <div className="flex flex-col w-screen h-screen overflow-hidden bg-[var(--color-paper)] text-[var(--color-ink)] select-none">
      {isDemoMode() && <div role="status" className="px-4 py-2 bg-[var(--color-lime)] text-[var(--color-accent-ink)] text-sm font-bold">
        {isFixtureMode() ? "TEST FIXTURES · Synthetic postings and simulated runs." : `RECORDED EXAMPLES · SEP public postings captured ${new Date(recordedProvenance.captured_at).toLocaleDateString()}. Availability may have changed. Open desktop to test a live source.`}
      </div>}
      <TopNav
        discoveredCount={discoveredCount}
        trackedCount={trackedCount}
        nextRunText={nextRunText}
      />
      <SubNav sort={jobSort} onSortChange={onJobSortChange} />
      <div className="flex-1 min-h-0 overflow-hidden flex flex-col">{children}</div>
      <Footer statusMessage={footerStatus} />
      <ToastContainer />
    </div>
  );
}
