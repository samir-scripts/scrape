import path from "node:path";

export function resolveCargoTargetDirectory(metadata) {
  const targetDirectory = metadata?.target_directory;
  if (typeof targetDirectory !== "string" || targetDirectory.length === 0) {
    throw new Error("Cargo metadata did not include a target_directory");
  }
  return path.resolve(targetDirectory);
}
