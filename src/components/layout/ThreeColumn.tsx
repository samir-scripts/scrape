import type { ReactNode } from "react";
import { Rule } from "../ui/Rule";

interface ThreeColumnProps {
  left: ReactNode;
  center: ReactNode;
  right: ReactNode;
  className?: string;
}

export function ThreeColumn({
  left,
  center,
  right,
  className = "",
}: ThreeColumnProps) {
  return (
    <div className={`flex flex-1 w-full h-full overflow-hidden ${className}`}>
      {/* Left Column: Job List */}
      <aside className="w-80 shrink-0 h-full flex flex-col overflow-y-auto bg-[var(--color-paper)]">
        {left}
      </aside>

      <Rule orientation="vertical" />

      {/* Center Column: Job Detail */}
      <main className="flex-1 h-full flex flex-col overflow-y-auto bg-[var(--color-paper)] p-6">
        {center}
      </main>

      <Rule orientation="vertical" />

      {/* Right Column: Metadata / Actions */}
      <aside className="w-72 shrink-0 h-full flex flex-col overflow-y-auto bg-[var(--color-paper-2)]/30 p-4">
        {right}
      </aside>
    </div>
  );
}
