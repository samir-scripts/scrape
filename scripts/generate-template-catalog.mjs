// Rust owns the templates. This projects validated data; never edit outputs.
import { execFileSync } from "node:child_process";
import { readFileSync, readdirSync, writeFileSync, unlinkSync } from "node:fs";
import { stringify } from "yaml";
const check = process.argv.includes("--check");
const json = execFileSync("cargo", ["run", "--quiet", "-p", "scrape", "--example", "export_template_catalog"], { encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
const catalog = JSON.parse(json);
const outputs = new Map([["src/generated/workflowTemplates.json", json]]);
for (const template of catalog.templates) {
  outputs.set(`workflows/${template.id}.yaml`, `# Generated from Rust template catalog v${catalog.version}. Do not edit.\n# ${template.description}\n${stringify(template.spec, { lineWidth: 0 })}`);
}
let stale = false;
// User-authored YAML is never catalog-owned merely because of its filename.
for (const name of readdirSync("workflows")) {
  if (!name.endsWith(".yaml") && !name.endsWith(".yml")) continue;
  const path = `workflows/${name}`;
  if (outputs.has(path)) continue;
  const firstLine = readFileSync(path, "utf8").split(/\r?\n/, 1)[0];
  if (!/^# Generated from Rust template catalog v\d+\. Do not edit\.$/.test(firstLine)) continue;
  if (check) { console.error(`Stale catalog-owned workflow: ${path}`); stale = true; }
  else unlinkSync(path);
}
for (const [path, content] of outputs) {
  if (check) {
    let current;
    try { current = readFileSync(path, "utf8"); }
    catch (error) { if (error.code !== "ENOENT") throw error; }
    if (current !== content) { console.error(`Stale template projection: ${path}`); stale = true; }
  } else writeFileSync(path, content);
}
if (stale) process.exit(1);
