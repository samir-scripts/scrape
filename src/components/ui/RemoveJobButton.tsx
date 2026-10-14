import type { MouseEvent } from "react";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";

export function RemoveJobButton({ title, company, label, onClick }: {
  title: string;
  company: string;
  label?: string;
  onClick: (event: MouseEvent<HTMLButtonElement>) => void;
}) {
  return <IconButton label={`Remove ${title} at ${company}`} title="Remove saved job" variant="danger" onClick={onClick}>
    <Icon name="trash" />
    {label && <span className="text-xs">{label}</span>}
  </IconButton>;
}
