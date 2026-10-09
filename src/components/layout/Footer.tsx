import { useUIStore } from "../../lib/store";
import { KeyHint } from "../ui/KeyHint";

interface FooterProps {
  statusMessage?: string;
}

export function Footer({ statusMessage }: FooterProps) {
  const { view, undoStack } = useUIStore();

  return (
    <footer className="flex items-center justify-between h-8 px-3 border-t border-[var(--color-ink)] bg-[var(--color-paper)] shrink-0 select-none text-xs font-mono text-[var(--color-ink)]">
      {/* Left: Key Hints */}
      <div className="flex items-center gap-3 overflow-x-auto py-1">
        {view === "discovered" && (
          <>
            <KeyHint keys="↵" label="OPEN" />
            <KeyHint keys="A" label="APPLY" />
            <KeyHint keys="X" label="IGNORE" />
            <KeyHint keys="V" label="VIEW" />
            <KeyHint keys="J/K" label="NAV" />
            {undoStack.length > 0 && <KeyHint keys="U" label="UNDO" />}
            <KeyHint keys="Q/E" label="TAB" />
            <KeyHint keys="/" label="SEARCH" />
          </>
        )}

        {view === "tracked" && (
          <>
            <KeyHint keys="↵" label="OPEN" />
            <KeyHint keys="⇧+←/→" label="STAGE" />
            <KeyHint keys="J/K" label="NAV" />
            {undoStack.length > 0 && <KeyHint keys="U" label="UNDO" />}
            <KeyHint keys="Q/E" label="TAB" />
            <KeyHint keys="/" label="SEARCH" />
          </>
        )}

        {(view === "settings" || view === "metrics") && (
          <>
            <KeyHint keys="Q/E" label="SWITCH TAB" />
            <KeyHint keys="ESC" label="CLOSE" />
          </>
        )}
      </div>

      {/* Right: Contextual Helper / Status */}
      <div className="text-[11px] text-[var(--color-ink-mute)] uppercase tracking-wider shrink-0 pl-2">
        {statusMessage || ""}
      </div>
    </footer>
  );
}
