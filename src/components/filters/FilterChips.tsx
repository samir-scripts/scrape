import { Icon } from "../ui/Icon";
import { IconButton } from "../ui/IconButton";
import type { JobFilter } from "../../bindings/JobFilter";
import { ROLE_TYPE_LABELS } from "../../lib/constants";

export interface FilterChipsProps {
  filter: Partial<JobFilter>;
  onUpdate: (updater: (prev: Partial<JobFilter>) => Partial<JobFilter>) => void;
  onClearAll: () => void;
}

export function FilterChips({ filter, onUpdate, onClearAll }: FilterChipsProps) {
  const chips: { id: string; label: string; onRemove: () => void }[] = [];

  // Contains (Keywords / Text)
  if (filter.search && filter.search.trim()) {
    chips.push({
      id: "search",
      label: `CONTAINS: "${filter.search.trim().toUpperCase()}"`,
      onRemove: () => onUpdate((prev) => ({ ...prev, search: null })),
    });
  }

  // Salary / Compensation
  if (filter.salary_min != null || filter.salary_max != null || filter.salary_interval) {
    if (filter.salary_interval === "hour") {
      const minHr = filter.salary_min != null
        ? `${filter.salary_currency || "USD"} ${filter.salary_min}`
        : "No minimum";
      const maxHr = filter.salary_max != null
        ? `${filter.salary_currency || "USD"} ${filter.salary_max}`
        : "No maximum";
      chips.push({
        id: "salary",
        label: `RATE: ${minHr} - ${maxHr}/HR`,
        onRemove: () =>
          onUpdate((prev) => ({
            ...prev,
            salary_min: null,
            salary_max: null,
            salary_interval: null,
          })),
      });
    } else if (filter.salary_min != null || filter.salary_max != null) {
      const minText = filter.salary_min != null ? `$${(filter.salary_min / 1000).toFixed(0)}K` : "No minimum";
      const maxText = filter.salary_max != null ? `$${(filter.salary_max / 1000).toFixed(0)}K` : "No maximum";
      chips.push({
        id: "salary",
        label: `SALARY: ${minText} - ${maxText}`,
        onRemove: () =>
          onUpdate((prev) => ({
            ...prev,
            salary_min: null,
            salary_max: null,
            salary_interval: null,
          })),
      });
    }
  }

  // Experience (YOE)
  if (filter.experience_min != null || filter.experience_max != null) {
    let expLabel = "YOE: ";
    if (filter.experience_min != null && filter.experience_max != null) {
      expLabel += `${filter.experience_min}-${filter.experience_max} YRS`;
    } else if (filter.experience_min != null) {
      expLabel += `${filter.experience_min}+ YRS`;
    } else {
      expLabel += `≤${filter.experience_max} YRS`;
    }
    chips.push({
      id: "experience",
      label: expLabel,
      onRemove: () =>
        onUpdate((prev) => ({ ...prev, experience_min: null, experience_max: null })),
    });
  }

  // New Grad
  if (filter.new_grad && filter.new_grad !== "any") {
    chips.push({
      id: "new_grad",
      label: filter.new_grad === "only" ? "NEW GRAD ONLY" : "EXCLUDE NEW GRAD",
      onRemove: () => onUpdate((prev) => ({ ...prev, new_grad: "any" })),
    });
  }

  // Sponsorship
  if (filter.sponsorship && filter.sponsorship.length > 0) {
    chips.push({
      id: "sponsorship",
      label: `SPON: ${filter.sponsorship.join("/").toUpperCase()}`,
      onRemove: () => onUpdate((prev) => ({ ...prev, sponsorship: null })),
    });
  }

  // Clearance
  if (filter.clearance && filter.clearance.length > 0) {
    chips.push({
      id: "clearance",
      label: `CLR: ${filter.clearance.join("/").toUpperCase()}`,
      onRemove: () => onUpdate((prev) => ({ ...prev, clearance: null })),
    });
  }

  // Remote
  if (filter.remote && filter.remote !== "all") {
    chips.push({
      id: "remote",
      label: `WORKPLACE: ${filter.remote.toUpperCase()}`,
      onRemove: () => onUpdate((prev) => ({ ...prev, remote: "all" })),
    });
  }

  // Countries
  if (filter.countries && filter.countries.length > 0) {
    chips.push({
      id: "countries",
      label: `GEO: ${filter.countries.join(", ")}`,
      onRemove: () => onUpdate((prev) => ({ ...prev, countries: null })),
    });
  }

  // Regions
  if (filter.regions && filter.regions.length > 0) {
    chips.push({
      id: "regions",
      label: `REG: ${filter.regions.join(", ")}`,
      onRemove: () => onUpdate((prev) => ({ ...prev, regions: null })),
    });
  }

  // Role Types
  if (filter.role_types && filter.role_types.length > 0) {
    const roleLabels = filter.role_types
      .map((r) => ROLE_TYPE_LABELS[r] || r.toUpperCase())
      .join(", ");
    chips.push({
      id: "role_types",
      label: `ROLES: ${roleLabels}`,
      onRemove: () => onUpdate((prev) => ({ ...prev, role_types: null })),
    });
  }

  if (chips.length === 0) return null;

  return (
    <div className="flex items-center flex-wrap gap-1.5 px-3 py-1 bg-[var(--color-paper-2)]/30 border-b border-[var(--color-ink)]/30 select-none">
      <span className="text-xs font-mono font-bold text-[var(--color-ink-mute)] uppercase tracking-wider mr-1">
        ACTIVE:
      </span>
      {chips.map((chip) => (
        <span
          key={chip.id}
          className="inline-flex items-center gap-1.5 px-2 py-0.5 border border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] text-xs font-mono font-bold uppercase tracking-wider"
        >
          <span>{chip.label}</span>
          <IconButton label={`Remove filter ${chip.label}`} onClick={chip.onRemove}>
            <Icon name="close" size="sm" />
          </IconButton>
        </span>
      ))}
      <button
        type="button"
        onClick={onClearAll}
        className="px-2 py-0.5 text-xs font-mono uppercase text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] underline cursor-pointer ml-1"
      >
        [RESET ALL]
      </button>
    </div>
  );
}
