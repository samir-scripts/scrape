import { defineConfig } from "@playwright/test";
const port = Number(process.env.SCRAPE_BROWSER_PORT || 1420);
if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error("Invalid SCRAPE_BROWSER_PORT");
const baseURL = `http://127.0.0.1:${port}`;
export default defineConfig({
  testDir: "./e2e",
  testMatch: "**/*.pw.ts",
  fullyParallel: false,
  use: {
    baseURL,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    timezoneId: "America/New_York",
  },
  webServer: {
    command: `npm run dev -- --host 127.0.0.1 --port ${port} --strictPort`,
    url: baseURL,
    reuseExistingServer: !process.env.CI,
  },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }],
});
