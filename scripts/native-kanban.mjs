// Real WKWebView controls and IPC in an isolated saved-posting fixture. Pointer
// gestures are covered separately; synthesized JS events cannot prove OS dragging.
export async function checkNativeKanban() {
 const invoke = window.__TAURI_INTERNALS__.invoke;
 const assert = (value, message) => { if (!value) throw new Error(message); };
 const wait = async read => { const deadline = Date.now() + 15000; while (Date.now() < deadline) { const result = await read(); if (result) return result; await new Promise(resolve => setTimeout(resolve, 50)); } throw new Error('Native Kanban deadline'); };
 const get = () => invoke('get_job', { id: 'saved' });
 const history = () => invoke('get_job_status_events', { jobId: 'saved' });
 (await wait(() => [...document.querySelectorAll('button')].find(button => button.textContent.startsWith('TRACKED')))).click();
 const picker = await wait(() => document.querySelector('[aria-label="Move Fixture engineer to"]'));
 if (sessionStorage.getItem('kanban-reloaded')) {
  const saved = await get(); assert(saved.status === 'offer' && saved.notes === 'Fixture saved note' && saved.favorited, 'Restart lost saved state');
  assert((await history()).length === 2, 'Restart duplicated history');
  assert(picker.value === 'offer', 'Restart card placement disagrees with repository');
  return { stage: saved.status, historyEvents: 2, notesAndFavoritePreserved: true, input: 'native select and keyboard handle; OS pointer acceptance pending' };
 }
 picker.value = 'interviewing'; picker.dispatchEvent(new Event('change', { bubbles: true }));
 await wait(async () => (await get()).status === 'interviewing');
 await wait(() => document.querySelector('[aria-label="Move Fixture engineer to"]')?.value === 'interviewing');
 assert((await history()).length === 1, 'Move to did not write exactly one transition');
 const handle = await wait(() => document.querySelector('[data-drag-handle]:not(:disabled)'));
 handle.focus();
 for (const key of [' ', 'ArrowRight', 'Escape']) handle.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
 assert((await get()).status === 'interviewing' && (await history()).length === 1, 'Cancelled keyboard drag changed repository');
 for (const key of [' ', 'ArrowRight', 'Enter']) handle.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true }));
 await wait(async () => (await get()).status === 'offer');
 await wait(() => document.querySelector('[aria-label="Move Fixture engineer to"]')?.value === 'offer');
 assert((await history()).length === 2, 'Keyboard move duplicated history');
 sessionStorage.setItem('kanban-reloaded', 'true'); location.reload(); return await new Promise(() => {});
}
