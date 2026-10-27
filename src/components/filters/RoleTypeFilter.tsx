import type { RoleType } from "../../bindings/RoleType";
import { ROLE_TYPE_LABELS } from "../../lib/constants";

export interface RoleTypeFilterProps {
  selected: RoleType[] | null;
  onChange: (roles: RoleType[] | null) => void;
}

const ALL_ROLES: RoleType[] = [
  "full_time",
  "contract",
  "internship",
  "part_time",
  "seasonal",
  "unknown",
];

export function toggleRoleSelection(
  current: RoleType[] | null,
  role: RoleType
): RoleType[] | null {
  const currentList = current || [];
  let next: RoleType[];
  if (currentList.includes(role)) {
    next = currentList.filter((r) => r !== role);
  } else {
    next = [...currentList, role];
  }
  // If all or none are selected, treat as null (no filter)
  if (next.length === 0 || next.length === ALL_ROLES.length) {
    return null;
  }
  return next;
}

export function RoleTypeFilter({ selected, onChange }: RoleTypeFilterProps) {
  const currentList = selected || [];

  const handleToggle = (role: RoleType) => {
    onChange(toggleRoleSelection(selected, role));
  };

  const isSelected = (role: RoleType) => {
    return currentList.length > 0 && currentList.includes(role);
  };

  return (
    <div className="flex flex-col gap-1.5 font-mono text-xs select-none">
      <div className="flex justify-between items-center">
        <label className="font-bold uppercase tracking-wider text-[var(--color-ink)]">
          EMPLOYMENT TYPE / ROLE
        </label>
        {currentList.length > 0 && (
          <button
            data-control-action="filters.role.clear" onClick={() => onChange(null)}
            className="text-xs text-[var(--color-ink-mute)] hover:text-[var(--color-danger)] uppercase cursor-pointer"
          >
            [ANY ROLE]
          </button>
        )}
      </div>

      <div className="grid grid-cols-2 sm:grid-cols-3 gap-1.5">
        {ALL_ROLES.map((role) => {
          const active = isSelected(role);
          const label = ROLE_TYPE_LABELS[role] || role.toUpperCase();
          return (
            <button
              data-control-action="filters.role.toggle" key={role}
              type="button"
              onClick={() => handleToggle(role)}
              className={`py-1 px-2 text-xs font-bold uppercase tracking-wider border cursor-pointer truncate ${
                active
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)] border-[var(--color-ink)]"
                  : "bg-[var(--color-paper)] text-[var(--color-ink)] border-[var(--color-ink)] hover:bg-[var(--color-paper-2)]"
              }`}
            >
              {label}
            </button>
          );
        })}
      </div>
    </div>
  );
}
