# Section 7 scheduling state contract

Priority is evaluated from current backend status; a retained response after an IPC error is stale and cannot substantiate a next-run label.

| Evidence | Header | Summary |
|---|---|---|
| Browser examples | Desktop required | Examples do not run an automatic scheduler |
| No response, pending | Loading schedule | Loading; refresh available |
| IPC error | Schedule unavailable | Error and retry; no inferred pause or next run |
| Active run | Running; paused if applicable | Progress, cancellation, and independent automatic scheduling state |
| Automatic scheduling paused | Paused | Existing schedules shown as suspended; manual runs remain available |
| No enabled workflows | No enabled workflows | Enable or create a workflow |
| Enabled workflows, all manual | Manual only | No automatic dispatch |
| Automatic workflow missing/invalid next instant | Schedule needs attention | Identify affected workflow; refresh or edit |
| Valid automatic schedules | Next date and time | Earliest workflow(s), all scheduled workflows and their IANA timezones |

Use absolute dates so tomorrow is not confused with today. Format dates in each schedule's timezone. Never calculate calendar occurrences in JavaScript. Global workflows use the dispatcher's global instant, not their historical per-source timestamp. Disabled/manual workflows never contribute a next run. Pausing does not cancel an active run. Source, settings, run-start, source-finish and terminal-run events invalidate the same status query. No component timer recalculates schedules.
