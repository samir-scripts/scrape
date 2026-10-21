// Plan 12 §7: Authoritative provider verification CLI.
// Delegates to the Rust workflow verification driver (verify_provider_workflows).
import { spawn } from "node:child_process";
import { writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";

const live = process.argv.includes("--live");
const refresh = process.argv.includes("--refresh-example");

if (live && refresh) {
  console.error("Error: --live and --refresh-example are mutually exclusive.");
  process.exit(1);
}

const executable = `src-tauri/binaries/scrape-worker-${process.arch === "arm64" ? "aarch64" : "x86_64"}-${process.platform === "darwin" ? "apple-darwin" : "unknown-linux-gnu"}`;
if (!existsSync(executable)) {
  console.error("Error: Build the packaged worker with npm run build:sidecar before provider verification.");
  process.exit(1);
}

const args = ["run", "--quiet", "-p", "scrape", "--example", "verify_provider_workflows", "--"];
if (live) args.push("--live");
if (refresh) args.push("--refresh-example");

const env = {
  ...process.env,
  SCRAPE_VERIFY_NODE: process.execPath,
};

const child = spawn("cargo", args, {
  env,
  stdio: ["inherit", "pipe", "inherit"],
});

let stdout = "";
child.stdout.on("data", (chunk) => {
  stdout += chunk;
});

child.on("close", async (code) => {
  if (!stdout.trim()) {
    process.exit(code || 1);
  }
  try {
    const report = JSON.parse(stdout.trim());
    if (refresh && report.recorded_example) {
      await writeFile(
        "src/lib/recordedExample.json",
        JSON.stringify(report.recorded_example, null, 2) + "\n"
      );
    }
    console.log(JSON.stringify(report, null, 2));
    const failed = !Array.isArray(report.checks) || report.checks.length === 0 || report.checks.some((c) => c.status === "failed");
    process.exit(failed ? 1 : (code || 0));
  } catch (err) {
    console.error("Failed to parse verification report JSON:", err.message);
    console.log(stdout);
    process.exit(1);
  }
});
