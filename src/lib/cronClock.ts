export type ClockCadence =
  | "daily"
  | "weekdays"
  | "weekends"
  | "every_12h"
  | "every_6h"
  | "hourly";

export type AmPm = "AM" | "PM";

export interface ClockScheduleState {
  hour12: number; // 1 - 12
  minute: number; // 0 - 59
  ampm: AmPm;
  cadence: ClockCadence;
}

export interface CadenceDefinition {
  id: ClockCadence;
  label: string;
  description: (hour12: number, minute: number, ampm: AmPm) => string;
}

export function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

export function to24Hour(hour12: number, ampm: AmPm): number {
  const norm = hour12 % 12;
  return ampm === "AM" ? norm : norm + 12;
}

export function from24Hour(hour24: number): { hour12: number; ampm: AmPm } {
  const norm = ((hour24 % 24) + 24) % 24;
  if (norm === 0) {
    return { hour12: 12, ampm: "AM" };
  } else if (norm < 12) {
    return { hour12: norm, ampm: "AM" };
  } else if (norm === 12) {
    return { hour12: 12, ampm: "PM" };
  } else {
    return { hour12: norm - 12, ampm: "PM" };
  }
}

export function formatTime12(hour12: number, minute: number, ampm: AmPm): string {
  return `${pad2(hour12)}:${pad2(minute)} ${ampm}`;
}

export const CLOCK_CADENCE_OPTIONS: CadenceDefinition[] = [
  {
    id: "daily",
    label: "DAILY",
    description: (h, m, ap) => `EVERY DAY AT ${pad2(h)}:${pad2(m)} ${ap}`,
  },
  {
    id: "weekdays",
    label: "WEEKDAYS",
    description: (h, m, ap) => `MON–FRI AT ${pad2(h)}:${pad2(m)} ${ap}`,
  },
  {
    id: "weekends",
    label: "WEEKENDS",
    description: (h, m, ap) => `SAT–SUN AT ${pad2(h)}:${pad2(m)} ${ap}`,
  },
  {
    id: "every_12h",
    label: "EVERY 12 HOURS",
    description: (_h, m, _ap) => `EVERY 12 HOURS AT :${pad2(m)}`,
  },
  {
    id: "every_6h",
    label: "EVERY 6 HOURS",
    description: (_h, m, _ap) => `EVERY 6 HOURS AT :${pad2(m)}`,
  },
  {
    id: "hourly",
    label: "HOURLY",
    description: (_h, m, _ap) => `HOURLY AT :${pad2(m)}`,
  },
];

/**
 * Converts clock time and cadence into a valid 6-field tokio-cron expression.
 * Format: sec min hour dom month dow
 */
export function clockToCron(
  hour12: number,
  minute: number,
  ampm: AmPm,
  cadence: ClockCadence
): string {
  const h24 = to24Hour(hour12, ampm);
  const m = Math.min(59, Math.max(0, Math.floor(minute)));

  switch (cadence) {
    case "daily":
      return `0 ${m} ${h24} * * *`;
    case "weekdays":
      return `0 ${m} ${h24} * * 1-5`;
    case "weekends":
      return `0 ${m} ${h24} * * 6,7`;
    case "every_12h":
      return `0 ${m} */12 * * *`;
    case "every_6h":
      return `0 ${m} */6 * * *`;
    case "hourly":
      return `0 ${m} * * * *`;
  }
}

/**
 * Parses a cron expression into ClockScheduleState.
 * Supports standard 6-field format (sec min hour dom mon dow)
 * as well as standard 5-field and 7-field formats.
 * Returns null if the expression does not match clock-supported patterns.
 */
export function cronToClock(cron: string): ClockScheduleState | null {
  if (!cron || typeof cron !== "string") return null;
  const parts = cron.trim().split(/\s+/);
  if (parts.length < 5 || parts.length > 7) {
    return null;
  }

  let secPart = "0";
  let minPart: string;
  let hourPart: string;
  let domPart: string;
  let monthPart: string;
  let dowPart: string;

  if (parts.length === 5) {
    [minPart, hourPart, domPart, monthPart, dowPart] = parts;
  } else {
    // 6 or 7 fields
    [secPart, minPart, hourPart, domPart, monthPart, dowPart] = parts;
  }

  // Seconds must be 0 for scheduled clock triggers
  if (secPart !== "0") {
    return null;
  }

  // Day of month and month must be wildcard for clock cadences
  if (domPart !== "*" && domPart !== "?") {
    return null;
  }
  if (monthPart !== "*") {
    return null;
  }

  if (!/^\d+$/.test(minPart)) {
    return null;
  }
  const minute = Number(minPart);
  if (minute < 0 || minute > 59) {
    return null;
  }

  const dowNorm = dowPart ? dowPart.toUpperCase() : "*";

  // Hourly
  if (hourPart === "*" || hourPart === "*/1") {
    if (dowNorm !== "*" && dowNorm !== "?") return null;
    return {
      hour12: 12,
      minute,
      ampm: "AM",
      cadence: "hourly",
    };
  }

  // Every 6 hours
  if (hourPart === "*/6") {
    if (dowNorm !== "*" && dowNorm !== "?") return null;
    return {
      hour12: 6,
      minute,
      ampm: "AM",
      cadence: "every_6h",
    };
  }

  // Every 12 hours
  if (hourPart === "*/12") {
    if (dowNorm !== "*" && dowNorm !== "?") return null;
    return {
      hour12: 12,
      minute,
      ampm: "PM",
      cadence: "every_12h",
    };
  }

  // Handle two-hour list with 12h offset (e.g. "9,21" or "0,12")
  if (hourPart.includes(",")) {
    if (dowNorm !== "*" && dowNorm !== "?") return null;
    const hours = hourPart
      .split(",")
      .map((s) => Number(s))
      .filter((n) => !isNaN(n));
    if (hours.length === 2 && Math.abs(hours[1] - hours[0]) === 12) {
      const minH = Math.min(hours[0], hours[1]);
      const { hour12, ampm } = from24Hour(minH);
      return {
        hour12,
        minute,
        ampm,
        cadence: "every_12h",
      };
    }
    return null;
  }

  // Single hour integer
  if (/^\d+$/.test(hourPart)) {
    const hourNum = Number(hourPart);
    if (hourNum < 0 || hourNum > 23) {
      return null;
    }
    const { hour12, ampm } = from24Hour(hourNum);

    if (dowNorm === "*" || dowNorm === "?") {
      return {
        hour12,
        minute,
        ampm,
        cadence: "daily",
      };
    }

    if (dowNorm === "1-5" || dowNorm === "MON-FRI") {
      return {
        hour12,
        minute,
        ampm,
        cadence: "weekdays",
      };
    }

    if (
      dowNorm === "6,7" ||
      dowNorm === "7,6" ||
      dowNorm === "SAT,SUN" ||
      dowNorm === "SUN,SAT" ||
      dowNorm === "0,6" ||
      dowNorm === "6,0"
    ) {
      return {
        hour12,
        minute,
        ampm,
        cadence: "weekends",
      };
    }

    // Specific DOWs that don't match standard clock cadence
    return null;
  }

  return null;
}
