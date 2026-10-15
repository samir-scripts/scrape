// Actual WKWebView control + IPC checks, restricted by the isolated native harness.
export async function checkNativeSchedule() {
 const invoke = window.__TAURI_INTERNALS__.invoke;
 const assert = (value, message) => { if (!value) throw new Error(message); };
 const wait = async read => { const deadline = Date.now() + 15000; while (Date.now() < deadline) { const result = await read(); if (result) return result; await new Promise(resolve => setTimeout(resolve, 50)); } throw new Error('Native schedule deadline'); };
 const button = text => [...document.querySelectorAll('button')].find(el => el.textContent.trim() === text && el.getClientRects().length);
 const trigger = () => document.querySelector('button[aria-label^="Schedule:"]');
 const beforeJobs = await invoke('list_jobs', {filter:{}}), beforeRuns = await invoke('list_scrape_runs', {limit:100});
 await wait(() => trigger()); trigger().focus(); trigger().click();
 await wait(() => document.querySelector('dialog[aria-label="Schedule summary"][open]'));
 assert(button('Resume automatic runs'), 'Paused backend not represented');
 button('Resume automatic runs').click(); await wait(() => trigger().textContent.includes('No enabled workflows'));
 const source = await invoke('create_source', {input:{kind:'greenhouse', name:'Schedule fixture', identifier:'schedule-fixture',enabled:true,cadence:'manual'}});
 await wait(() => trigger().textContent.includes('Manual only'));
 await invoke('update_source', {input:{id:source.id,cadence:'daily',config:{__schedule:{time:'09:00',timezone:'Pacific/Auckland',weekdays:[]}}}});
 await wait(() => trigger().textContent.includes('Next'));
 const status = await invoke('get_scheduler_status');
 const schedule = status.schedules.find(s => s.id === source.id);
 assert(schedule && schedule.next_run_at === status.next_run_at && schedule.timezone === 'Pacific/Auckland', 'Header status disagrees with dispatcher instant/timezone');
 const formatted = new Intl.DateTimeFormat(undefined,{month:'short',day:'numeric',year:'numeric',hour:'2-digit',minute:'2-digit',hour12:false,timeZone:schedule.timezone}).format(new Date(status.next_run_at));
 assert(trigger().textContent.includes(formatted), 'Header formats a different instant');
 assert(document.querySelector('dialog[aria-label="Schedule summary"]').textContent.includes(schedule.timezone), 'Summary missing workflow timezone');
 await invoke('update_source',{input:{id:source.id,enabled:false}});await wait(()=>trigger().textContent.includes('No enabled workflows'));
 await invoke('update_source',{input:{id:source.id,enabled:true}});await wait(()=>trigger().textContent.includes('Next'));
 button('Pause automatic runs').click();await wait(()=>trigger().textContent.includes('Paused'));
 document.querySelector('dialog[aria-label="Schedule summary"] button[aria-label="Close dialog"]').click();
 await wait(() => !document.querySelector('dialog[aria-label="Schedule summary"][open]'));
 await wait(() => document.activeElement===trigger());
 await invoke('delete_source',{id:source.id});
 assert((await invoke('list_jobs',{filter:{}})).total===beforeJobs.total, 'Schedule controls persisted jobs');
 assert((await invoke('list_scrape_runs',{limit:100})).length===beforeRuns.length, 'Opening/editing schedules triggered a worker');
 return {summary:true,pauseResume:true,createUpdateToggleDelete:true,timezone:schedule.timezone,nextRun:status.next_run_at,noImplicitRun:true};
}
