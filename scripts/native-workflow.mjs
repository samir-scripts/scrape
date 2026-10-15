// Uses the actual desktop wizard and local HTTP -> sidecar -> preview/save path.
// File-chooser gestures are separately verified in browser tests, not simulated here.
export async function checkNativeWorkflow(origin) {
 const invoke = window.__TAURI_INTERNALS__.invoke;
 const assert = (value, message) => { if (!value) throw new Error(message); };
 const wait = async read => { const end = Date.now() + 20000; while (Date.now() < end) { const value = await read(); if (value) return value; await new Promise(resolve => setTimeout(resolve, 50)); } throw new Error('Native workflow deadline'); };
 const button = text => [...document.querySelectorAll('button')].find(element => element.textContent.trim() === text && element.getClientRects().length);
 const field = label => document.querySelector(`[aria-label="${label}"]`);
 const change = (input, value) => { Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value); input.dispatchEvent(new Event('input', { bubbles: true })); };
 const before = await invoke('list_jobs', { filter: {} }); const runs = await invoke('list_scrape_runs', { limit: 100 });
 (await wait(() => button('SCRAPER'))).click();
 (await wait(() => [...document.querySelectorAll('button')].find(element => element.textContent.includes('ADD WORKFLOW')))).click();
 await wait(() => field('Source URL or board identifier'));
 change(field('Source URL or board identifier'), origin + '/careers');
 change(field('Workflow name'), 'Native wizard fixture');
 button('Continue to Matching').click(); await wait(() => button('Continue to Schedule'));
 change(field('Title keywords (any)'), 'Native workflow');
 button('Continue to Schedule').click(); await wait(() => button('Review workflow'));
 field('Schedule').value = 'manual'; field('Schedule').dispatchEvent(new Event('change', { bubbles: true }));
 button('Review workflow').click(); await wait(() => button('Preview matches'));
 button('Preview matches').click(); await wait(() => button('Save workflow') && !button('Save workflow').disabled);
 assert(document.querySelector('[aria-label="Preview results"]').textContent.includes('1 eligible matches'), 'Preview did not use current keyword rule');
 assert((await invoke('list_jobs', { filter: {} })).total === before.total, 'Preview persisted postings');
 button('Save workflow').click(); await wait(() => button('Done'));
 const sources = await invoke('list_sources'); const saved = sources.find(source => source.name === 'Native wizard fixture');
 assert(saved && saved.cadence === 'manual' && saved.filters.matching.keywords.any[0] === 'Native workflow', 'Saved spec differs from reviewed rules');
 assert((await invoke('list_jobs', { filter: {} })).total === before.total, 'Save scraped implicitly');
 assert((await invoke('list_scrape_runs', { limit: 100 })).length === runs.length, 'Save created an execution run');
 button('Done').click();
 return { source: saved.name, cadence: saved.cadence, previewMatched: 1, persistedJobsDuringPreviewAndSave: 0, input: 'actual WKWebView wizard controls and actual local worker; native file chooser remains manual QA' };
}
