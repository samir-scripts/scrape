import type { InputHTMLAttributes, Ref } from "react";

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  ref?: Ref<HTMLInputElement>;
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
      className={`normal-case w-full min-h-[var(--control-height)] px-3 py-2 text-sm bg-[var(--color-paper)] text-[var(--color-ink)] border border-[var(--color-ink)] placeholder-[var(--color-ink-mute)] focus:bg-[var(--color-paper-2)] focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] ${fontClass} ${className}`}
      {...props}
    />
  );
}
