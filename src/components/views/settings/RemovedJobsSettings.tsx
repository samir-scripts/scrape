import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { openPostingUrl } from "../../../lib/postingUrl";
import { api } from "../../../lib/api";
import { notifyError } from "../../../lib/toast";
import { Select } from "../../ui/Select";

export function RemovedJobsSettings() {
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [view, setView] = useState("all");
  const [offset, setOffset] = useState(0);
  const limit = 25;
  const { data, isPending, error } = useQuery({
    queryKey: ["removed_postings", search, view, offset],
    queryFn: () => api.listRemovedPostings(search, view, offset, limit),
  });
  const action = useMutation({
    mutationFn: ({ identity, operation }: { identity: string; operation: "restore" | "unblock" | "block" }) =>
      operation === "restore" ? api.restorePosting(identity) : api.setPostingBlocked(identity, operation === "block"),
    onError: (error) => notifyError(String(error), "POSTING ACTION FAILED"),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["removed_postings"] });
      queryClient.invalidateQueries({ queryKey: ["jobs"] });
      queryClient.invalidateQueries({ queryKey: ["metrics"] });
      queryClient.invalidateQueries({ queryKey: ["job_counts"] });
    },
  });
  return <section className="max-w-4xl">
    <h1 className="text-xl font-bold">Removed &amp; blocked jobs</h1>
    <p className="my-3 text-sm">Restore saved jobs with their notes and history, or change which postings future scrapes may rediscover.</p>
    <div className="flex flex-wrap gap-3 mb-5">
      <label className="flex flex-col gap-1 text-sm">Search title, company or source
        <input data-control-action="settings.removed.search" value={search} onChange={(event) => { setSearch(event.target.value); setOffset(0); }} className="border bg-[var(--color-paper)] p-2" />
      </label>
      <label className="flex flex-col gap-1 text-sm">View
        <Select actionId="settings.removed.view" aria-label="View" value={view} onChange={(value) => { setView(value); setOffset(0); }} options={[
          { value: "all", label: "All" }, { value: "removed", label: "Removed" }, { value: "blocked", label: "Blocked" },
        ]} />
      </label>
    </div>
    {isPending && <p role="status">Loading removed jobs…</p>}
    {error && <p role="alert">Could not load removed jobs: {String(error)}</p>}
    {data && data.items.length === 0 && <p>No {view === "all" ? "removed or blocked" : view} jobs match this view.</p>}
    <ul className="divide-y divide-[var(--color-ink)]">
      {data?.items.map((posting) => <li key={posting.identity} className="py-4 select-text">
        <h2 className="font-bold">{posting.title}</h2>
        <p>{posting.company || "Company unavailable"} · {posting.source_name || "Source unavailable"}</p>
        <p className="my-2 text-sm text-[var(--color-ink-mute)]">
          {posting.removed_at ? `Removed ${new Date(posting.removed_at).toLocaleString()}` : "No saved removed copy"}
          {posting.blocked ? " · Blocked from future scrapes" : " · May return on a future scrape"}
          {posting.copies > 1 && ` · ${posting.copies} saved copies`}
        </p>
        {posting.url ? <a data-control-action="settings.removed.open_posting" href={posting.url} onClick={(event) => {
          event.preventDefault(); openPostingUrl(posting.url!).catch((error) => notifyError(String(error), "OPEN FAILED"));
        }} className="underline break-all text-sm">{posting.url}</a> : <p className="text-sm">Legacy suppression: the original posting details are unavailable.</p>}
        <div className="flex flex-wrap gap-3 mt-3">
          {posting.copies > 0 && <button data-control-action="settings.removed.restore" type="button" disabled={action.isPending} onClick={() => action.mutate({ identity: posting.identity, operation: "restore" })} className="border px-3 py-2">Restore saved job and unblock</button>}
          <button data-control-action="settings.removed.toggle_block" type="button" disabled={action.isPending} onClick={() => action.mutate({ identity: posting.identity, operation: posting.blocked ? "unblock" : "block" })} className="border px-3 py-2">
            {posting.blocked ? "Unblock future discovery" : "Block future discovery"}
          </button>
        </div>
      </li>)}
    </ul>
    {data && <div className="flex items-center gap-3 mt-5 text-sm">
      <button data-control-action="settings.removed.previous" disabled={offset === 0} onClick={() => setOffset(Math.max(0, offset - limit))} className="border px-3 py-2">Previous</button>
      <span>{Number(data.total) === 0 ? "0 results" : `${offset + 1}–${Math.min(offset + limit, Number(data.total))} of ${data.total}`}</span>
      <button data-control-action="settings.removed.next" disabled={offset + limit >= Number(data.total)} onClick={() => setOffset(offset + limit)} className="border px-3 py-2">Next</button>
    </div>}
  </section>;
}
