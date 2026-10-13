import { useToastStore, type Toast } from "../../lib/toast";

export function ToastContainer() {
  const { toasts, dismissToast } = useToastStore();

  if (toasts.length === 0) {
    return null;
  }

  return (
    <aside
      aria-label="Notifications"
      className="fixed bottom-4 right-4 z-50 flex flex-col gap-2 max-w-sm w-full pointer-events-none select-none font-mono"
    >
      {toasts.map((toast) => (
        <ToastItem key={toast.id} toast={toast} onDismiss={dismissToast} />
      ))}
    </aside>
  );
}

function ToastItem({
  toast,
  onDismiss,
}: {
  toast: Toast;
  onDismiss: (id: string) => void;
}) {
  const isError = toast.type === "error";
  const isSuccess = toast.type === "success";

  const borderColor = isError
    ? "border-[var(--color-cherry)]"
    : isSuccess
    ? "border-[var(--color-lime)]"
    : "border-[var(--color-signal)]";

  const markerColor = isError
    ? "text-[var(--color-cherry)]"
    : isSuccess
    ? "text-[var(--color-lime)]"
    : "text-[var(--color-signal)]";

  return (
    <div
      role="alert"
      className={`pointer-events-auto bg-[var(--color-paper)] border ${borderColor} p-3 text-xs shadow-none transition-all duration-150`}
    >
      <div className={`flex items-center justify-between border-b ${borderColor} pb-1.5 mb-2`}>
        <div className="flex items-center gap-1.5 font-bold uppercase tracking-wider">
          <span className={markerColor}>■</span>
          <span className="text-[var(--color-ink)]">{toast.title ?? (isError ? "ERROR" : isSuccess ? "SUCCESS" : "INFO")}</span>
        </div>
        <button
          type="button"
          onClick={() => onDismiss(toast.id)}
          className="text-[var(--color-ink-mute)] hover:text-[var(--color-ink)] hover:bg-[var(--color-paper-2)] px-1 py-0.5 text-xs font-mono transition-colors"
          title="Dismiss"
          aria-label="Dismiss notification"
        >
          [×]
        </button>
      </div>
      <div className="text-[var(--color-ink)] select-text break-words whitespace-pre-wrap leading-relaxed font-mono">
        {toast.message}
      </div>
      {toast.action && <button type="button" className="mt-3 border px-3 py-2 text-sm" onClick={() => { toast.action?.run(); onDismiss(toast.id); }}>{toast.action.label}</button>}
    </div>
  );
}
