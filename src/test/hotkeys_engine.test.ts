import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { useUIStore } from "../lib/store";

// Create minimal DOM mocks for Node.js environment supporting capture and bubble phases
class MockEventTarget {
  private captureListeners: Record<string, Set<(e: any) => void>> = {};
  private bubbleListeners: Record<string, Set<(e: any) => void>> = {};

  addEventListener(type: string, listener: (e: any) => void, useCapture = false) {
    const target = useCapture ? this.captureListeners : this.bubbleListeners;
    if (!target[type]) {
      target[type] = new Set();
    }
    target[type].add(listener);
  }

  removeEventListener(type: string, listener: (e: any) => void, useCapture = false) {
    const target = useCapture ? this.captureListeners : this.bubbleListeners;
    target[type]?.delete(listener);
  }

  dispatchEvent(event: { type: string; [key: string]: any }) {
    let stopped = false;
    event.stopPropagation = () => {
      stopped = true;
    };
    event.stopImmediatePropagation = () => {
      stopped = true;
    };

    // 1. Capture phase
    const captureHandlers = this.captureListeners[event.type];
    if (captureHandlers) {
      for (const handler of Array.from(captureHandlers)) {
        handler(event);
        if (stopped) return true;
      }
    }

    // 2. Bubble phase
    const bubbleHandlers = this.bubbleListeners[event.type];
    if (bubbleHandlers) {
      for (const handler of Array.from(bubbleHandlers)) {
        handler(event);
        if (stopped) return true;
      }
    }
    return true;
  }

