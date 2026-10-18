// Rust owns the templates. This projects validated data; never edit outputs.
import { execFileSync } from "node:child_process";
import { readFileSync, writeFileSync } from "node:fs";
import { stringify } from "yaml";
const check = process.argv.includes("--check");
const json = execFileSync("cargo", ["run", "--quiet", "-p", "scrape", "--example", "export_template_catalog"], { encoding: "utf8", maxBuffer: 4 * 1024 * 1024 });
const catalog = JSON.parse(json);
const outputs = new Map([["src/generated/workflowTemplates.json", json]]);
for (const template of catalog.templates) {
  outputs.set(`workflows/${template.id}.yaml`, `# Generated from Rust template catalog v${catalog.version}. Do not edit.\n# ${template.description}\n${stringify(template.spec, { lineWidth: 0 })}`);
}
let stale = false;
for (const [path, content] of outputs) {
  if (check) {
    if (readFileSync(path, "utf8") !== content) { console.error(`Stale template projection: ${path}`); stale = true; }
  } else writeFileSync(path, content);
}
if (stale) process.exit(1);
