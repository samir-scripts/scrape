import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, writeFile, readFile, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createServer } from "node:net";
import { createHash } from "node:crypto";

const fixture = String.raw`
import sqlite3,hashlib,pathlib,sys
root=pathlib.Path(sys.argv[1]); database=sys.argv[2]; mode=sys.argv[3]
c=sqlite3.connect(database)
c.executescript('CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY, description TEXT NOT NULL, installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, success BOOLEAN NOT NULL, checksum BLOB NOT NULL, execution_time BIGINT NOT NULL);')
for migration in sorted((root/'src-tauri/migrations').glob('*.sql')):
 version=int(migration.name.split('_')[0])
 if version>12:break
 sql=migration.read_bytes()
 if version==12 and mode!='current':sql=(root/'src-tauri/historical-migrations/0012_posting_removal.sql').read_bytes()
 c.executescript(sql.decode())
 checksum=hashlib.sha384(sql).digest() if not(version==12 and mode=='unknown') else bytes.fromhex('12345678')
 c.execute('INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES (?,?,1,?,0)',(version,migration.stem.split('_',1)[1].replace('_',' '),checksum))
 c.commit()
c.execute("INSERT INTO sources(id,kind,name,identifier,created_at,enabled,cadence) VALUES ('fixture','greenhouse','Fixture Co','fixture','2026-01-01',0,'manual')")
c.execute("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited) VALUES ('saved','fixture:42','fixture','42','Fixture engineer','Fixture Co','full_time','https://example.com/42','2026-01-01','2026-01-01','applied','2026-01-01','Fixture saved note',1)")
c.execute("INSERT INTO app_settings VALUES ('scraper.enabled','false','2026-01-01')")
c.commit();c.close()
`;

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
for (const mode of ["fresh", "legacy", "current", "unknown"]) {
  const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-"));
  const database = join(directory, "scrape_data.db");
  if (mode !== "fresh") execFileSync("python3", ["-c", fixture, resolve("."), database, mode]);
  const before = mode === "unknown" ? hash(await readFile(database)) : null;
  await writeFile(join(directory, "smoke.js"), `(async () => {
    try {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const jobs = await invoke("list_jobs", {filter:{}});
      ${mode === "fresh" ? "" : "if (jobs.items[0]?.notes !== 'Fixture saved note' || !jobs.items[0]?.favorited || jobs.items[0]?.status !== 'applied') throw new Error('Saved state changed');"}
      await invoke("plugin:event|emit", {event:"native-smoke-result",payload:{ok:true,steps:["actual Tauri dev readiness: ${mode}"]}});
    } catch(error) {
      await window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {event:"native-smoke-result",payload:{ok:false,error:String(error)}});
    }
  })();`);
  // Use an isolated port while retaining the actual configured build/dev pipeline.
  const reservation = createServer();
  await new Promise(resolve => reservation.listen(0, "127.0.0.1", resolve));
  const port = reservation.address().port;
  await new Promise(resolve => reservation.close(resolve));
  const config = JSON.stringify({build:{
    beforeDevCommand:`npm run build:sidecar && npm run dev -- --host 127.0.0.1 --port ${port}`,
    devUrl:`http://127.0.0.1:${port}`,
  }});
  const child = spawn("npm", ["run", "tauri", "--", "dev", "--no-watch", "--config", config], {
    env: {...process.env, SCRAPE_NATIVE_SMOKE_DIR:directory}, detached:true,
    stdio:["ignore","pipe","pipe"],
  });
  let output = "";
  child.stdout.on("data", bytes => output += bytes);
  child.stderr.on("data", bytes => output += bytes);
  const timer = setTimeout(() => { try { process.kill(-child.pid, "SIGKILL"); } catch {} }, 120000);
  let code, signal;
  try {
    [code, signal] = await new Promise(resolve => child.once("exit", (code,signal) => resolve([code,signal])));
  } finally {
    clearTimeout(timer);
    // Clean up only this test's owned dev server/process group.
    try { process.kill(-child.pid, "SIGTERM"); } catch {}
  }
  await writeFile(join(directory, "dev-startup.log"), output);
  const invalid = signal || /panicked|non-unwinding|SIGABRT/.test(output);
  if (mode === "unknown") {
    if (invalid || code === 0 || !output.includes("STARTUP_FAILED:") || !output.includes("version 12") || hash(await readFile(database)) !== before)
      throw new Error(`Unknown history did not fail safely: ${directory}\n${output}`);
    if ((await readdir(directory)).some(name => name.includes("pre-migration"))) throw new Error("Rejected history was backed up/mutated");
  } else if (invalid || code !== 0 || !output.includes('"ok":true')) {
    throw new Error(`Native startup failed for ${mode}: ${directory}\n${output}`);
  }
  console.log(`PASS actual Tauri dev startup ${mode}; artifacts ${directory}`);
}
