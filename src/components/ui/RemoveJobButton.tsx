import type { MouseEvent } from "react";
export function RemoveJobButton({
  title,
  company,
  label,
  onClick,
}: {
  title: string;
  company: string;
  label?: string;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return (
    <button
      type="button"
      aria-label={`Remove ${title} at ${company}`}
      title="Remove saved job"
      onClick={onClick}
      className="inline-flex min-w-8 min-h-8 items-center justify-center border border-current text-[var(--color-danger)] hover:bg-[var(--color-paper-2)] cursor-pointer shrink-0"
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
        <path d="M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7M14 10v7" />
      </svg>
      {label && <span className="px-2 text-xs font-mono">{label}</span>}
    </button>
  );
}
