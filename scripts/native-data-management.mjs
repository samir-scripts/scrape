// Actual WKWebView confirmations against a temporary populated desktop profile.
export async function checkNativeDataManagement(origin) {
  const invoke = window.__TAURI_INTERNALS__.invoke;
  const assert = (value, message) => { if (!value) throw new Error(message); };
  const wait = async (read, stage = "control") => { const end = Date.now() + 15000; while (Date.now() < end) { const value = await read(); if (value) return value; await new Promise(resolve => setTimeout(resolve, 50)); } throw new Error('Native data management deadline: ' + stage + '\n' + document.body.innerText); };
  const button = text => [...document.querySelectorAll('button')].find(el => el.textContent.trim() === text && el.getClientRects().length);
  const spec = { version: 3, metadata: { name: 'Cleanup cancellation fixture' }, extractor: { kind: 'jsonld', url: origin + '/slow' }, schedule: { cadence: 'manual', timezone: 'UTC' } };
  const source = await invoke('save_workflow', { spec, id: null });
  const started = invoke('run_source_now', { id: source.id }).then(run => ({ run }), error => ({ error }));
  await wait(async () => (await invoke('get_scheduler_status')).running, 'active runner');
  await wait(async () => (await (await fetch(origin + '/seen-slow')).json()).seen, 'worker HTTP request');
  (await wait(() => button('SETTINGS'), 'Settings navigation')).click();
  (await wait(() => button('DATA MANAGEMENT'), 'Data management navigation')).click();
  async function open(name) {
    (await wait(() => button(name), name)).click();
    await new Promise(resolve => setTimeout(resolve, 0));
    await wait(() => document.querySelector('dialog[open] [aria-label="Cleanup counts"]'), name + ' counts');
    if (name !== 'Review reset') await wait(() => button('Permanently delete') && !button('Permanently delete').disabled, name + ' enabled confirmation');
    assert(document.activeElement === button('Cancel'), 'Initial cleanup focus must be Cancel');
  }
  await open('Review keep favorites');
  assert(document.querySelector('[aria-label="Cleanup counts"]').textContent.includes('Preserve 1 postings (1 saved records)'), 'Favorite preservation preview is wrong');
  button('Cancel').click();
  await wait(() => !document.querySelector('dialog[open]'), 'cancelled confirmation');
  assert((await invoke('list_jobs', { filter: {} })).total === 1, 'Cancel deleted jobs');
  await open('Review keep favorites'); button('Permanently delete').click();
  await wait(() => !document.querySelector('dialog[open]') && document.body.textContent.includes('Recovery backup:'));
  const completed = await started;
  assert(!completed.error && completed.run.status !== 'success' && completed.run.error?.includes('Worker cancelled'), 'Active scrape did not finish with an explicit cancellation failure');
  assert(!(await invoke('get_scheduler_status')).running, 'Cleanup returned while worker was active');
  let saved = await invoke('get_job', { id: 'saved' });
  assert(saved.notes === 'Fixture saved note' && saved.favorited && saved.status === 'applied', 'Favorite cleanup changed saved state');
  assert(!(await invoke('get_settings')).scraper_enabled, 'Cleanup did not pause automatic scraping');
  await open('Review keep tracked'); button('Permanently delete').click();
  await wait(() => !document.querySelector('dialog[open]'));
  saved = await invoke('get_job', { id: 'saved' });assert(saved.status === 'applied' && saved.notes === 'Fixture saved note', 'Tracked cleanup lost application');
  await open('Review all jobs'); button('Permanently delete').click();
  await wait(() => !document.querySelector('dialog[open]'));
  assert((await invoke('list_jobs', { filter: {} })).total === 0, 'Delete all retained jobs');
  assert((await invoke('list_sources')).length >= 2, 'Delete all removed workflows');
  await open('Review reset');
  assert(button('Permanently delete').disabled, 'Reset confirmation was not required');
  const input = [...document.querySelectorAll('label')].find(el => el.textContent.includes('Type RESET to confirm')).querySelector('input');
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, 'RESET');input.dispatchEvent(new Event('input', { bubbles: true }));
  await wait(() => !button('Permanently delete').disabled);button('Permanently delete').click();
  await wait(() => !document.querySelector('dialog[open]'));
  assert((await invoke('list_sources')).length === 0, 'Full reset retained workflows');
  assert(!(await invoke('get_settings')).scraper_enabled, 'Full reset enabled scheduling');
  assert((await invoke('list_scrape_runs', { limit: 100 })).length === 0, 'Full reset retained run logs');
  assert(document.body.textContent.includes('Recovery backup:'), 'Recovery path missing after reset');
  return { modes: 4, actualWorkerCancelled: true, favoriteAndTrackedStatePreserved: true, resetPaused: true, database: 'isolated temporary profile only' };
}
