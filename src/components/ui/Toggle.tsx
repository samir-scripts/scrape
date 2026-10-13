interface ToggleProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label?: string;
  description?: string;
  disabled?: boolean;
  size?: "default" | "sm";
  className?: string;
}

export function Toggle({
  checked,
  onChange,
  label,
  description,
  disabled = false,
  size = "default",
  className = "",
}: ToggleProps) {
  const isSm = size === "sm";

  return (
    <div
      role="switch"
      aria-checked={checked}
      aria-disabled={disabled}
      tabIndex={disabled ? -1 : 0}
      onClick={() => !disabled && onChange(!checked)}
      onKeyDown={(e) => {
        if (!disabled && (e.key === "Enter" || e.key === " ")) {
          e.preventDefault();
          onChange(!checked);
        }
      }}
      className={`inline-flex items-center gap-2.5 cursor-pointer select-none focus:outline-none focus-visible:ring-1 focus-visible:ring-[var(--color-ink)] ${
        disabled ? "opacity-50 cursor-not-allowed" : ""
      } ${className}`}
    >
      {/* Brutalist Mechanical Track */}
      <div
        className={`${
          isSm ? "w-7 h-3.5" : "w-8 h-4"
        } relative border border-[var(--color-ink)] transition-colors duration-150 shrink-0 ${
          checked
            ? "bg-[var(--color-ink)]"
            : "bg-[var(--color-paper-2)] hover:bg-[var(--color-paper-2)]/80"
        }`}
      >
        {/* Brutalist Sharp Square Thumb / Slider */}
        <div
          className={`${
            isSm ? "w-2 h-2 top-[2px] left-[2px]" : "w-2.5 h-2.5 top-[2px] left-[2px]"
          } absolute transition-transform duration-150 ease-out ${
            checked
              ? `${isSm ? "translate-x-3" : "translate-x-3.5"} bg-[var(--color-paper)]`
              : "translate-x-0 bg-[var(--color-ink)]"
          }`}
        />
      </div>

      {(label || description) && (
        <div className="flex flex-col">
          {label && (
            <span className="text-xs font-mono font-bold uppercase tracking-wider text-[var(--color-ink)]">
              {label}
            </span>
          )}
          {description && (
            <span className="text-xs text-[var(--color-ink-mute)]">
              {description}
            </span>
          )}
        </div>
      )}
    </div>
  );
}