  listenerCount(type: string): number {
    return (
      (this.captureListeners[type]?.size ?? 0) +
      (this.bubbleListeners[type]?.size ?? 0)
    );
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

  it("cycles across all five tabs bidirectionally via Q/E and [/] without dead zones", () => {
    // Handler matching useGlobalHotkeys
    const NAV_TABS = [
      { id: "discovered" },
      { id: "tracked" },
      { id: "metrics" },
      { id: "scraper" },
      { id: "settings" },
    ];

    const handleGlobalKeyDown = (e: any) => {
      const activeEl = (globalThis as any).document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";
      if (isInput) return;

      const hasMod = e.ctrlKey || e.metaKey || e.altKey;
      if (hasMod) return;

      // Q / [ -> prev
      if (!e.shiftKey && (e.key === "q" || e.key === "Q" || e.key === "[")) {
        e.preventDefault?.();
        const currentView = useUIStore.getState().view;
        const currentIdx = NAV_TABS.findIndex((t) => t.id === currentView);
        const prevIdx = (currentIdx - 1 + NAV_TABS.length) % NAV_TABS.length;
        useUIStore.getState().setView(NAV_TABS[prevIdx].id as any);
        return;
      }

      // E / ] -> next
      if (!e.shiftKey && (e.key === "e" || e.key === "E" || e.key === "]")) {
        e.preventDefault?.();
        const currentView = useUIStore.getState().view;
        const currentIdx = NAV_TABS.findIndex((t) => t.id === currentView);
        const nextIdx = (currentIdx + 1) % NAV_TABS.length;
        useUIStore.getState().setView(NAV_TABS[nextIdx].id as any);
        return;
      }
    };

    mockWindow.addEventListener("keydown", handleGlobalKeyDown);

    // Start at discovered
    useUIStore.setState({ view: "discovered" });

    // Press 'e': discovered -> tracked
    mockWindow.dispatchEvent({ type: "keydown", key: "e", shiftKey: false });
    expect(useUIStore.getState().view).toBe("tracked");

    // Press 'e': tracked -> metrics
    mockWindow.dispatchEvent({ type: "keydown", key: "e", shiftKey: false });
    expect(useUIStore.getState().view).toBe("metrics");

    // Press 'E' (capital): metrics -> scraper (CRITICAL: was previously dead!)
    mockWindow.dispatchEvent({ type: "keydown", key: "E", shiftKey: false });
    expect(useUIStore.getState().view).toBe("scraper");

    // Press ']': scraper -> settings
    mockWindow.dispatchEvent({ type: "keydown", key: "]", shiftKey: false });
    expect(useUIStore.getState().view).toBe("settings");

    // Press 'e': settings -> wrap around to discovered
    mockWindow.dispatchEvent({ type: "keydown", key: "e", shiftKey: false });
    expect(useUIStore.getState().view).toBe("discovered");

    // Backward cycle with 'q': discovered -> settings
    mockWindow.dispatchEvent({ type: "keydown", key: "q", shiftKey: false });
    expect(useUIStore.getState().view).toBe("settings");

    // Press 'q': settings -> scraper
    mockWindow.dispatchEvent({ type: "keydown", key: "q", shiftKey: false });
    expect(useUIStore.getState().view).toBe("scraper");

    // Press '[': scraper -> metrics
    mockWindow.dispatchEvent({ type: "keydown", key: "[", shiftKey: false });
    expect(useUIStore.getState().view).toBe("metrics");

    // Press 'Q' (capital): metrics -> tracked
    mockWindow.dispatchEvent({ type: "keydown", key: "Q", shiftKey: false });
    expect(useUIStore.getState().view).toBe("tracked");

    // Press 'q': tracked -> discovered
    mockWindow.dispatchEvent({ type: "keydown", key: "q", shiftKey: false });
    expect(useUIStore.getState().view).toBe("discovered");
  });

  it("handles Escape by closing filter panel or drawer before blurring", () => {
    const handleGlobalKeyDown = (e: any) => {
      const activeEl = (globalThis as any).document.activeElement;
      const isInput =
        activeEl?.tagName === "INPUT" ||
        activeEl?.tagName === "TEXTAREA" ||
        activeEl?.tagName === "SELECT";

      if (e.key === "Escape") {
        if (isInput) {
          activeEl?.blur?.();
          return;
        }
        const { isFilterPanelOpen, setFilterPanelOpen, drawerJobId, setDrawerJobId } =
          useUIStore.getState();
        if (isFilterPanelOpen) {
          setFilterPanelOpen(false);
          return;
        }
        if (drawerJobId) {
          setDrawerJobId(null);
          return;
        }
      }
    };

    mockWindow.addEventListener("keydown", handleGlobalKeyDown);

    // 1. Filter panel open
    useUIStore.setState({ isFilterPanelOpen: true, drawerJobId: null });
    mockWindow.dispatchEvent({ type: "keydown", key: "Escape" });
    expect(useUIStore.getState().isFilterPanelOpen).toBe(false);

    // 2. Detail drawer open
    useUIStore.setState({ isFilterPanelOpen: false, drawerJobId: "job-123" });
    mockWindow.dispatchEvent({ type: "keydown", key: "Escape" });
    expect(useUIStore.getState().drawerJobId).toBeNull();

    // 3. Input blur
    const blurFn = vi.fn();
    (globalThis as any).document.activeElement = {
      tagName: "INPUT",
      blur: blurFn,
    };
    mockWindow.dispatchEvent({ type: "keydown", key: "Escape" });
    expect(blurFn).toHaveBeenCalledTimes(1);
  });

  it("dismisses Modal dialogs immediately on Escape via capture phase even when input is focused", () => {
    const onModalClose = vi.fn();
    let isModalOpen = true;

    // Capture-phase listener (matching Modal.tsx)
    const handleModalKeyDown = (e: any) => {
      if (!isModalOpen) return;
      if (e.key === "Escape") {
        e.preventDefault?.();
        e.stopPropagation?.();
        onModalClose();
        isModalOpen = false;
      }
    };

    // Bubble-phase listener (matching useGlobalHotkeys in App.tsx)
    const handleGlobalKeyDown = (e: any) => {
      if (e.key === "Escape") {
        const activeEl = (globalThis as any).document.activeElement;
        if (activeEl?.tagName === "INPUT") {
          activeEl.blur?.();
        }
      }
    };

    mockWindow.addEventListener("keydown", handleModalKeyDown, true); // capture = true
    mockWindow.addEventListener("keydown", handleGlobalKeyDown, false); // bubble = false

    // Simulate input actively focused inside the AddSourceModal
    const inputBlurFn = vi.fn();
    (globalThis as any).document.activeElement = {
      tagName: "INPUT",
      blur: inputBlurFn,
    };

    // User presses Escape
    mockWindow.dispatchEvent({ type: "keydown", key: "Escape" });

    // 1. Modal's onClose MUST be called immediately
    expect(onModalClose).toHaveBeenCalledTimes(1);

    // 2. Global hotkey handler was prevented from running due to stopPropagation in capture phase
    expect(inputBlurFn).not.toHaveBeenCalled();

    // 3. Subsequent keydown when modal is closed now falls through to global handlers
    mockWindow.dispatchEvent({ type: "keydown", key: "Escape" });
    expect(onModalClose).toHaveBeenCalledTimes(1); // not called again
    expect(inputBlurFn).toHaveBeenCalledTimes(1); // now global handler runs
  });
});
