import type { JobSort } from "../bindings/JobSort";

export const JOB_SORT_OPTIONS: { value: JobSort; label: string }[] = [
  { value: "status_changed_at_desc", label: "Recent stage activity" },
  { value: "date_added_desc", label: "Newest discoveries" },
  { value: "date_added_asc", label: "Oldest discoveries" },
  { value: "date_posted_desc", label: "Newest postings" },
  { value: "company_asc", label: "Company A to Z" },
  { value: "title_asc", label: "Title A to Z" },
  { value: "salary_desc", label: "Highest derived annual pay" },
  { value: "favorited_desc", label: "Favorites first" },
];

export function contextualJobSort(view: "discovered" | "tracked"): JobSort {
  return view === "tracked" ? "status_changed_at_desc" : "date_added_desc";
}

export function jobSortLabel(sort: JobSort): string {
  return JOB_SORT_OPTIONS.find(option => option.value === sort)?.label ?? sort.replaceAll("_", " ");
}
