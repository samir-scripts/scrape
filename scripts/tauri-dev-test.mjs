import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { writeFile } from 'node:fs/promises';
import { join } from 'node:path';

// One launcher owns the server, process group, isolated profile, and completion log.
export async function runNativeDev(directory, { width = 1280, height = 800, scrollbarMode = 'Automatic' } = {}) {
  if (!['Automatic', 'Always', 'WhenScrolling'].includes(scrollbarMode)) throw new Error('Invalid test scrollbar mode');
  const reservation = createServer();
  await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
  const port = reservation.address().port;
  await new Promise(resolve => reservation.close(resolve));
  const config = JSON.stringify({ build: {
    beforeDevCommand: `npm run build:sidecar && npm run dev -- --host 127.0.0.1 --port ${port} --strictPort`,
    devUrl: `http://127.0.0.1:${port}`,
  }, app: { windows: [{ label: 'main', title: 'Scrape isolated startup test', visible: true, focus: true, backgroundThrottling: 'disabled', width, height }] } });
  const child = spawn('npm', ['run', 'tauri', '--', 'dev', '--no-watch', '--config', config, '--', '--', '-AppleShowScrollBars', scrollbarMode], {
    env: { ...process.env, SCRAPE_NATIVE_SMOKE_DIR: directory }, detached: true, stdio: ['ignore', 'pipe', 'pipe'],
  });
  let output = '';
  child.stdout.on('data', bytes => output += bytes);
  child.stderr.on('data', bytes => output += bytes);
  const stop = signal => { try { process.kill(-child.pid, signal); } catch {} };
  const timer = setTimeout(() => stop('SIGKILL'), 120000);
  let code, signal;
  try {
    [code, signal] = await new Promise((resolve, reject) => { child.once('exit', (code, signal) => resolve([code, signal])); child.once('error', reject); });
  } finally { clearTimeout(timer); stop('SIGTERM'); }
  await writeFile(join(directory, 'dev-startup.log'), output);
  return { code, signal, output };
}
