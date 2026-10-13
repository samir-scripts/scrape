import type { TriState } from "../../bindings/TriState";

export interface TriStateFilterProps {
  label: string;
  selected: TriState[] | null;
  onChange: (vals: TriState[] | null) => void;
  description?: string;
}

const OPTIONS: { val: TriState; label: string }[] = [
  { val: "yes", label: "YES" },
  { val: "no", label: "NO" },
  { val: "unknown", label: "N/A" },
];

export function toggleTriStateSelection(
  current: TriState[] | null,
  val: TriState
): TriState[] | null {
  const currentList = current || [];
  let next: TriState[];
  if (currentList.includes(val)) {
    next = currentList.filter((v) => v !== val);
  } else {
    next = [...currentList, val];
  }
  // If all 3 are selected or none are selected, treat as null (no filter)
  if (next.length === 0 || next.length === 3) {
    return null;
  }
  return next;
}

export function TriStateFilter({
  label,
  selected,
  onChange,
  description,
}: TriStateFilterProps) {
  const currentList = selected || [];

  const handleToggle = (val: TriState) => {
    onChange(toggleTriStateSelection(selected, val));
  };

  const isSelected = (val: TriState) => {
    return currentList.length > 0 && currentList.includes(val);
  };

  return (
    <div className="flex flex-col gap-1.5 font-mono text-xs select-none">
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          {label}
        </label>
        {currentList.length > 0 && (
          <button
            onClick={() => onChange(null)}
            className="text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] uppercase cursor-pointer"
          >
            [ANY]
          </button>
        )}
      </div>

      {description && (
        <span className="text-xs text-[var(--color-ink-mute)]">
          {description}
        </span>
      )}

      <div className="flex gap-2">
        {OPTIONS.map((opt) => {
          const active = isSelected(opt.val);
          return (
            <button
              key={opt.val}
              type="button"
              onClick={() => handleToggle(opt.val)}
              className={`flex-1 py-1 px-2 text-xs font-bold uppercase tracking-wider border cursor-pointer ${
                active
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              {opt.label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
