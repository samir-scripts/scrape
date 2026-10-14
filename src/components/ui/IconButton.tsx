import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";

export const IconButton = forwardRef<HTMLButtonElement, ButtonHTMLAttributes<HTMLButtonElement> & {
  label: string;
  children: ReactNode;
  variant?: "default" | "outlined" | "danger" | "inverse";
}>(function IconButton({ label, children, className = "", variant = "default", ...props }, ref) {
  const variants = {
    default: "border-transparent enabled:hover:bg-[var(--color-paper-2)]",
    outlined: "border-current enabled:hover:bg-[var(--color-paper-2)]",
    danger: "border-current text-[var(--color-danger)] enabled:hover:bg-[var(--color-paper-2)]",
    inverse: "border-transparent text-[var(--color-paper)] enabled:hover:border-current focus-visible:outline-[var(--color-paper)]",
  };
  return <button type="button" title={label} {...props} ref={ref} aria-label={label}
    className={`ui-icon-button inline-flex items-center justify-center gap-2 border shrink-0 cursor-pointer transition-colors disabled:opacity-40 disabled:cursor-not-allowed ${variants[variant]} ${className}`}>
    {children}
  </button>;
});
