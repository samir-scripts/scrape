import { test as base, expect } from "@playwright/test";

// Required browser verification is offline apart from its local Vite server.
// This also prevents external font downloads from changing screenshot baselines.
export const test = base.extend({
  page: async ({ page }, use) => {
    await page.route("**/*", route => {
      const host = new URL(route.request().url()).hostname;
      return host === "127.0.0.1" || host === "localhost"
        ? route.continue()
        : route.abort();
    });
    await use(page);
  },
});
export { expect };
