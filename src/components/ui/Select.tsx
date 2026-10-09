import type { SelectHTMLAttributes } from "react";

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  options: { value: string; label: string }[];
}

export function Select({ options, className = "", ...props }: SelectProps) {
  return (
    <select
      className={`px-2.5 py-1.5 text-xs font-mono uppercase bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] focus:bg-[var(--color-paper-2)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] cursor-pointer ${className}`}
      {...props}
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}
