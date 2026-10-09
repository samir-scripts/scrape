interface RuleProps {
  orientation?: "horizontal" | "vertical";
  double?: boolean;
  className?: string;
}

export function Rule({
  orientation = "horizontal",
  double = false,
  className = "",
}: RuleProps) {
  if (orientation === "vertical") {
    return (
      <div
        className={`w-px h-full bg-[var(--color-ink)] shrink-0 ${
          double ? "rule-double" : ""
        } ${className}`}
      />
    );
  }

  return (
    <hr
      className={`border-0 border-t border-[var(--color-ink)] w-full my-0 shrink-0 ${
        double ? "border-t-[3px] border-double border-[var(--color-signal)]" : ""
      } ${className}`}
    />
  );
}
