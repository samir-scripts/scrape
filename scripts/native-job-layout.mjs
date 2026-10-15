// Serialized into the isolated WKWebView; fixture setup never touches the live profile.
export async function checkNativeJobLayout() {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const assert = (ok, message) => { if (!ok) throw new Error(message); };
  const wait = async (read, label) => {
    const deadline = Date.now() + 15000;
    while (Date.now() < deadline) { const value = await read(); if (value) return value; await new Promise(resolve => setTimeout(resolve, 50)); }
    throw new Error(`Native layout deadline: ${label}`);
  };
  const find = label => document.querySelector(`[aria-label="${label}"]`);
  const change = (input, value) => {
    const prototype = input.tagName === 'TEXTAREA' ? HTMLTextAreaElement.prototype : HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype, 'value').set.call(input, value);
    input.dispatchEvent(new Event('input', { bubbles: true }));
  };
  (await wait(() => [...document.querySelectorAll('button')].find(button => button.textContent.startsWith('DISCOVERED')), 'discovered navigation')).click();
  await wait(() => find('Page number')?.value === '1', 'first page');
  assert(document.querySelectorAll('[role="toolbar"][aria-label="Job controls"]').length === 1, 'Duplicated toolbar');
  assert(find('Job controls').contains(find('Sort jobs')), 'Sort outside toolbar');
  const open = await wait(() => document.querySelector('[aria-label^="View Layout fixture 000 "]'), 'first fixture posting');
  open.click();
  const separator = await wait(() => find('Job results width'), 'separator');
  if (sessionStorage.getItem('layout-reloaded')) {
    assert(Number(separator.getAttribute('aria-valuenow')) === Number(sessionStorage.getItem('layout-ratio')), 'Pane preference lost on restart');
    const notes = await wait(() => document.querySelector('textarea'), 'reopened notes');
    assert(notes.value === 'Native pending note', 'Saved note lost on reload');
    assert((await invoke('get_job', { id: 'layout-000' })).notes === notes.value, 'Native repository note disagrees');
    return { persistedRatio: separator.getAttribute('aria-valuenow'), page: find('Page number').value, notesPersisted: true, resizeInput: 'keyboard plus native window size changes; real pointer drag remains manual QA' };
  }
  const before = Number(separator.getAttribute('aria-valuenow'));
  separator.focus(); separator.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
  await wait(() => Number(separator.getAttribute('aria-valuenow')) < before, 'keyboard resize');
  const ratio = separator.getAttribute('aria-valuenow');
  const notes = await wait(() => document.querySelector('textarea'), 'notes');
  notes.focus(); change(notes, 'Native pending note');
  const scroll = notes.closest('[data-dialog-scroll]'); scroll.scrollTop = 150;
  await invoke('plugin:event|emit', { event: 'native-smoke-control', payload: { mode: 'resize-window', width: 480, height: 800 } });
  await wait(() => find('Job details')?.getAttribute('role') !== 'complementary', 'narrow modal');
  await new Promise(resolve => setTimeout(resolve, 50));
  assert(scroll.scrollTop === 150, `Narrow scroll changed: ${scroll.scrollTop}`);
  assert(document.querySelector('textarea') === notes && notes.value === 'Native pending note', 'Draft remounted on narrow transition');
  await invoke('plugin:event|emit', { event: 'native-smoke-control', payload: { mode: 'resize-window', width: 1280, height: 800 } });
  await wait(() => find('Job details')?.getAttribute('role') === 'complementary', 'wide pane');

  await new Promise(resolve => setTimeout(resolve, 50));
  assert(document.querySelector('textarea') === notes && scroll.scrollTop === 150, `Draft/scroll lost across resize: sameNode=${document.querySelector('textarea') === notes} scroll=${scroll.scrollTop}`);
  const input = find('Page number'); change(input, '3');
  find('Go to page').click();
  await wait(() => find('Job pagination')?.textContent.includes('101–105 of 105'), 'direct last-page jump');
  assert(!find('Job details'), 'Off-page inspector remained open');
  await wait(async () => (await invoke('get_job', { id: 'layout-000' })).notes === 'Native pending note', 'queued note persisted to original job');
  sessionStorage.setItem('layout-ratio', ratio); sessionStorage.setItem('layout-reloaded', 'true');
  location.reload();
  return await new Promise(() => {}); // The reloaded page verifies persistence and emits completion.
}
