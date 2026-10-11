#!/usr/bin/env node
import { createServer } from "vite";

import fs from "fs";
import path from "path";

async function main() {
  console.log("[Smoke Test] Initializing Vite development server pipeline...");
  const server = await createServer({
    logLevel: "warn",
  });

  try {
    const rawHtml = fs.readFileSync(path.resolve(process.cwd(), "index.html"), "utf-8");
    const transformedHtml = await server.transformIndexHtml("/", rawHtml);

    if (!transformedHtml.includes('<div id="root"></div>')) {
      throw new Error("Transformed HTML missing root container '<div id=\"root\"></div>'");
    }

    if (!transformedHtml.includes("@vite/client")) {
      throw new Error("Transformed HTML missing Vite HMR client injection");
    }

    if (!transformedHtml.includes('src="/src/main.tsx"')) {
      throw new Error("Transformed HTML missing entrypoint script reference");
    }

    console.log("✓ Frontend development server initializes cleanly and transforms HTML entrypoint without errors.");
  } finally {
    await server.close();
    console.log("[Smoke Test] Server shut down cleanly.");
  }
}

main().catch((err) => {
  console.error("✗ Frontend server startup smoke test failed:", err);
  process.exit(1);
});
