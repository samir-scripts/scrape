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
    text: "var(--color-accent-ink)",
    border: "var(--color-ink)",
  },
  viewed: {
    label: "VIEWED",
    bg: "var(--color-cream)",
    text: "var(--color-accent-ink)",
    border: "var(--color-ink)",
  },
  applied: {
    label: "APPLIED",
    bg: "var(--color-lime)",
    text: "var(--color-accent-ink)",
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
    bg: "var(--color-danger)",
    text: "var(--color-paper)",
    border: "var(--color-danger)",
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
  seasonal: "SEASONAL",
  unknown: "OTHER",
};

export const ROLE_TYPES: (RoleType | "all")[] = [
  "all",
  "full_time",
  "contract",
  "internship",
  "part_time",
  "seasonal",
];

export const CRON_PRESETS = [
  { label: "HOURLY", expr: "0 0 * * * *" },
  { label: "EVERY 6 HOURS", expr: "0 0 */6 * * *" },
  { label: "DAILY 09:00", expr: "0 0 9 * * *" },
  { label: "WEEKDAYS 09:00", expr: "0 0 9 * * MON-FRI" },
];

export const NAV_TABS = [
  { id: "discovered", label: "DISCOVERED" },
  { id: "tracked", label: "TRACKED" },
  { id: "metrics", label: "METRICS" },
  { id: "scraper", label: "SCRAPER" },
  { id: "settings", label: "SETTINGS" },
] as const;

export type NavTabId = (typeof NAV_TABS)[number]["id"];

export type FontFamilyId =
  | "default"
  | "inter"
  | "jetbrains"
  | "geist"
  | "fira_code"
  | "ibm_plex";

export interface FontFamilyOption {
  id: FontFamilyId;
  label: string;
  name: string;
  category: "readable" | "monospace";
  description: string;
  fontFamily: string;
  previewText: string;
}

export const FONT_OPTIONS: FontFamilyOption[] = [
  {
    id: "default",
    label: "DEFAULT",
    name: "SF Mono / System",
    category: "monospace",
    description: "Classic NieR brutalist monospace typography.",
    fontFamily: '"SF Mono", Menlo, Monaco, Consolas, monospace',
    previewText: "STATUS: SCRAPED // 127.0.0.1 // [APPLY]",
  },
  {
    id: "inter",
    label: "INTER",
    name: "Inter (Readable Sans)",
    category: "readable",
    description: "Highly legible sans-serif designed for supreme readability and zero eye fatigue.",
    fontFamily: '"Inter", -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif',
    previewText: "Senior Distributed Systems Engineer · Fully Remote",
  },
  {
    id: "jetbrains",
    label: "JETBRAINS MONO",
    name: "JetBrains Mono",
    category: "monospace",
    description: "Developer favorite with open counters and distinct code glyphs.",
    fontFamily: '"JetBrains Mono", "SF Mono", monospace',
    previewText: "fn triage_postings(queue: &mut Vec<Job>) -> Result<()>",
  },
  {
    id: "geist",
    label: "GEIST MONO",
    name: "Geist Mono",
    category: "monospace",
    description: "Modern minimalist developer typography by Vercel.",
    fontFamily: '"Geist Mono", "SF Mono", monospace',
    previewText: "export const RETENTION_TTL = 90_DAYS;",
  },
  {
    id: "fira_code",
    label: "FIRA CODE",
    name: "Fira Code",
    category: "monospace",
    description: "Iconic programming font with high-clarity punctuation.",
    fontFamily: '"Fira Code", Menlo, monospace',
    previewText: "match status { JobStatus::Offer => 100% }",
  },
  {
    id: "ibm_plex",
    label: "IBM PLEX MONO",
    name: "IBM Plex Mono",
    category: "monospace",
    description: "Engineered grotesque monospace blending mechanical rigor with human warmth.",
    fontFamily: '"IBM Plex Mono", Menlo, monospace',
    previewText: "SYSTEM_CADENCE: 0 0 9 * * MON-FRI [ACTIVE]",
  },
];
