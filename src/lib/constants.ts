import type { JobStatus } from "../bindings/JobStatus";
import type { RoleType } from "../bindings/RoleType";

export interface StatusStyle {
  label: string;
  bg: string;
  text: string;
  border: string;
  dither?: "checker" | "dots" | "lines";
}

export const STATUS_CONFIG: Record<JobStatus, StatusStyle> = {
  scraped: {
    label: "SCRAPED",
    bg: "var(--color-lavender)",
    text: "var(--color-ink)",
    border: "var(--color-ink)",
  },
  viewed: {
    label: "VIEWED",
    bg: "var(--color-cream)",
    text: "var(--color-ink)",
    border: "var(--color-ink)",
  },
  applied: {
    label: "APPLIED",
    bg: "var(--color-lime)",
    text: "var(--color-ink)",
    border: "var(--color-ink)",
  },
  interviewing: {
    label: "INTERVIEWING",
    bg: "var(--color-signal)",
    text: "#FFFFFF",
    border: "var(--color-signal)",
  },
  offer: {
    label: "OFFER",
    bg: "var(--color-ink)",
    text: "var(--color-paper)",
    border: "var(--color-ink)",
  },
  rejected: {
    label: "REJECTED",
    bg: "var(--color-cherry)",
    text: "#FFFFFF",
    border: "var(--color-cherry)",
  },
  ignored: {
    label: "IGNORED",
    bg: "transparent",
    text: "var(--color-ink-mute)",
    border: "var(--color-ink-mute)",
    dither: "checker",
  },
};

export const TRACKED_STAGES: JobStatus[] = [
  "applied",
  "interviewing",
  "offer",
  "rejected",
];

export const ROLE_TYPE_LABELS: Record<string, string> = {
  all: "ALL",
  full_time: "FULL-TIME",
  contract: "CONTRACT",
  internship: "INTERNSHIP",
  part_time: "PART-TIME",
  unknown: "OTHER",
};

export const ROLE_TYPES: (RoleType | "all")[] = [
  "all",
  "full_time",
  "contract",
  "internship",
  "part_time",
];

export const CRON_PRESETS = [
  { label: "HOURLY", expr: "0 0 * * * *" },
  { label: "EVERY 6 HOURS", expr: "0 0 */6 * * *" },
  { label: "DAILY 09:00", expr: "0 0 9 * * *" },
  { label: "WEEKDAYS 09:00", expr: "0 0 9 * * 1-5" },
];

export const NAV_TABS = [
  { id: "discovered", label: "DISCOVERED" },
  { id: "tracked", label: "TRACKED" },
  { id: "metrics", label: "METRICS" },
  { id: "settings", label: "SETTINGS" },
] as const;

export type NavTabId = (typeof NAV_TABS)[number]["id"];
