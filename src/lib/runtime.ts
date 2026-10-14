import { isTauri } from "@tauri-apps/api/core";

export function isDemoMode(): boolean {
  return !isTauri() && (import.meta.env.MODE === "test" ||
    (typeof window !== "undefined" && new URLSearchParams(window.location.search).get("demo") === "1"));
}

export function needsDesktop(): boolean {
  return !isTauri() && !isDemoMode();
}

/** Synthetic UI fixtures are available only in tests or explicit development harnesses. */
export function isFixtureMode(): boolean {
  return import.meta.env.MODE === "test" || (import.meta.env.DEV && typeof window !== "undefined" && new URLSearchParams(window.location.search).get("fixtures") === "1");
}
export function isRecordedMode(): boolean { return isDemoMode() && !isFixtureMode(); }
