import { isTauri } from "@tauri-apps/api/core";

export function isDemoMode(): boolean {
  return !isTauri() && (import.meta.env.MODE === "test" ||
    (typeof window !== "undefined" && new URLSearchParams(window.location.search).get("demo") === "1"));
}

export function needsDesktop(): boolean {
  return !isTauri() && !isDemoMode();
}
