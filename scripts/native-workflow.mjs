// Uses the actual desktop wizard and local HTTP -> sidecar -> preview/save path.
// File-chooser gestures are separately verified in browser tests, not simulated here.
export async function checkNativeWorkflow(origin) {
 const invoke = window.__TAURI_INTERNALS__.invoke;
 const started = Date.now();
 const state = window.__SCRAPE_NATIVE_WORKFLOW_STATE__ = { step: 'starting', polls: 0 };
 const snapshot = () => ({ ...state, elapsedMs: Date.now() - started, visibility: document.visibilityState, focused: document.hasFocus(), active: document.activeElement?.getAttribute('aria-label'), country: field('Country')?.value, stateDisabled: field('State/province')?.disabled });
 const mark = async step => { state.step = step; state.polls = 0; await invoke('plugin:event|emit', { event: 'native-smoke-log', payload: snapshot() }); };
 const assert = (value, message) => { if (!value) throw new Error(message); };
 const wait = async (read, label) => {
  await mark('Waiting for ' + label);
  const end = Date.now() + 20000;
  while (Date.now() < end) {
   state.polls++;
   const value = await read();
   if (value) return value;
   await new Promise(resolve => setTimeout(resolve, 50));
  }
  throw new Error('Native workflow deadline: ' + label + ' ' + JSON.stringify(snapshot()));
 };
 const settle = async label => {
  await mark(label + ': timer pending');
  await new Promise(resolve => setTimeout(resolve, 0));
  await mark(label + ': timer fired');
 };
 const button = text => [...document.querySelectorAll('button')].find(element => element.textContent.trim() === text && element.getClientRects().length);
 const nav = text => [...document.querySelectorAll('button')].find(element => element.textContent.trim().startsWith(text) && element.getClientRects().length);
 const field = label => document.querySelector(`input[aria-label="${label}"], textarea[aria-label="${label}"], button[aria-label="${label}"][aria-haspopup="listbox"]`);
 const change = (input, value) => { input.focus(); Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, value); input.dispatchEvent(new Event('input', { bubbles: true })); };
 const scheduleTrigger = await wait(() => document.querySelector('button[aria-label^="Schedule:"]'), 'scheduler status');
 scheduleTrigger.click();
 await wait(() => document.querySelector('dialog[aria-label="Schedule summary"][open]'), 'scheduler summary');
 if (button('Pause automatic runs')) button('Pause automatic runs').click();
 await wait(() => button('Resume automatic runs'), 'automatic scheduling paused');
 document.querySelector('dialog[aria-label="Schedule summary"] button[aria-label="Close dialog"]').click();
 await wait(() => !document.querySelector('dialog[aria-label="Schedule summary"][open]'), 'closed scheduler summary');
 const before = await invoke('list_jobs', { filter: {} }); const runs = await invoke('list_scrape_runs', { limit: 100 });
 (await wait(() => button('SCRAPER'), 'Scraper navigation')).click();
 (await wait(() => [...document.querySelectorAll('button')].find(element => element.textContent.includes('ADD WORKFLOW')), 'Add workflow')).click();
 await wait(() => field('Workflow name'), 'role-first wizard');
 change(field('Workflow name'), 'Native wizard fixture');
 await settle('Workflow name input');
 change(field('All keywords'), 'Native workflow,engineer');
 await settle('All keywords input');
 assert(field('Any keywords') && field('Exclude keywords') && !field('Match keywords'), 'Keyword groups must be available together without a mode selector');
 button('Continue to Criteria').click(); await wait(() => button('Continue to Sources'), 'criteria step');
 field('Date posted range').click();
 (await wait(() => [...document.querySelectorAll('[role=option]')].find(option => option.textContent.trim() === 'Any time'), 'unrestricted fixture posting dates')).click();
 await wait(() => field('Date posted range').textContent.includes('Any time') && !document.querySelector('[role=option]'), 'fixture posting-date rule');
 await mark('Entering Country US');
 change(field('Country'), 'US');
 await settle('Country input');
 field('Country').blur();
 await wait(() => field('Country').value === 'United States' && !field('State/province').disabled, 'committed United States and enabled subdivisions');
 change(field('State/province'), 'TX');
 await settle('State/province input');
 field('State/province').dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
 await wait(() => document.querySelector('button[aria-label="Remove Texas"]'), 'committed Texas subdivision');
 [...document.querySelectorAll('label')].find(label => label.textContent.trim() === 'Remote').querySelector('input').click();
 const fullTime = [...document.querySelectorAll('label')].find(label => label.textContent.trim() === 'Full-time')?.querySelector('input');
 assert(fullTime, 'Employment type controls are missing');
 if (!fullTime.checked) fullTime.click();
 (await wait(() => [...document.querySelectorAll('[aria-label="Years of experience"] button')].find(element => element.textContent === '0–2'), 'minimum-required 0–2 preset')).click();
 await wait(() => field('Minimum experience (years)')?.value === '0', 'committed 0–2 minimum preset');
 button('New Grad only').click();
 await wait(() => document.querySelector('[aria-label="New Grad eligibility"] button[aria-pressed="true"]')?.textContent === 'New Grad only', 'New Grad only selected');
 assert(field('Minimum experience (years)').value === '0' && field('Maximum experience (years)').value === '2', 'New Grad changed numeric YOE');
 const unknownExperience = [...document.querySelectorAll('label')].find(label => label.textContent.includes('Include jobs with unknown required experience'))?.querySelector('input');
 assert(unknownExperience && !unknownExperience.checked, 'New numeric rules must exclude unknown minimums');
 button('Continue to Sources').click(); await wait(() => button('+ Add source'), 'sources step');
 assert(!field('Source URL or board identifier'), 'Default public search must not require a careers URL');
 document.querySelector('button[aria-label="Add source"]').click();
 await wait(() => field('Source 2 URL or board identifier'), 'optional direct board slot');
 document.querySelector('button[aria-label="Remove source Cross-Company Search"]').click();
 const company = await wait(() => document.querySelector("[data-source-index='0'] [aria-label='Search companies']"), 'company picker in attached source slot');
 change(company, 'cloud flare');
 await settle('Company query');
 const option = await wait(() => [...document.querySelectorAll('[role=option]')].find(option => option.textContent.includes('Cloudflare')), 'Cloudflare catalog option');
 option.dispatchEvent(new MouseEvent('mousedown', { bubbles: true }));
 option.dispatchEvent(new MouseEvent('mouseup', { bubbles: true }));
 option.click();
 company.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
 await wait(() => document.querySelector("[data-source-index='0']").textContent.includes('Cloudflare'), 'selected company details');
 (await wait(() => button('Enter a custom careers URL'), 'custom URL action')).click();
 await wait(() => field('Source URL or board identifier'), 'wizard custom source field');
 change(field('Source URL or board identifier'), origin + '/careers');
 await settle('Source URL input');
 document.querySelector("[data-source-index='0'] details > summary").click();
 change(field('Source 1 employer hint'), 'Fixture Co');
 await settle('Explicit custom employer identity');
 button('Continue to Schedule').click(); await wait(() => button('Save workflow'), 'schedule preview and save step');
 field('Schedule').focus();
 field('Schedule').click();
 (await wait(() => [...document.querySelectorAll('[role=option]')].find(option => option.textContent.trim() === 'manual'), 'manual cadence option')).click();
 await wait(() => field('Schedule').textContent.includes('manual') && !document.querySelector('[role=option]'), 'manual cadence');
 button('Preview matches').click(); await wait(() => button('Save workflow') && !button('Save workflow').disabled, 'completed read-only preview');
 assert(document.querySelector('[aria-label="Preview results"]').textContent.includes('1 eligible matches'), 'Preview did not use current keyword rule');
 assert((await invoke('list_jobs', { filter: {} })).total === before.total, 'Preview persisted postings');
 const previewLogo = document.querySelector('[aria-label="Preview results"] [data-logo-status="missing_metadata"]');
 assert(previewLogo && previewLogo.title.includes('Fixture Co') && previewLogo.title !== 'Fixture Co', 'Preview must explain genuinely absent employer artwork');
 button('Back').click(); await wait(() => button('Continue to Schedule'), 'sources after preview');
 button('Back').click(); await wait(() => button('Continue to Sources'), 'criteria after preview');
 change(field('Maximum experience (years)'), '1');
 await settle('Numeric edit invalidates preview');
 change(field('Maximum experience (years)'), '2');
 await settle('Restore numeric bound without restoring stale preview');
 button('Continue to Sources').click(); await wait(() => button('Continue to Schedule'), 'sources after rule edit');
 button('Continue to Schedule').click(); await wait(() => button('Save workflow')?.disabled, 'stale preview cannot save');
 button('Preview matches').click(); await wait(() => button('Save workflow') && !button('Save workflow').disabled, 'fresh preview after rule edit');
 assert((await invoke('list_jobs', { filter: {} })).total === before.total, 'Repeated preview persisted jobs');
 button('Save workflow').click(); await wait(() => button('Done'), 'saved workflow confirmation');
 const sources = await invoke('list_sources'); const saved = sources.find(source => source.name === 'Native wizard fixture');
 assert(saved && saved.cadence === 'manual' && saved.filters.matching.keywords.any.length === 0 && saved.filters.matching.keywords.all.join('|') === 'Native workflow|engineer', 'Saved spec differs from reviewed rules: ' + JSON.stringify({ cadence: saved?.cadence, keywords: saved?.filters?.matching?.keywords }));
 assert(saved.filters.matching.countries.join() === 'US' && saved.filters.matching.regions.join() === 'US-TX' && saved.filters.matching.workplaces.join() === 'remote', 'Native geography controls lost their effective rules');
 assert(saved.filters.matching.role_types.join() === 'full_time' && saved.filters.matching.experience_min === 0 && saved.filters.matching.experience_max === 2 && saved.filters.matching.experience_match_mode === 'required_minimum' && saved.filters.matching.new_grad === 'only' && !saved.filters.matching.experience_include_unknown, 'Native employment/YOE/New Grad controls lost their effective rules: ' + JSON.stringify(saved.filters.matching));
 assert((await invoke('list_jobs', { filter: {} })).total === before.total, 'Save scraped implicitly');
 assert((await invoke('list_scrape_runs', { limit: 100 })).length === runs.length, 'Save created an execution run');
 button('Done').click();
 const getRow = async () => await wait(() => [...document.querySelectorAll('tr')].find(element => element.textContent.includes(saved.name)), 'saved workflow row');
 const run = async () => {
  const previous = (await invoke('list_source_runs', { sourceId: saved.id, limit: 100 })).map(item => item.id);
  const targetRow = await getRow();
  const control = [...targetRow.querySelectorAll('button')].find(element => element.textContent.trim() === 'RUN');
  assert(control && !control.disabled, 'Saved native workflow is not runnable');
  control.click();
  // Row RUN already launches the worker and opens the logs dock; its dock
  // button is a second launch control, not a confirmation.
  const completed = await wait(async () => (await invoke('list_source_runs', { sourceId: saved.id, limit: 100 })).find(item => !previous.includes(item.id) && item.status !== 'running'), 'one completed workflow execution');
  assert(completed.status === 'success', 'Native workflow did not succeed: ' + JSON.stringify(completed));
  await wait(() => document.body.textContent.includes('inserted ' + completed.counts.inserted), 'rendered workflow counters');
  await wait(() => { const el = button('RUN WORKFLOW NOW'); return el && !el.disabled; }, 'idle workflow execution controls');
  const launched = (await invoke('list_source_runs', { sourceId: saved.id, limit: 100 })).filter(item => !previous.includes(item.id));
  assert(launched.length === 1 && launched[0].id === completed.id, 'A single Run gesture launched more than one execution: ' + JSON.stringify(launched));
  assert(completed.counts.fetched === 5 && completed.counts.rejected === 4 && completed.counts.failed === 0, 'Native counters do not explain keyword/employment/YOE/New Grad rejections: ' + JSON.stringify(completed.counts));
  return completed;
 };
 const first = await run();
 assert(first.counts.inserted === 1 && first.counts.updated === 0 && first.jobs_new === 1, 'First native run did not insert exactly one job');
 const jobs = await invoke('list_jobs', { filter: { search: 'Native workflow engineer' } });
 assert(jobs.total === 1 && jobs.items[0].url === origin + '/jobs/wizard-1', 'Native worker identity/destination did not survive ingestion');
 const job = jobs.items[0];
 assert(job.role_type === 'full_time' && job.experience_min === 1 && job.is_new_grad === 'yes' && job.experience_evidence, 'Native ingestion lost structured employment or required-experience evidence: ' + JSON.stringify(job));
 (await wait(() => nav('DISCOVERED'), 'Discovered navigation')).click();
 const search = await wait(() => field('Search jobs'), 'Discovered search');
 change(search, 'Native workflow engineer');
 const card = await wait(() => [...document.querySelectorAll('button')].find(element => element.getAttribute('aria-label')?.startsWith('View Native workflow engineer ')), 'discovered native posting');
 const cardContainer = card.closest('.group') || card.parentElement;
 const missingLogo = cardContainer.querySelector('[data-logo-status="missing_metadata"]');
 assert(missingLogo && missingLogo.getAttribute('aria-label') === 'Fixture Co initials' && missingLogo.title !== 'Fixture Co', 'Discovered must retain honest missing-logo initials and reason: ' + (missingLogo ? missingLogo.outerHTML : cardContainer?.innerHTML));
 card.click();
 await wait(() => document.body.textContent.includes(job.experience_evidence), 'inspector requirement evidence');
 (await wait(() => button('Mark as applied'), 'native posting status control')).click();
 await wait(async () => (await invoke('get_job', { id: job.id })).status === 'applied', 'persisted Applied status');
 (await wait(() => button('SCRAPER'), 'Scraper navigation after tracking')).click();
 const refreshed = await run();
 assert(refreshed.counts.inserted === 0 && refreshed.counts.updated === 1 && refreshed.jobs_new === 0, 'Refresh was presented as a new discovery');
 assert((await invoke('get_job', { id: job.id })).status === 'applied', 'Refresh lost tracked status');
 (await wait(() => nav('DISCOVERED'), 'Discovered navigation after refresh')).click();
 await wait(() => field('Search jobs'), 'saved Discovered search after refresh');
 assert(field('Search jobs').value === 'Native workflow engineer', 'Run completion changed the saved Discovered search');
 await wait(() => document.body.textContent.includes('NO POSTINGS FOUND'), 'tracked posting excluded from Discovered');
 assert(![...document.querySelectorAll('button')].some(element => element.getAttribute('aria-label')?.startsWith('View Native workflow engineer ')), 'Tracked posting returned to Discovered after refresh');
 const finalRuns = await invoke('list_source_runs', { sourceId: saved.id, limit: 100 });
 assert(finalRuns.length === 2 && new Set(finalRuns.map(item => item.id)).size === 2, 'Two Run gestures did not create exactly two executions: ' + JSON.stringify(finalRuns));
 assert((await invoke('list_scrape_runs', { limit: 100 })).length === runs.length + 2, 'Native workflow launched an unexpected execution');
 await mark('Starting role-first public search fixture');
 (await wait(() => button('SCRAPER'), 'search workflow registry')).click();
 (await wait(() => button('Add workflow'), 'new role-first workflow')).click();
 await new Promise(resolve => requestAnimationFrame(resolve));
 const rolePicker = await wait(() => field('Target roles'), 'role picker mount');
 change(rolePicker, 'Software Engineer');
 (await wait(() => [...document.querySelectorAll('[role="option"]')].find(option => option.textContent.trim().startsWith('Software Engineer')), 'O*NET occupation role')).click();
 const companyPicker = field('Companies (Optional)');
 for (const company of ['Google', 'Amazon']) {
  change(companyPicker, company);
  (await wait(() => [...document.querySelectorAll('[role="option"]')].find(option => option.textContent.trim().startsWith(company)), company + ' employer option')).click();
 }
 await wait(() => document.body.textContent.includes('Software Engineer at Google & Amazon'), 'role/company-derived workflow name');
 button('Continue to Criteria').click();
 await wait(() => field('Date posted range'), 'role-first criteria');
 field('Date posted range').click();
 (await wait(() => [...document.querySelectorAll('[role=option]')].find(option => option.textContent.trim() === 'Past 12 hours'), '12-hour preset')).click();
 button('Continue to Sources').click();
 await wait(() => button('Continue to Schedule'), 'public search sources');
 assert(!field('Source URL or board identifier'), 'Default search displayed a mandatory career URL');
 button('Advanced settings').click();
 (await wait(() => button('Edit YAML'), 'search YAML controls')).click();
 const searchYaml = await wait(() => document.querySelector('textarea[aria-label="Workflow YAML"]'), 'search YAML');
 const withEndpoint = searchYaml.value.replace(/^(\s+)url:.*$/m,
   (_, indent) => `${indent}url: ""\n${indent}source_config:\n${indent}  endpoint: "${origin}/search"`);
 assert(withEndpoint !== searchYaml.value, 'Search fixture endpoint was not configured');
 Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype, 'value').set.call(searchYaml, withEndpoint);
 searchYaml.dispatchEvent(new Event('input', { bubbles: true }));
 button('Apply YAML to form').click();
 await wait(() => !document.querySelector('textarea[aria-label="Workflow YAML"]'), 'validated search YAML');
 button('Continue to Schedule').click();
 await wait(() => button('Save workflow'), 'search workflow schedule');
 field('Schedule').click();
 (await wait(() => [...document.querySelectorAll('[role=option]')].find(option => option.textContent.trim() === 'manual'), 'search manual cadence')).click();
 const searchBefore = await invoke('list_jobs', { filter: {} });
 const searchRunsBefore = await invoke('list_scrape_runs', { limit: 100 });
 button('Preview matches').click();
 await wait(() => button('Save workflow') && !button('Save workflow').disabled, 'real public search preview');
 assert(document.querySelector('[aria-label="Preview results"]').textContent.includes('2 eligible matches'), 'Search preview lost role/company/date criteria');
 button('Save workflow').click();
 await wait(() => button('Done'), 'role-first saved confirmation');
 const savedRoleCompanies = (await invoke('list_sources')).filter(source => source.name === 'Software Engineer at Google & Amazon');
 const publicSearch = savedRoleCompanies.find(source => source.kind === 'search' && source.identifier === 'themuse');
 assert(publicSearch && savedRoleCompanies.length === 3, 'Role/company selection did not persist public search plus both employer boards: ' + JSON.stringify(savedRoleCompanies.map(({ name, kind, identifier, enabled }) => ({ name, kind, identifier, enabled }))));
 assert(publicSearch.filters.matching.posting_date.preset === 'past_12h' && !publicSearch.filters.matching.posting_date.last_hours,
   'Named search preset was frozen or combined with an explicit age');
 assert((await invoke('list_jobs', { filter: {} })).total === searchBefore.total, 'Search preview/save persisted jobs');
 assert((await invoke('list_scrape_runs', { limit: 100 })).length === searchRunsBefore.length, 'Search Save launched a run');
 button('Run now').click();
 const searchRun = await wait(async () => (await invoke('list_source_runs', { sourceId: publicSearch.id, limit: 10 })).find(run => run.status !== 'running'),
   'explicit search run completion');
 assert(searchRun.status === 'success' && searchRun.counts.inserted === 2 && searchRun.counts.rejected === 3,
   'Search run did not preserve canonical criteria/accounting: ' + JSON.stringify(searchRun));
 const discoveredSearch = await invoke('list_jobs', { filter: { source_id: publicSearch.id } });
 assert(discoveredSearch.total === 2 && discoveredSearch.items.map(job => job.external_id).sort().join() === '9001,9002',
   'Real role-first search did not reach Discovered query');
 assert((await invoke('list_source_runs', { sourceId: publicSearch.id, limit: 10 })).length === 1, 'One search Run now gesture launched multiple executions');
 await mark('Native workflows complete: board refresh and role-first search preview/save/run verified');
 return {
   source: saved.name, cadence: saved.cadence,
   keywords: { mode: 'all', terms: saved.filters.matching.keywords.all },
   geography: { countries: ['US'], regions: ['US-TX'], workplaces: ['remote'] },
   matching: { roleTypes: ['full_time'], minimumRequiredYears: [0, 2], includeUnknown: false, newGrad: 'only' },
   companyPicker: 'Cloudflare selection in source slot 0 then explicit custom loopback URL',
   logo: 'missing metadata; explanatory initials in preview and Discovered',
   requirementEvidence: job.experience_evidence, previewMatched: 1,
   stalePreviewSaveBlocked: true, persistedJobsDuringPreviewAndSave: 0,
   workflowRuns: [first.id, refreshed.id], inserted: 1, refreshed: 1,
   trackedStatusPreserved: true, destination: job.url,
   roleFirstSearch: { provider: publicSearch.identifier, roles: publicSearch.filters.matching.target_roles,
     companies: publicSearch.filters.matching.companies, preset: publicSearch.filters.matching.posting_date.preset,
     previewMatched: 2, inserted: 2, rejected: 3, workflowRuns: [searchRun.id], careerUrlRequired: false },
   input: 'authored local fixtures, actual WKWebView wizard/Run/Discovered controls, packaged worker and isolated SQLite; not live provider, verified-logo rendering or physical pointer/native file-chooser acceptance',
 };
}
