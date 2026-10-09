interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  description?: string;
  disabled?: boolean;
  className?: string;
}

export function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  className = "",
}: ToggleProps) {
  return (
    <label
      className={`inline-flex items-start gap-2.5 cursor-pointer select-none ${
        disabled ? "opacity-50 cursor-not-allowed" : ""
      } ${className}`}
    >
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => !disabled && onChange(!checked)}
        className={`w-4 h-4 mt-0.5 border border-[var(--color-ink)] flex items-center justify-center font-mono text-xs leading-none shrink-0 cursor-pointer ${
          checked
            ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
            : "bg-transparent text-transparent"
        }`}
      >
        {checked ? "■" : "□"}
      </button>
      {(label || description) && (
        <div className="flex flex-col">
          {label && (
            <span className="text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
              {label}
            </span>
          )}
          {description && (
            <span className="text-[11px] text-[var(--color-ink-mute)]">
              {description}
            </span>
          )}
        </div>
      )}
    </label>
  );
}
