import { test, expect } from "./fixtures";

test("workflow role and company selectors preserve literal criteria and keep preview/save explicit", async ({ page }) => {
  await page.goto("/?demo=1&fixtures=1");
  // Chromium has no native SQLite/IPC. This explicit suggestion fixture covers
  // presentation/selection; native startup and Rust tests cover the real query.
  await page.evaluate(async (modulePath) => {
    const { api } = await import(/* @vite-ignore */ modulePath);
    api.searchRoleTitles = async (query: string) => query.toLowerCase().startsWith("software")
      ? [{ title: "Software Engineer", soc_code: "15-1252", priority_tier: 2 }]
      : [];
  }, "/src/lib/api.ts");
  await page.getByRole("button", { name: "WORKFLOWS", exact: true }).click();
  await page.getByRole("button", { name: "Add workflow", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Create workflow" });
  await dialog.getByRole("textbox", { name: "Workflow name", exact: true }).fill("selection browser fixture");

  const roles = dialog.getByRole("combobox", { name: "Target roles", exact: true });
  await roles.fill("software eng");
  await dialog.getByRole("option", { name: /^Software Engineer\b/ }).click();
  await expect(dialog.getByRole("button", { name: "Remove Software Engineer from Target roles", exact: true })).toBeVisible();
  await roles.fill("Cloud Platform Generalist");
  await dialog.getByRole("option", { name: "Use “Cloud Platform Generalist”", exact: true }).click();

  const companies = dialog.getByRole("combobox", { name: "Companies (Optional)", exact: true });
  for (const company of ["Cloudflare", "Linear"]) {
    await companies.fill(company);
    await dialog.getByRole("option", { name: new RegExp(`^${company}\\b`) }).click();
  }
  await companies.fill("Independent Fictional Employer");
  await dialog.getByRole("option", { name: "Use “Independent Fictional Employer”", exact: true }).click();
  await expect(dialog.getByRole("button", { name: "Remove Cloudflare from Companies (Optional)", exact: true })).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Remove Linear from Companies (Optional)", exact: true })).toBeVisible();
  await expect(dialog.getByText(/public-feed coverage is not guaranteed/)).toBeVisible();

  await dialog.getByRole("button", { name: "Continue to Criteria" }).click();
  await dialog.getByRole("button", { name: "Continue to Sources" }).click();
  await expect(dialog.getByText("Sources (3)", { exact: true })).toBeVisible();
  await expect(dialog.getByText("✓ Cross-Company Search", { exact: true })).toBeVisible();
  await dialog.getByRole("button", { name: "Continue to Schedule" }).click();
  await dialog.getByRole("button", { name: "Continue to Review" }).click();
  await expect(dialog.getByRole("button", { name: "Preview matches" })).toBeVisible();
  await dialog.getByText("All configured rules", { exact: true }).click();
  const spec = await dialog.locator("details pre").textContent();
  expect(spec).toContain("Software Engineer");
  expect(spec).toContain("Cloud Platform Generalist");
  expect(spec).toContain("Independent Fictional Employer");
  await dialog.getByRole("navigation", { name: "Workflow steps" }).getByRole("button", { name: /\bRoles\b/ }).click();
  for (const role of ["Software Engineer", "Cloud Platform Generalist"]) {
    await dialog.getByRole("button", { name: `Remove ${role} from Target roles`, exact: true }).click();
  }
  for (const company of ["Cloudflare", "Linear", "Independent Fictional Employer"]) {
    await dialog.getByRole("button", { name: `Remove ${company} from Companies (Optional)`, exact: true }).click();
  }
  const steps = dialog.getByRole("navigation", { name: "Workflow steps" });
  await steps.getByRole("button", { name: /Criteria/ }).click();
  await dialog.getByRole("button", { name: "Date posted range", exact: true }).click();
  await dialog.getByRole("option", { name: "Any time", exact: true }).click();
  await steps.getByRole("button", { name: /Schedule/ }).click();
  await steps.getByRole("button", { name: /Review/ }).click();
  await expect(dialog.getByRole("button", { name: "Save workflow" })).toBeDisabled();
  await dialog.getByRole("button", { name: "Preview matches" }).click();
  await expect(dialog.getByRole("button", { name: "Save workflow" })).toBeEnabled();
  await dialog.getByRole("button", { name: "Save workflow" }).click();
  await expect(dialog.getByText(/Saving has not started a scrape/)).toBeVisible();
  await dialog.getByRole("button", { name: "Done" }).click();
  const row = page.getByRole("row").filter({ hasText: "selection browser fixture" });
  await expect(row).toBeVisible();
  await row.getByRole("button", { name: "Edit workflow selection browser fixture", exact: true }).click();
  const edit = page.getByRole("dialog", { name: "Edit workflow" });
  await expect(edit.getByRole("textbox", { name: "Workflow name", exact: true })).toHaveValue("selection browser fixture");
  await expect(edit.getByRole("button", { name: /Remove .* from Target roles/ })).toHaveCount(0);
  await expect(edit.getByRole("button", { name: /Remove .* from Companies \(Optional\)/ })).toHaveCount(0);
  await edit.getByRole("button", { name: "Continue to Criteria" }).click();
  await edit.getByRole("button", { name: "Continue to Sources" }).click();
  await expect(edit.getByText("Sources (3)", { exact: true })).toBeVisible();
  await edit.getByRole("button", { name: "Cancel", exact: true }).click();
});
