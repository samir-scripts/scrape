import type { MouseEvent } from "react";

export interface FavoriteButtonProps {
  favorited: boolean;
  onToggle?: (e: MouseEvent<HTMLButtonElement>) => void;
  size?: "sm" | "md" | "lg";
  shortcutHint?: string;
  className?: string;
  title?: string;
}

export function FavoriteButton({
  favorited,
  onToggle,
  size = "md",
  shortcutHint = "S",
  className = "",
  title,
}: FavoriteButtonProps) {
  const sizeClasses = {
    sm: "w-5 h-5",
    md: "w-6 h-6",
    lg: "w-7 h-7",
  }[size];

  const svgSizes = {
    sm: "w-3.5 h-3.5",
    md: "w-4.5 h-4.5",
    lg: "w-5.5 h-5.5",
  }[size];

  const defaultTitle = favorited
    ? `Favorited (Protected from pruning) [${shortcutHint}]`
    : `Favorite posting [${shortcutHint}]`;

  return (
    <button
      type="button"
      onClick={(e) => {
        e.stopPropagation();
        onToggle?.(e);
      }}
      aria-pressed={favorited}
      aria-label={favorited ? "Unfavorite posting" : "Favorite posting"}
      title={title || defaultTitle}
      className={`inline-flex items-center justify-center cursor-pointer transition-colors focus:outline-none focus:ring-1 focus:ring-[var(--color-ink)] shrink-0 group ${sizeClasses} ${className}`}
    >
      <svg
        className={`${svgSizes} transition-transform group-hover:scale-110 active:scale-95`}
        viewBox="0 0 24 24"
        fill={favorited ? "var(--color-lime)" : "none"}
        stroke={favorited ? "var(--color-ink)" : "currentColor"}
        strokeWidth={favorited ? "1.8" : "1.6"}
        strokeLinecap="square"
        strokeLinejoin="miter"
      >
        <polygon points="12,2 15.09,8.26 22,9.27 17,14.14 18.18,21.02 12,17.77 5.82,21.02 7,14.14 2,9.27 8.91,8.26" />
      </svg>
    </button>
  );
}
