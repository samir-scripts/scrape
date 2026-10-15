import assert from "node:assert/strict";
import { resolveCargoTargetDirectory } from "./cargo-target-dir.mjs";

assert.equal(resolveCargoTargetDirectory({ target_directory: "/tmp/custom-target" }), "/tmp/custom-target");
assert.equal(resolveCargoTargetDirectory({ target_directory: "relative-target" }), `${process.cwd()}/relative-target`);
assert.throws(() => resolveCargoTargetDirectory({}), /target_directory/);
console.log("Cargo target directory resolution passed.");
