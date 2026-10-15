import { checkNativeJobLayout } from "./native-job-layout.mjs";
import { runNativeDev } from "./tauri-dev-test.mjs";
import { checkNativeAppearance } from "./native-appearance.mjs";
import { execFileSync } from "node:child_process";
import { mkdtemp, writeFile, readFile, readdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { createHash } from "node:crypto";

const fixture = String.raw`
import sqlite3,hashlib,pathlib,sys
root=pathlib.Path(sys.argv[1]); database=sys.argv[2]; mode=sys.argv[3]
c=sqlite3.connect(database)
c.executescript('CREATE TABLE _sqlx_migrations(version BIGINT PRIMARY KEY, description TEXT NOT NULL, installed_on TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP, success BOOLEAN NOT NULL, checksum BLOB NOT NULL, execution_time BIGINT NOT NULL);')
for migration in sorted((root/'src-tauri/migrations').glob('*.sql')):
 version=int(migration.name.split('_')[0])
 if version>(19 if mode.endswith("19") else 12):break
 sql=migration.read_bytes()
 if version==12 and mode in ('legacy','unknown'):sql=(root/'src-tauri/historical-migrations/0012_posting_removal.sql').read_bytes()
 if version==19 and mode in ('legacy19','unknown19'):sql=(root/'src-tauri/historical-migrations/0019_structured_job_descriptions.sql').read_bytes()
 c.executescript(sql.decode())
 checksum=hashlib.sha384(sql).digest() if not((version==12 and mode=='unknown') or (version==19 and mode=='unknown19')) else bytes.fromhex('12345678')
 c.execute('INSERT INTO _sqlx_migrations(version,description,success,checksum,execution_time) VALUES (?,?,1,?,0)',(version,migration.stem.split('_',1)[1].replace('_',' '),checksum))
 c.commit()
c.execute("INSERT INTO sources(id,kind,name,identifier,created_at,enabled,cadence) VALUES ('fixture','greenhouse','Fixture Co','fixture','2026-01-01',0,'manual')")
c.execute("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,notes,favorited) VALUES ('saved','fixture:42','fixture','42','Fixture engineer','Fixture Co','full_time','https://example.com/42','2026-01-01','2026-01-01','applied','2026-01-01','Fixture saved note',1)")
c.execute("INSERT INTO app_settings VALUES ('scraper.enabled','false','2026-01-01')")
if sys.argv[4]=='layout':
 for i in range(105):
  identity=f'layout-{i:03d}'
  c.execute("INSERT INTO jobs(id,dedupe_key,source_id,external_id,title,company,role_type,url,date_added,last_seen_at,status,status_changed_at,description_text) VALUES (?,?, 'fixture',?,?,'Fixture Co','full_time',?,'2026-01-02','2026-01-02','viewed','2026-01-02',?)",(identity,identity,str(i),f'Layout fixture {i:03d}',f'https://fixture.example.test/jobs/{i}', 'Authored native layout fixture.\n\n'*150))
c.commit();c.close()
`;

const hash = bytes => createHash("sha256").update(bytes).digest("hex");
const supportedModes = ["fresh", "legacy", "current", "unknown", "legacy19", "current19", "unknown19"];
const modes = process.env.SCRAPE_STARTUP_MODES?.split(",") ?? supportedModes;
if (modes.some(mode => !supportedModes.includes(mode))) throw new Error("Unsupported startup fixture mode");
for (const mode of modes) {
  const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-"));
  const database = join(directory, "scrape_data.db");
  if (mode !== "fresh") execFileSync("python3", ["-c", fixture, resolve("."), database, mode, process.env.SCRAPE_NATIVE_LAYOUT === "1" ? "layout" : "normal"]);
  const before = mode.startsWith("unknown") ? hash(await readFile(database)) : null;
  await writeFile(join(directory, "smoke.js"), `(async () => {
    try {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const jobs = await invoke("list_jobs", {filter:{}});
      ${mode === "fresh" ? "" : "const saved = await invoke('get_job', {id:'saved'}); if (saved.notes !== 'Fixture saved note' || !saved.favorited || saved.status !== 'applied') throw new Error('Saved state changed');"}
      const appearance = ${process.env.SCRAPE_NATIVE_APPEARANCE === "1" ? `await (${checkNativeAppearance.toString()})()` : "null"};
      const layout = ${process.env.SCRAPE_NATIVE_LAYOUT === "1" ? `await (${checkNativeJobLayout.toString()})()` : "null"};
      await invoke("plugin:event|emit", {event:"native-smoke-result",payload:{ok:true,appearance,layout,steps:["actual Tauri dev readiness: ${mode}"]}});
    } catch(error) {
      await window.__TAURI_INTERNALS__.invoke("plugin:event|emit", {event:"native-smoke-result",payload:{ok:false,error:String(error)}});
    }
  })();`);
  const { code, signal, output } = await runNativeDev(directory, {
    width: Number(process.env.SCRAPE_NATIVE_WIDTH || 1280),
    height: process.env.SCRAPE_NATIVE_APPEARANCE === "1" ? 600 : 800,
    scrollbarMode: process.env.SCRAPE_SCROLLBAR_MODE || "Automatic",
  });
  const invalid = signal || /panicked|non-unwinding|SIGABRT/.test(output);
  if (mode.startsWith("unknown")) {
    if (invalid || code === 0 || !output.includes("STARTUP_FAILED:") || !output.includes(`version ${mode === "unknown19" ? 19 : 12}`) || hash(await readFile(database)) !== before)
      throw new Error(`Unknown history did not fail safely: ${directory}\n${output}`);
    if ((await readdir(directory)).some(name => name.includes("pre-migration"))) throw new Error("Rejected history was backed up/mutated");
  } else if (invalid || code !== 0 || !output.includes('"ok":true')) {
    throw new Error(`Native startup failed for ${mode}: ${directory}\n${output}`);
  }
  console.log(`PASS actual Tauri dev startup ${mode}; artifacts ${directory}`);
}
