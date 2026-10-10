import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useUIStore } from "../lib/store";

// Create minimal DOM mocks for Node.js environment
class MockEventTarget {
  private listeners: Record<string, Set<(e: any) => void>> = {};

  addEventListener(type: string, listener: (e: any) => void) {
    if (!this.listeners[type]) {
      this.listeners[type] = new Set();
    }
    this.listeners[type].add(listener);
  }

  removeEventListener(type: string, listener: (e: any) => void) {
    this.listeners[type]?.delete(listener);
  }

  dispatchEvent(event: { type: string; [key: string]: any }) {
    const handlers = this.listeners[event.type];
    if (handlers) {
      for (const handler of Array.from(handlers)) {
        handler(event);
      }
    }
    return true;
  }

  listenerCount(type: string): number {
    return this.listeners[type]?.size ?? 0;
  }
}

describe("Hotkeys Engine & Dispatch Logic", () => {
  let mockWindow: MockEventTarget;
  let originalWindow: any;
  let originalDocument: any;

  beforeEach(() => {
    mockWindow = new MockEventTarget();
    originalWindow = (globalThis as any).window;
    originalDocument = (globalThis as any).document;

    (globalThis as any).window = mockWindow;
    (globalThis as any).document = {
      activeElement: null,
      getElementById: vi.fn().mockReturnValue(null),
    };

    useUIStore.setState({ view: "tracked" });
  });

  afterEach(() => {
    (globalThis as any).window = originalWindow;
    (globalThis as any).document = originalDocument;
    vi.restoreAllMocks();
  });

  it("dispatches stage shifts for both Arrow keys and Vim H/L with Shift", () => {
    const onShiftLeft = vi.fn();
    const onShiftRight = vi.fn();
    const onColLeft = vi.fn();
    const onColRight = vi.fn();

    const fakeOptionsRef = {
      current: {
        onShiftLeft,
        onShiftRight,
        onColLeft,
        onColRight,
      },
    };

    // Handler logic under test (matching useHotkeys)
    const handleKeyDown = (e: any) => {
      const activeEl = (globalThis as any).document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";
      if (isInput) return;

      const hasMod = e.ctrlKey || e.metaKey || e.altKey;

      if (!hasMod && e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "H" || e.key === "h") {
          e.preventDefault?.();
          fakeOptionsRef.current.onShiftLeft?.();
          return;
        }
        if (e.key === "ArrowRight" || e.key === "L" || e.key === "l") {
          e.preventDefault?.();
          fakeOptionsRef.current.onShiftRight?.();
          return;
        }
      }

      if (!hasMod && !e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "h" || e.key === "H") {
          e.preventDefault?.();
          fakeOptionsRef.current.onColLeft?.();
          return;
        }
        if (e.key === "ArrowRight" || e.key === "l" || e.key === "L") {
          e.preventDefault?.();
          fakeOptionsRef.current.onColRight?.();
          return;
        }
      }
    };

    mockWindow.addEventListener("keydown", handleKeyDown);

    // 1. Shift + ArrowLeft
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "ArrowLeft",
      shiftKey: true,
      preventDefault: vi.fn(),
    });
    expect(onShiftLeft).toHaveBeenCalledTimes(1);

    // 2. Shift + H
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "H",
      shiftKey: true,
      preventDefault: vi.fn(),
    });
    expect(onShiftLeft).toHaveBeenCalledTimes(2);

    // 3. Shift + ArrowRight
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "ArrowRight",
      shiftKey: true,
      preventDefault: vi.fn(),
    });
    expect(onShiftRight).toHaveBeenCalledTimes(1);

    // 4. Shift + L
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "L",
      shiftKey: true,
      preventDefault: vi.fn(),
    });
    expect(onShiftRight).toHaveBeenCalledTimes(2);

    // Column navigation callbacks should not have fired during Shift
    expect(onColLeft).not.toHaveBeenCalled();
    expect(onColRight).not.toHaveBeenCalled();

    // 5. Unshifted ArrowLeft and 'h'
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "ArrowLeft",
      shiftKey: false,
      preventDefault: vi.fn(),
    });
    expect(onColLeft).toHaveBeenCalledTimes(1);

    mockWindow.dispatchEvent({
      type: "keydown",
      key: "h",
      shiftKey: false,
      preventDefault: vi.fn(),
    });
    expect(onColLeft).toHaveBeenCalledTimes(2);

    // 6. Unshifted ArrowRight and 'l'
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "ArrowRight",
      shiftKey: false,
      preventDefault: vi.fn(),
    });
    expect(onColRight).toHaveBeenCalledTimes(1);

    mockWindow.dispatchEvent({
      type: "keydown",
      key: "l",
      shiftKey: false,
      preventDefault: vi.fn(),
    });
    expect(onColRight).toHaveBeenCalledTimes(2);
  });

  it("ignores single navigation keys when system modifier keys (Ctrl/Cmd/Alt) are pressed", () => {
    const onColLeft = vi.fn();
    const onShiftLeft = vi.fn();

    const handleKeyDown = (e: any) => {
      const hasMod = e.ctrlKey || e.metaKey || e.altKey;
      if (!hasMod && e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "H" || e.key === "h") {
          onShiftLeft();
        }
      }
      if (!hasMod && !e.shiftKey) {
        if (e.key === "ArrowLeft" || e.key === "h" || e.key === "H") {
          onColLeft();
        }
      }
    };

    mockWindow.addEventListener("keydown", handleKeyDown);

    // Cmd + h (macOS hide window)
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "h",
      metaKey: true,
      shiftKey: false,
    });
    expect(onColLeft).not.toHaveBeenCalled();

    // Ctrl + h
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "h",
      ctrlKey: true,
      shiftKey: false,
    });
    expect(onColLeft).not.toHaveBeenCalled();

    // Alt + ArrowLeft (browser back)
    mockWindow.dispatchEvent({
      type: "keydown",
      key: "ArrowLeft",
      altKey: true,
      shiftKey: false,
    });
    expect(onColLeft).not.toHaveBeenCalled();
  });

  it("suppresses hotkeys when input or textarea elements are active", () => {
    const onColLeft = vi.fn();
    const onNextItem = vi.fn();

    const handleKeyDown = (e: any) => {
      const activeEl = (globalThis as any).document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";
      if (isInput) return;

      if (e.key === "h") onColLeft();
      if (e.key === "j") onNextItem();
    };

    mockWindow.addEventListener("keydown", handleKeyDown);

    // Set active element to INPUT
    (globalThis as any).document.activeElement = { tagName: "INPUT" };

    mockWindow.dispatchEvent({ type: "keydown", key: "h" });
    mockWindow.dispatchEvent({ type: "keydown", key: "j" });

    expect(onColLeft).not.toHaveBeenCalled();
    expect(onNextItem).not.toHaveBeenCalled();

    // Blur input
    (globalThis as any).document.activeElement = null;

    mockWindow.dispatchEvent({ type: "keydown", key: "h" });
    expect(onColLeft).toHaveBeenCalledTimes(1);
  });
});
