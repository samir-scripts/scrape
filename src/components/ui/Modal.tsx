import { useEffect, useRef, type ReactNode, type MouseEvent } from "react";
import { KeyHint } from "./KeyHint";
import { Rule } from "./Rule";

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  width?: string;
}

export function Modal({
  isOpen,
  onClose,
  title,
  children,
  width = "max-w-lg",
}: ModalProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const previousFocus = useRef<HTMLElement | null>(null);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      if (!dialog.open) {
        previousFocus.current = document.activeElement as HTMLElement;
        dialog.showModal();
      }
    } else {
      if (dialog.open) {
        dialog.close();
        previousFocus.current?.focus();
      }
    }
  }, [isOpen]);

  // Capture-phase keydown listener for Escape: guarantees modal closes immediately regardless of focus/input state
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [isOpen, onClose]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    const handleCancel = (e: Event) => {
      e.preventDefault();
      onClose();
    };

    dialog.addEventListener("cancel", handleCancel);
    return () => dialog.removeEventListener("cancel", handleCancel);
  }, [onClose]);

  // Click outside backdrop
  const handleBackdropClick = (e: MouseEvent<HTMLDialogElement>) => {
    if (e.target === dialogRef.current) {
      onClose();
    }
  };

  return (
    <dialog
      ref={dialogRef}
      aria-label={title}
      onClick={handleBackdropClick}
      className={`fixed inset-0 m-auto p-0 bg-transparent backdrop:bg-[var(--color-ink)]/70 backdrop:backdrop-blur-none backdrop:dither-checker ${
        isOpen ? "block" : "hidden"
      }`}
    >
      <div
        className={`w-full ${width} border-2 border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] shadow-none flex flex-col`}
      >
        {/* Modal Header */}
        <div className="bg-[var(--color-ink)] text-[var(--color-paper)] px-4 py-2.5 flex items-center justify-between select-none">
          <h2 className="font-mono font-bold text-xs uppercase tracking-wider">
            {title}
          </h2>
          <button
            type="button"
            aria-label="Close dialog"
            onClick={onClose}
            className="text-[var(--color-paper)] hover:opacity-70 cursor-pointer flex items-center gap-1.5"
          >
            <KeyHint keys="ESC" className="border-[var(--color-paper)] text-xs" />
          </button>
        </div>

        <Rule />

        {/* Modal Body */}
        <div className="p-4 overflow-y-auto max-h-[80vh]">{children}</div>
      </div>
    </dialog>
  );
}
