export interface NewGradFilterProps {
  value: string | null; // "any" | "only" | "exclude" | null
  onChange: (val: string) => void;
}

export function NewGradFilter({ value = "any", onChange }: NewGradFilterProps) {
  const current = value || "any";

  const options: { id: string; label: string }[] = [
    { id: "any", label: "ANY POSTING" },
    { id: "only", label: "NEW GRAD ONLY" },
    { id: "exclude", label: "EXCLUDE NEW GRAD" },
  ];

  return (
    <div className="flex flex-col gap-2 font-mono text-xs select-none">
      <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
        NEW GRAD / ENTRY-LEVEL STATUS
      </label>
      <div className="flex gap-1.5">
        {options.map((opt) => {
          const active = current === opt.id;
          return (
            <button
              key={opt.id}
              type="button"
              onClick={() => onChange(opt.id)}
              className={`flex-1 py-1 px-2 text-[11px] font-bold uppercase tracking-wider border cursor-pointer ${
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
