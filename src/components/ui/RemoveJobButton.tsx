import type { MouseEvent } from "react";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";

export function RemoveJobButton({ title, company, label, variant = "danger", onClick }: {
  title: string;
  company: string;
  label?: string;
  variant?: "danger" | "ghost-danger";
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return <IconButton label={`Remove ${title} at ${company}`} title="Remove saved job" variant={variant} onClick={onClick}>
    <Icon name="trash" />
    {label && <span className="text-xs">{label}</span>}
  </IconButton>;
}
