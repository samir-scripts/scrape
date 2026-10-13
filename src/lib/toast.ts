import { create } from "zustand";

export interface Toast {
  id: string;
  type: "error" | "success" | "info";
  title?: string;
  message: string;
  duration?: number;
  createdAt: number;
  action?: { label: string; run: () => void };
}

interface ToastState {
  toasts: Toast[];
  addToast: (toast: Omit<Toast, "id" | "createdAt">) => string;
  dismissToast: (id: string) => void;
  clearAll: () => void;
}

let toastIdSeq = 0;

export const useToastStore = create<ToastState>((set) => ({
  toasts: [],
  addToast: (t) => {
    const id = `toast-${++toastIdSeq}-${Date.now()}`;
    const duration = t.duration ?? 5000;
    const newToast: Toast = {
      ...t,
      id,
      duration,
      createdAt: Date.now(),
    };

    set((state) => ({
      toasts: [...state.toasts, newToast],
    }));

    if (duration > 0) {
      setTimeout(() => {
        set((state) => ({
          toasts: state.toasts.filter((item) => item.id !== id),
        }));
      }, duration);
    }

    return id;
  },
  dismissToast: (id) =>
    set((state) => ({
      toasts: state.toasts.filter((item) => item.id !== id),
    })),
  clearAll: () => set({ toasts: [] }),
}));

export function notifyError(message: string, title = "ERROR", duration = 5000) {
  return useToastStore.getState().addToast({
    type: "error",
    title,
    message,
    duration,
  });
}

export function notifySuccess(message: string, title = "SUCCESS", duration = 5000) {
  return useToastStore.getState().addToast({
    type: "success",
    title,
    message,
    duration,
  });
}

export function notifyInfo(message: string, title = "INFO", duration = 5000) {
  return useToastStore.getState().addToast({
    type: "info",
    title,
    message,
    duration,
  });
}
