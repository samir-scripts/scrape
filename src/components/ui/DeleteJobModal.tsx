import { useEffect, useRef, useState } from "react";
import type { Job } from "../../bindings/Job";

interface DeleteJobModalProps {
  isOpen: boolean;
  job: Job | null;
  copies?: number;
  pending?: boolean;
  onClose: () => void;
  onConfirm: (block: boolean) => void;
}

export function DeleteJobModal({ isOpen, job, copies = 1, pending = false, onClose, onConfirm }: DeleteJobModalProps) {
  const ref = useRef<HTMLDialogElement>(null);
  const cancelRef = useRef<HTMLButtonElement>(null);
  const [block, setBlock] = useState(false);
  useEffect(() => {
    const dialog = ref.current;
    if (!dialog || !isOpen) return;
    const previous = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    setBlock(false);
    dialog.showModal();
    cancelRef.current?.focus();
    return () => { dialog.close(); previous?.focus(); };
  }, [isOpen, job?.id]);
  if (!isOpen || !job) return null;
  return <dialog ref={ref} aria-labelledby="remove-job-title" aria-describedby="remove-job-description"
    onCancel={(event) => { event.preventDefault(); if (!pending) onClose(); }}
    className="m-auto w-full max-w-md border-2 border-[var(--color-ink)] p-5 bg-[var(--color-paper)] text-[var(--color-ink)] backdrop:bg-black/60">
    <h2 id="remove-job-title" className="text-lg font-bold">Remove saved job</h2>
    <p className="mt-4 font-bold select-text">{job.title}</p>
    <p className="select-text">{job.company}</p>
    <p id="remove-job-description" className="mt-4 text-sm">Your notes, favorites and status history remain recoverable in Settings → Removed &amp; blocked jobs.
      {copies > 1 && ` All ${copies} saved copies of this posting will be removed.`}</p>
    <label className="flex gap-3 items-start my-4 text-sm">
      <input type="checkbox" checked={block} onChange={(event) => setBlock(event.target.checked)} disabled={pending} />
      Keep this job out of future scrapes
    </label>
    <p className="text-sm text-[var(--color-ink-mute)]">Without blocking, this posting may return on a later scrape.</p>
    <div className="mt-5 flex gap-3 justify-end">
      <button ref={cancelRef} type="button" onClick={onClose} disabled={pending} className="border px-3 py-2">Cancel</button>
      <button type="button" onClick={() => onConfirm(block)} disabled={pending} className="border border-current px-3 py-2 text-[var(--color-danger)]">
        {pending ? "Removing…" : "Remove job"}
      </button>
    </div>
  </dialog>;
}
