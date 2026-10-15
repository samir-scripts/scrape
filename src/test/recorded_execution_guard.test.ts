import { expect, it, vi } from 'vitest';
vi.mock('../lib/runtime', async original => ({ ...await original<typeof import('../lib/runtime')>(), isRecordedMode: () => true }));
import { api } from '../lib/api';
it('rejects every recorded-mode execution path including saved and transient tests', async () => {
 await expect(api.testSource('recorded')).rejects.toThrow(/Recorded examples cannot execute/);
 await expect(api.testSourceTransient({ kind: 'greenhouse', name: 'Recorded', identifier: 'fixture', config: null, filters: null })).rejects.toThrow(/Recorded examples cannot execute/);
 await expect(api.runScrapeNow()).rejects.toThrow(/Recorded examples cannot execute/);
});
