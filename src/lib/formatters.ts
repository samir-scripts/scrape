export type TimeFormat = "12h" | "24h";
export type DateFormat = "MM/DD/YY" | "DD/MM/YY" | "MM/DD/YYYY" | "DD/MM/YYYY" | "YYYY-MM-DD";

export interface FormatOptions {
  timeFormat?: string | null;
  dateFormat?: string | null;
  system_time_format?: string | null;
  system_date_format?: string | null;
}

export function formatDate(
  isoOrDate: string | number | Date | null | undefined,
  options?: FormatOptions | string | null
): string {
  if (!isoOrDate) return "—";
  const d = typeof isoOrDate === "object" ? isoOrDate : new Date(isoOrDate);
  if (isNaN(d.getTime())) return String(isoOrDate);

  const formatStr =
    typeof options === "string"
      ? options
      : options?.dateFormat || options?.system_date_format || "MM/DD/YY";

  const yyyy = d.getFullYear().toString();
  const yy = yyyy.slice(-2);
  const mm = (d.getMonth() + 1).toString().padStart(2, "0");
  const dd = d.getDate().toString().padStart(2, "0");

  switch (formatStr) {
    case "DD/MM/YY":
      return `${dd}/${mm}/${yy}`;
    case "DD/MM/YYYY":
      return `${dd}/${mm}/${yyyy}`;
    case "MM/DD/YYYY":
      return `${mm}/${dd}/${yyyy}`;
    case "YYYY-MM-DD":
      return `${yyyy}-${mm}-${dd}`;
    case "MM/DD/YY":
    default:
      return `${mm}/${dd}/${yy}`;
  }
}

export function formatTime(
  isoOrDate: string | number | Date | null | undefined,
  options?: FormatOptions | string | null,
  includeSeconds = false
): string {
  if (!isoOrDate) return "—";
  const d = typeof isoOrDate === "object" ? isoOrDate : new Date(isoOrDate);
  if (isNaN(d.getTime())) return String(isoOrDate);

  const timeFormatStr =
    typeof options === "string"
      ? options
      : options?.timeFormat || options?.system_time_format || "12h";

  const is24h = timeFormatStr === "24h";
  const hours = d.getHours();
  const minutes = d.getMinutes().toString().padStart(2, "0");
  const seconds = d.getSeconds().toString().padStart(2, "0");
  const secPart = includeSeconds ? `:${seconds}` : "";

  if (is24h) {
    return `${hours.toString().padStart(2, "0")}:${minutes}${secPart}`;
  } else {
    const ampm = hours >= 12 ? "PM" : "AM";
    const h12 = hours % 12 || 12;
    return `${h12.toString().padStart(2, "0")}:${minutes}${secPart} ${ampm}`;
  }
}

export function formatDateTime(
  isoOrDate: string | number | Date | null | undefined,
  options?: FormatOptions | null,
  includeSeconds = false
): string {
  if (!isoOrDate) return "—";
  return `${formatDate(isoOrDate, options)} ${formatTime(isoOrDate, options, includeSeconds)}`;
}
