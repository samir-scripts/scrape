import { useState, useEffect, useRef, type MouseEvent } from "react";
import type { Job } from "../../bindings/Job";
import { KeyHint } from "./KeyHint";
import { Rule } from "./Rule";
import { Tag } from "./Tag";
import { CompanyLogo } from "./CompanyLogo";

interface DeleteJobModalProps {
  isOpen: boolean;
  job: Job | null;
  onClose: () => void;
  onConfirm: (dontAskAgain: boolean) => void;
}

export function DeleteJobModal({
  isOpen,
  job,
  onClose,
  onConfirm,
}: DeleteJobModalProps) {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const [dontAskAgain, setDontAskAgain] = useState(false);

  // Reset checkbox state whenever modal opens
  useEffect(() => {
    if (isOpen) {
      setDontAskAgain(false);
    }
  }, [isOpen]);

  useEffect(() => {
    const dialog = dialogRef.current;
    if (!dialog) return;

    if (isOpen) {
      if (!dialog.open) {
        dialog.showModal();
      }
    } else {
      if (dialog.open) {
        dialog.close();
      }
    }
  }, [isOpen]);

  // Capture-phase keydown listener for Escape and Enter
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        e.stopPropagation();
        onClose();
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        e.stopPropagation();
        onConfirm(dontAskAgain);
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => window.removeEventListener("keydown", handleKeyDown, true);
  }, [isOpen, onClose, onConfirm, dontAskAgain]);

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

  const handleBackdropClick = (e: MouseEvent<HTMLDialogElement>) => {
    if (e.target === dialogRef.current) {
      onClose();
    }
  };

  if (!isOpen || !job) return null;

  return (
    <dialog
      ref={dialogRef}
      onClick={handleBackdropClick}
      className={`fixed inset-0 m-auto p-0 bg-transparent backdrop:bg-[var(--color-ink)]/70 backdrop:backdrop-blur-none backdrop:dither-checker ${
        isOpen ? "block" : "hidden"
      }`}
    >
      <div className="w-full max-w-md border-2 border-[var(--color-ink)] bg-[var(--color-paper)] text-[var(--color-ink)] shadow-none flex flex-col select-none">
        {/* Modal Header */}
        <div className="bg-[var(--color-ink)] text-[var(--color-paper)] px-4 py-2.5 flex items-center justify-between">
          <div className="flex items-center gap-2">
            <span className="w-2 h-2 bg-[var(--color-cherry)] inline-block" />
            <h2 className="font-mono font-bold text-xs uppercase tracking-wider text-[var(--color-cherry)]">
              CONFIRM DELETION // DANGER
            </h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            className="text-[var(--color-paper)] hover:opacity-70 cursor-pointer flex items-center gap-1.5"
          >
            <KeyHint keys="ESC" className="border-[var(--color-paper)] text-[10px]" />
          </button>
        </div>

        <Rule />

        {/* Modal Body */}
        <div className="p-5 flex flex-col gap-4 font-mono text-xs">
          <p className="font-bold text-sm text-[var(--color-ink)]">
            Are you sure you want to delete?
          </p>

          {/* Job summary card */}
          <div className="p-3 border border-[var(--color-ink)] bg-[var(--color-paper-2)]/40 flex flex-col gap-1.5">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2 min-w-0">
                <CompanyLogo
                  company={job.company}
                  iconUrl={job.company_icon_url}
                  size="sm"
                />
                <span className="font-bold uppercase truncate">
                  {job.company}
                </span>
              </div>
              <Tag status={job.status} />
            </div>
            <div className="font-sans font-bold text-xs text-[var(--color-ink)] line-clamp-2">
              {job.title}
            </div>
            <div className="text-[10px] text-[var(--color-ink-mute)]">
              {job.location || "REMOTE"} · ADDED {job.date_added.slice(0, 10)}
            </div>
          </div>

          <div className="text-[11px] text-[var(--color-ink-mute)] leading-relaxed">
            // NOTICE: THIS ACTION CANNOT BE UNDONE.
            <br />
            // THIS POSTING WILL BE TOMBSTONED AND EXCLUDED FROM ALL FUTURE SCRAPES.
          </div>

          {/* Don't ask again Checkbox */}
          <label className="flex items-center gap-2.5 pt-2 cursor-pointer select-none group">
            <input
              type="checkbox"
              checked={dontAskAgain}
              onChange={(e) => setDontAskAgain(e.target.checked)}
              className="sr-only"
            />
            <span
              className={`w-4 h-4 border border-[var(--color-ink)] flex items-center justify-center font-bold text-xs ${
                dontAskAgain
                  ? "bg-[var(--color-ink)] text-[var(--color-paper)]"
                  : "bg-[var(--color-paper)] text-transparent group-hover:bg-[var(--color-paper-2)]"
              }`}
            >
              ✕
            </span>
            <span className="font-bold text-xs uppercase tracking-wide group-hover:underline">
              DON&apos;T ASK AGAIN
            </span>
          </label>
        </div>

        <Rule />

        {/* Modal Footer Actions */}
        <div className="p-4 bg-[var(--color-paper-2)]/20 flex items-center justify-between gap-3">
          <button
            type="button"
            onClick={onClose}
            className="px-3 py-1.5 border border-[var(--color-ink)] hover:bg-[var(--color-paper-2)] font-mono text-xs font-bold uppercase cursor-pointer"
          >
            [ESC] CANCEL
          </button>

          <button
            type="button"
            onClick={() => onConfirm(dontAskAgain)}
            className="px-4 py-1.5 border border-[var(--color-ink)] bg-[var(--color-cherry)] text-white hover:opacity-90 active:translate-y-[1px] font-mono text-xs font-bold uppercase cursor-pointer flex items-center gap-2"
          >
            <span>[DEL] DELETE</span>
            <KeyHint keys="↵" className="border-white text-white text-[10px]" />
          </button>
        </div>
      </div>
    </dialog>
  );
}
