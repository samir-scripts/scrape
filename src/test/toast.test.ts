import { describe, it, expect, beforeEach, vi, afterEach } from "vitest";
import { useToastStore, notifyError, notifySuccess, notifyInfo } from "../lib/toast";

describe("Toast Store", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    useToastStore.getState().clearAll();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("adds an error toast and auto-dismisses after 5000ms", () => {
    const id = notifyError("YAML parse error on line 4");
    expect(useToastStore.getState().toasts).toHaveLength(1);
    const toast = useToastStore.getState().toasts[0];
    expect(toast.id).toBe(id);
    expect(toast.type).toBe("error");
    expect(toast.message).toBe("YAML parse error on line 4");
    expect(toast.title).toBe("ERROR");

    // Advance 4999ms - should still be there
    vi.advanceTimersByTime(4999);
    expect(useToastStore.getState().toasts).toHaveLength(1);

    // Advance past 5000ms - should be auto-dismissed
    vi.advanceTimersByTime(1);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("adds success and info toasts with custom titles and durations", () => {
    notifySuccess("Workflow saved successfully", "WORKFLOW SAVED", 2000);
    notifyInfo("Background run queued", "RUN STATUS", 3000);

    const toasts = useToastStore.getState().toasts;
    expect(toasts).toHaveLength(2);
    expect(toasts[0].type).toBe("success");
    expect(toasts[0].title).toBe("WORKFLOW SAVED");
    expect(toasts[1].type).toBe("info");

    vi.advanceTimersByTime(2000);
    expect(useToastStore.getState().toasts).toHaveLength(1);
    expect(useToastStore.getState().toasts[0].type).toBe("info");

    vi.advanceTimersByTime(1000);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });

  it("manually dismisses a toast before timeout", () => {
    const id = notifyError("Failed to fetch jobs");
    expect(useToastStore.getState().toasts).toHaveLength(1);

    useToastStore.getState().dismissToast(id);
    expect(useToastStore.getState().toasts).toHaveLength(0);
  });
});
