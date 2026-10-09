interface DitherProps {
  type?: "dots" | "checker" | "lines";
  width?: string | number;
  height?: string | number;
  className?: string;
}

export function Dither({
  type = "dots",
  width = "100%",
  height = "16px",
  className = "",
}: DitherProps) {
  const utilityClass =
    type === "checker"
      ? "dither-checker"
      : type === "lines"
      ? "dither-lines"
      : "dither-dots";

  return (
    <div
      style={{ width, height }}
      className={`text-[var(--color-ink-mute)] opacity-50 shrink-0 ${utilityClass} ${className}`}
    />
  );
}
