import type { MouseEvent } from "react";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";

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
  const defaultTitle = favorited
    ? `Favorited (Protected from pruning) [${shortcutHint}]`
    : `Favorite posting [${shortcutHint}]`;

  return (
    <IconButton
      data-control-action="job.favorite.toggle"
      label={favorited ? "Unfavorite posting" : "Favorite posting"}
      onClick={(e) => { e.stopPropagation(); onToggle?.(e); }}
      aria-pressed={favorited}
      title={title || defaultTitle}
      className={className}
    >
      <Icon name="star" size={size} style={{ fill: favorited ? "var(--color-lime)" : "none" }} />
    </IconButton>
  );
}
