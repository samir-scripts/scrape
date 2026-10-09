import type { InputHTMLAttributes } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  variant?: "default" | "mono";
}

export function Input({
  variant = "default",
  className = "",
  ...props
}: InputProps) {
  const fontClass = variant === "mono" ? "font-mono" : "font-sans";

  return (
    <input
      className={`w-full px-2.5 py-1.5 text-xs bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:bg-[var(--color-paper-2)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] ${fontClass} ${className}`}
      {...props}
    />
  );
}
