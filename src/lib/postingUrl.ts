import { errorMessage } from "./errors";
import { isTauri } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { needsDesktop } from "./runtime";

/** Preserve the stored provider destination exactly, with a safe scheme boundary. */
export async function openPostingUrl(destination: string): Promise<void> {
  const url = new URL(destination);
  if (!["http:", "https:"].includes(url.protocol)) throw new Error("Job destinations must use HTTP or HTTPS.");
  if (isTauri()) { try { await openUrl(destination); } catch (error) { throw new Error(errorMessage(error)); } return; }
  if (needsDesktop()) throw new Error("Opening saved postings requires the desktop application.");
  window.open(destination, "_blank", "noopener,noreferrer");
}
