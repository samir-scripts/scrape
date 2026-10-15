import { useEffect, useLayoutEffect, useRef, type ReactNode, type MouseEvent } from "react";
import { Icon } from "./Icon";
import { IconButton } from "./IconButton";

interface ModalProps {
  isOpen: boolean; onClose: () => void; title: string; children: ReactNode;
  width?: string; footer?: ReactNode; inline?: boolean;
}
// A responsive inspector can switch modal/inline presentation without moving its children.
// Other dialogs use the same focus, Escape, cancellation and restoration lifecycle.
export function Modal({ isOpen, onClose, title, children, width = "max-w-lg", footer, inline = false }: ModalProps) {
  const dialogRef = useRef<HTMLDialogElement>(null), scrollRef = useRef<HTMLDivElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null), wasOpen = useRef(false);
  const previousInline = useRef<boolean | null>(null);
  const restoreFocus = (dialog: HTMLDialogElement) => {
    const active = document.activeElement;
    if ((active === document.body || dialog.contains(active)) && previousFocus.current?.isConnected) previousFocus.current.focus({ preventScroll: true });
  };
  useLayoutEffect(() => {
    const dialog = dialogRef.current!;
    const active = document.activeElement as HTMLElement | null;
    const inside = active && dialog.contains(active);
    const scroll = scrollRef.current?.scrollTop ?? 0;
    if (isOpen) {
      if (!wasOpen.current) previousFocus.current = active;
      if (dialog.open && previousInline.current !== inline) dialog.close();
      if (!dialog.open) { if (inline) dialog.setAttribute("open", ""); else dialog.showModal(); }
      if (inside) active.focus({ preventScroll: true });
      if (scrollRef.current) scrollRef.current.scrollTop = scroll;
      // WebKit may scroll the previously focused field after showModal returns.
      // Restore after its focus task, without relying on animation frames in inactive windows.
      const timer = window.setTimeout(() => { if (scrollRef.current) scrollRef.current.scrollTop = scroll; }, 0);
      wasOpen.current = isOpen; previousInline.current = inline;
      return () => window.clearTimeout(timer);
    } else if (dialog.open) { dialog.close(); restoreFocus(dialog); }
    wasOpen.current = isOpen; previousInline.current = inline;
  }, [isOpen, inline]);
  useEffect(() => {
    const dialog = dialogRef.current!;
    return () => restoreFocus(dialog);
  }, []);
  useEffect(() => {
    if (!isOpen) return;
    const escape = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.key !== "Escape") return;
      // The shared Listbox owns the first Escape while its options are open.
      const dialog = dialogRef.current;
      if (dialog?.contains(document.activeElement) && dialog.querySelector('[aria-haspopup="listbox"][aria-expanded="true"], [aria-haspopup="menu"][aria-expanded="true"], [role="combobox"][aria-expanded="true"]')) return;
      if (inline && document.querySelector('dialog[open]:not([data-inline-dialog="true"])')) return;
      event.preventDefault(); event.stopPropagation(); onClose();
    };
    window.addEventListener("keydown", escape, !inline);
    return () => window.removeEventListener("keydown", escape, !inline);
  }, [isOpen, inline, onClose]);
  const backdrop = (event: MouseEvent<HTMLDialogElement>) => { if (!inline && event.target === event.currentTarget) onClose(); };
  return <dialog ref={dialogRef} role={inline ? "complementary" : undefined}
    aria-label={title} data-inline-dialog={inline ? "true" : undefined} onClick={backdrop}
    onCancel={event => {
      event.preventDefault();
      if (dialogRef.current?.querySelector('[aria-haspopup="listbox"][aria-expanded="true"], [aria-haspopup="menu"][aria-expanded="true"], [role="combobox"][aria-expanded="true"]')) return;
      onClose();
    }}
    className={`${inline ? "relative m-0 w-full h-full min-w-0" : "fixed inset-0 m-auto max-w-[calc(100vw-32px)] max-h-[calc(100dvh-32px)] backdrop:bg-black/50"} p-0 border-0 bg-transparent ${isOpen ? "block" : "hidden"}`}>
    <div className={`${inline ? "w-full h-full min-h-0" : `w-full ${width} border-2 border-[var(--color-ink)] max-h-[calc(100dvh-32px)]`} bg-[var(--color-paper)] text-[var(--color-ink)] flex flex-col`}>
      <div className={`${inline ? "hidden" : "flex"} bg-[var(--color-ink)] text-[var(--color-paper)] px-4 py-3 shrink-0 items-center justify-between select-none`}>
        <h2 className="font-bold text-xs uppercase tracking-wider">{title}</h2>
        <IconButton label="Close dialog" onClick={onClose} variant="inverse"><Icon name="close" /></IconButton>
      </div>
      <div key="content" ref={scrollRef} data-dialog-scroll className="p-4 overflow-y-auto min-h-0 flex-1">{children}</div>
      {footer && <div className="p-4 border-t shrink-0 bg-[var(--color-paper)]">{footer}</div>}
    </div>
  </dialog>;
}
