import type { ButtonHTMLAttributes, ReactNode } from "react";
export function IconButton({
  label,
  children,
  className = "",
  ...props
}: ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
}) {
  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      className={`inline-flex min-w-8 min-h-8 items-center justify-center cursor-pointer hover:bg-[var(--color-paper-2)] shrink-0 ${className}`}
      {...props}
    >
      {children}
    </button>
  );
}
