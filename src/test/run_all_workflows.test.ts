import { expect, it, vi } from 'vitest';
import { api, SAMPLE_ENGINEERING_SOURCE } from '../lib/api';
it('fixture Run all dispatches every enabled workflow and never falls back to a disabled source', async () => {
 for (const source of await api.listSources()) await api.updateSource({ id: source.id, enabled: false });
 const first = await api.createSource({ kind: 'greenhouse', name: 'First authored fixture', identifier: 'fixture-first', enabled: true, filters: { include_keywords: ['Engineer'] } });
 const second = await api.createSource({ kind: 'greenhouse', name: 'Second authored fixture', identifier: 'fixture-second', enabled: true, filters: { include_keywords: ['Designer'] } });
 const run = vi.spyOn(api, 'runSourceNow').mockImplementation(async id => ({ id: `run-${id}`, source_id: id, trigger: 'manual', status: 'success', started_at: '2026-10-15', finished_at: '2026-10-15', jobs_found: 2n, jobs_new: 1n, error: null }));
 const result = await api.runScrapeNow(); expect(run.mock.calls.map(([id]) => id)).toEqual([first.id, second.id]); expect(result.jobs_found).toBe(4n);
 expect((await api.listSources()).find(source => source.id === second.id)?.filters).toEqual({ include_keywords: ['Designer'] });
 await api.updateSource({ id: first.id, enabled: false }); await api.updateSource({ id: second.id, enabled: false }); run.mockClear();
 await api.runScrapeNow(); expect(run).not.toHaveBeenCalled();
 run.mockRestore(); await api.deleteSource(first.id); await api.deleteSource(second.id); await api.updateSource({ id: SAMPLE_ENGINEERING_SOURCE.id, enabled: true });
});
