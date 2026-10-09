import { execSync } from 'child_process';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const rootDir = path.resolve(__dirname, '..');

let target = process.env.TAURI_ENV_TARGET_TRIPLE;
if (!target) {
  try {
    const env = {
      ...process.env,
      PATH: `${process.env.HOME || ''}/.cargo/bin:${process.env.PATH}`,
    };
    const rustcOutput = execSync('rustc -vV', { encoding: 'utf8', env });
    const match = rustcOutput.match(/host:\s*([^\s]+)/);
    if (match) {
      target = match[1];
    }
  } catch (err) {
    console.error('[build-sidecar] Failed to query rustc for host triple:', err);
    process.exit(1);
  }
}

if (!target) {
  console.error('[build-sidecar] Could not determine target triple');
  process.exit(1);
}

console.log(`[build-sidecar] Building scrape-worker for target: ${target}`);

const isWindows = target.includes('windows');
const exeExt = isWindows ? '.exe' : '';

const env = {
  ...process.env,
  PATH: `${process.env.HOME || ''}/.cargo/bin:${process.env.PATH}`,
};

execSync(`cargo build -p scrape-worker --release --target ${target}`, {
  cwd: rootDir,
  stdio: 'inherit',
  env,
});

const srcBin = path.join(rootDir, 'target', target, 'release', `scrape-worker${exeExt}`);
const destDir = path.join(rootDir, 'src-tauri', 'binaries');
const destBin = path.join(destDir, `scrape-worker-${target}${exeExt}`);

if (!fs.existsSync(destDir)) {
  fs.mkdirSync(destDir, { recursive: true });
}

fs.copyFileSync(srcBin, destBin);
if (!isWindows) {
  fs.chmodSync(destBin, 0o755);
}

console.log(`[build-sidecar] Successfully copied sidecar binary to ${destBin}`);
