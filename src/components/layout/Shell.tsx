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
  children: ReactNode;
}

export function Shell({
  discoveredCount,
  trackedCount,
  nextRunText,
  footerStatus,
  children,
}: ShellProps) {
  return (
    <div className="flex flex-col w-screen h-screen overflow-hidden bg-[var(--color-paper)] text-[var(--color-ink)] select-none">
      <TopNav
        discoveredCount={discoveredCount}
        trackedCount={trackedCount}
        nextRunText={nextRunText}
      />
      <SubNav />
      <div className="flex-1 overflow-hidden flex flex-col">{children}</div>
      <Footer statusMessage={footerStatus} />
      <ToastContainer />
    </div>
  );
}
