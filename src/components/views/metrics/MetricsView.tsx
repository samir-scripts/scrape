import { Panel } from "../../ui/Panel";
import { Dither } from "../../ui/Dither";
import { Rule } from "../../ui/Rule";

export function MetricsView() {
  return (
    <div className="flex-1 w-full h-full overflow-y-auto p-6 bg-[var(--color-paper)] flex flex-col gap-6 select-none">
      {/* Header banner */}
      <div className="flex items-center justify-between border-b border-[var(--color-ink)] pb-3">
        <div>
          <h1 className="text-xl font-black uppercase tracking-tight text-[var(--color-ink)]">
            METRICS & PIPELINE ANALYTICS
          </h1>
        </div>
        <Dither type="checker" width="96px" height="24px" />
      </div>

      {/* KPI Metric Cards */}
      <div className="grid grid-cols-4 gap-4">
        <Panel className="p-4 flex flex-col gap-1">
          <span className="font-mono text-xs text-[var(--color-ink-mute)] uppercase">
            TOTAL DISCOVERED
          </span>
          <span className="text-3xl font-black font-mono text-[var(--color-ink)]">
            --
          </span>
          <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
            ALL-TIME POSTINGS SCRAPED
          </span>
        </Panel>

        <Panel className="p-4 flex flex-col gap-1">
          <span className="font-mono text-xs text-[var(--color-ink-mute)] uppercase">
            APPLICATIONS FILED
          </span>
          <span className="text-3xl font-black font-mono text-[var(--color-lime)]">
            --
          </span>
          <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
            STAGE: APPLIED
          </span>
        </Panel>

        <Panel className="p-4 flex flex-col gap-1">
          <span className="font-mono text-xs text-[var(--color-ink-mute)] uppercase">
            INTERVIEW RATE
          </span>
          <span className="text-3xl font-black font-mono text-[var(--color-signal)]">
            --%
          </span>
          <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
            APPLIED → INTERVIEWING
          </span>
        </Panel>

        <Panel className="p-4 flex flex-col gap-1">
          <span className="font-mono text-xs text-[var(--color-ink-mute)] uppercase">
            OFFER CONVERSION
          </span>
          <span className="text-3xl font-black font-mono text-[var(--color-cherry)]">
            --%
          </span>
          <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
            INTERVIEW → OFFER
          </span>
        </Panel>
      </div>

      <Rule />

      {/* Placeholder Funnel and Heatmap Grids */}
      <div className="grid grid-cols-2 gap-6 flex-1 min-h-[300px]">
        <Panel className="p-4 flex flex-col">
          <div className="flex justify-between items-center mb-3">
            <span className="font-mono text-xs font-bold uppercase tracking-wider">
              CONVERSION FUNNEL
            </span>
            <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
              [LOCKED · PHASE 4]
            </span>
          </div>
          <div className="flex-1 border border-dashed border-[var(--color-ink-mute)] flex flex-col items-center justify-center p-6 text-center">
            <Dither type="lines" width="120px" height="32px" className="mb-3" />
            <span className="font-mono text-xs uppercase text-[var(--color-ink-mute)]">
              SANKEY STAGE FLOW METRICS ACTIVATING IN PHASE 4
            </span>
          </div>
        </Panel>

        <Panel className="p-4 flex flex-col">
          <div className="flex justify-between items-center mb-3">
            <span className="font-mono text-xs font-bold uppercase tracking-wider">
              APPLICATION ACTIVITY VELOCITY
            </span>
            <span className="font-mono text-[10px] text-[var(--color-ink-mute)]">
              [LOCKED · PHASE 4]
            </span>
          </div>
          <div className="flex-1 border border-dashed border-[var(--color-ink-mute)] flex flex-col items-center justify-center p-6 text-center">
            <Dither type="dots" width="120px" height="32px" className="mb-3" />
            <span className="font-mono text-xs uppercase text-[var(--color-ink-mute)]">
              HEATMAP DITHER VELOCITY ACTIVATING IN PHASE 4
            </span>
          </div>
        </Panel>
      </div>
    </div>
  );
}
