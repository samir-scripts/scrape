/** Tauri rejects with serialized { kind, message }, while browser failures are Errors. */
export function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  return "The operation failed. Check the execution log for details.";
}
