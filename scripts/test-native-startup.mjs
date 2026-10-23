import { checkNativeDataManagement } from "./native-data-management.mjs";
import { checkNativeSchedule } from "./native-schedule.mjs";
import { checkNativeWorkflow } from "./native-workflow.mjs";
import { createServer } from "node:http";
import { checkNativeKanban } from "./native-kanban.mjs";
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
 if version>(30 if mode.endswith("30") else (23 if mode.endswith("23") else (19 if mode.endswith("19") else 12))):break
 sql=migration.read_bytes()
 if version==12 and mode in ('legacy','unknown','legacy23'):sql=(root/'src-tauri/historical-migrations/0012_posting_removal.sql').read_bytes()
 if version==18 and mode=='legacy23':sql=(root/'src-tauri/historical-migrations/0018_finalize_posting_removal.sql').read_bytes()
 if version==19 and mode in ('legacy19','unknown19','legacy23'):sql=(root/'src-tauri/historical-migrations/0019_structured_job_descriptions.sql').read_bytes()
 if version==20 and mode=='legacy23':sql=(root/'src-tauri/historical-migrations/0020_finalize_structured_descriptions.sql').read_bytes()
 if version==23 and mode in ('legacy23','unknown23'):sql=(root/'src-tauri/historical-migrations/0023_employer_branding.sql').read_bytes()
 if version==30 and mode=='legacy30':sql=(root/'src-tauri/historical-migrations/0030_role_titles.sql').read_bytes()
 if version==30 and mode=='observed30':sql=(root/'src-tauri/historical-migrations/0030_role_titles_role_first.sql').read_bytes()
 c.executescript(sql.decode())
 checksum=hashlib.sha384(sql).digest() if not((version==12 and mode=='unknown') or (version==19 and mode=='unknown19') or (version==23 and mode=='unknown23') or (version==30 and mode=='unknown30')) else bytes.fromhex('12345678')
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
const supportedModes = ["fresh", "legacy", "current", "unknown", "legacy19", "current19", "unknown19", "legacy23", "current23", "unknown23", "legacy30", "observed30", "current30", "unknown30"];
const modes = process.env.SCRAPE_STARTUP_MODES?.split(",") ?? supportedModes;
if (modes.some(mode => !supportedModes.includes(mode))) throw new Error("Unsupported startup fixture mode");
let workflowServer;
let workflowOrigin = "";
let slowSeen = false;
if (process.env.SCRAPE_NATIVE_WORKFLOW === "1" || process.env.SCRAPE_NATIVE_DATA_MANAGEMENT === "1") {
  workflowServer = createServer((request, response) => {
    response.setHeader("Access-Control-Allow-Origin", "*");
    if (request.url === "/seen-slow") { response.end(JSON.stringify({ seen: slowSeen })); return; }
    if (request.url === "/slow") { slowSeen = true; request.on("close", () => response.destroy()); return; }
    if (request.url === "/robots.txt") { response.end("User-agent: *\nAllow: /"); return; }
    if (request.url === "/search") {
      const publication = hours => new Date(Date.now() - hours * 3_600_000).toISOString();
      const rows = [
        [9001, "Software Engineer", "Google", 6],
        [9002, "Software Engineer II", "Amazon", 8],
        [9003, "Software Engineer", "Meta", 6],
        [9004, "Software Engineer", "Google", 36],
        [9005, "Product Manager", "Google", 6],
      ];
      response.setHeader("Content-Type", "application/json");
      response.end(JSON.stringify({ page: 0, page_count: 1, results: rows.map(([id, name, company, age]) => ({
        id, name, company: { name: company }, publication_date: publication(age),
        contents: "<p>Labelled native search fixture. Build reliable systems.</p>",
        locations: [{ name: "Austin, TX" }], refs: { landing_page: `${workflowOrigin}/search-job/${id}` },
      })) }));
      return;
    }
    response.end(`<script type="application/ld+json">${JSON.stringify([
      { "@type": "JobPosting", identifier: "wizard-1", title: "Native workflow engineer", url: "/jobs/wizard-1", employmentType: "FULL_TIME", description: "Minimum qualifications: 1 year of professional experience required. Recent university graduates are eligible.", hiringOrganization: { name: "Fixture Co" }, jobLocationType: "TELECOMMUTE", applicantLocationRequirements: { "@type": "AdministrativeArea", name: "US-TX" }, jobLocation: { "@type": "Place", address: { addressLocality: "Austin", addressRegion: "TX", addressCountry: "US" } } },
      { "@type": "JobPosting", identifier: "wizard-2", title: "Unrelated role", url: "/jobs/wizard-2", hiringOrganization: { name: "Fixture Co" } },
      ...[
        ["wizard-senior", "Native workflow engineer III", "FULL_TIME", "Minimum qualifications: 5 years of professional experience required. 1 year of Kubernetes preferred."],
        ["wizard-contract", "Native workflow engineer contract", "CONTRACTOR", "No professional experience required. Recent university graduates are eligible."],
        ["wizard-unknown", "Native workflow engineer unknown", "FULL_TIME", "Join our engineering team."],
      ].map(([identifier, title, employmentType, description]) => ({ "@type": "JobPosting", identifier, title, url: "/jobs/" + identifier, employmentType, description, hiringOrganization: { name: "Fixture Co" }, jobLocationType: "TELECOMMUTE", applicantLocationRequirements: { "@type": "AdministrativeArea", name: "US-TX" }, jobLocation: { "@type": "Place", address: { addressLocality: "Austin", addressRegion: "TX", addressCountry: "US" } } })),
    ])}</script>`);
  });
  await new Promise(resolve => workflowServer.listen(0, "127.0.0.1", resolve));
  workflowOrigin = `http://127.0.0.1:${workflowServer.address().port}`;
}
try {
for (const mode of modes) {
  const directory = await mkdtemp(join(tmpdir(), "scrape-native-smoke-"));
  const database = join(directory, "scrape_data.db");
  if (mode !== "fresh") execFileSync("python3", ["-c", fixture, resolve("."), database, mode, process.env.SCRAPE_NATIVE_LAYOUT === "1" ? "layout" : "normal"]);
  const before = mode.startsWith("unknown") ? hash(await readFile(database)) : null;
  await writeFile(join(directory, "smoke.js"), `(async () => {
    try {
      const invoke = window.__TAURI_INTERNALS__.invoke;
      const jobs = await invoke("list_jobs", {filter:{}});
      const roleTitles = await invoke("search_role_titles", {query:"Software Developers"});
      if (roleTitles[0]?.title !== "Software Developers" || roleTitles[0]?.soc_code !== "15-1252" || roleTitles[0]?.priority_tier !== 1)
        throw new Error("Native indexed role search did not return the canonical title and provenance");
      ${mode === "fresh" ? "" : "const companies = await invoke('search_companies', {query:'Fixture'}); if (!companies.some(company => company.name === 'Fixture Co' && company.jobs_seen === 1)) throw new Error('Native company search did not return the indexed fixture employer');"}
      ${mode === "fresh" ? "" : "const saved = await invoke('get_job', {id:'saved'}); if (saved.notes !== 'Fixture saved note' || !saved.favorited || saved.status !== (sessionStorage.getItem('kanban-reloaded') ? 'offer' : 'applied')) throw new Error('Saved state changed');"}
      const schedule = ${process.env.SCRAPE_NATIVE_SCHEDULE === "1" ? `await (${checkNativeSchedule.toString()})()` : "null"};
      const appearance = ${process.env.SCRAPE_NATIVE_APPEARANCE === "1" ? `await (${checkNativeAppearance.toString()})()` : "null"};
      const workflow = ${process.env.SCRAPE_NATIVE_WORKFLOW === "1" ? `await (${checkNativeWorkflow.toString()})(${JSON.stringify(workflowOrigin)})` : "null"};
      const kanban = ${process.env.SCRAPE_NATIVE_KANBAN === "1" ? `await (${checkNativeKanban.toString()})()` : "null"};
      const layout = ${process.env.SCRAPE_NATIVE_LAYOUT === "1" ? `await (${checkNativeJobLayout.toString()})()` : "null"};
      const dataManagement = ${process.env.SCRAPE_NATIVE_DATA_MANAGEMENT === "1" ? `await (${checkNativeDataManagement.toString()})(${JSON.stringify(workflowOrigin)})` : "null"};
      await invoke("plugin:event|emit", {event:"native-smoke-result",payload:{ok:true,schedule,appearance,layout,kanban,workflow,dataManagement,steps:["actual Tauri dev readiness: ${mode}"]}});
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
    const expectedVersion = mode === "unknown30" ? 30 : mode === "unknown23" ? 23 : mode === "unknown19" ? 19 : 12;
    if (invalid || code === 0 || !output.includes("STARTUP_FAILED:") || !output.includes(`version ${expectedVersion}`) || hash(await readFile(database)) !== before)
      throw new Error(`Unknown history did not fail safely: ${directory}\n${output}`);
    if ((await readdir(directory)).some(name => name.includes("pre-migration"))) throw new Error("Rejected history was backed up/mutated");
  } else if (invalid || code !== 0 || !output.includes('"ok":true')) {
    throw new Error(`Native startup failed for ${mode}: ${directory}\n${output}`);
  }
  if (process.env.SCRAPE_NATIVE_DATA_MANAGEMENT === "1") execFileSync("python3", ["-c", String.raw`
import pathlib,sqlite3,sys
root=pathlib.Path(sys.argv[1])
def audit(path):
 c=sqlite3.connect(path)
 assert c.execute('PRAGMA integrity_check').fetchone()[0]=='ok'
 assert not c.execute('PRAGMA foreign_key_check').fetchall()
 history=c.execute('SELECT version,description,success,hex(checksum) FROM _sqlx_migrations ORDER BY version').fetchall()
 counts={name:c.execute('SELECT COUNT(*) FROM '+name).fetchone()[0] for (name,) in c.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'").fetchall()}
 c.close(); return history,counts
history,counts=audit(root/'scrape_data.db')
assert [row[0] for row in history]==sorted(int(path.name.split("_")[0]) for path in pathlib.Path("src-tauri/migrations").glob("*.sql"))
assert all(n==0 for table,n in counts.items() if table not in ('_sqlx_migrations','app_settings'))
backups=list((root/'backups').glob('*.db')); assert len(backups)==4
saved=[]
for path in backups:
 previous,n=audit(path); assert previous==history; saved.append(n['jobs'])
assert sorted(saved)==[0,1,1,1]
`, directory]);
  console.log(`PASS actual Tauri dev startup ${mode}; artifacts ${directory}`);
}

} finally { if (workflowServer) { workflowServer.closeAllConnections(); await new Promise(resolve => workflowServer.close(resolve)); } }
