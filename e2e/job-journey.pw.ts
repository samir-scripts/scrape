import { moveByPointer } from "./kanbanPointer";
import { contrastFailures } from "./contrast";
import { test, expect } from "./fixtures";
import { discoverFixtures } from "./jobFixtures";
for (const theme of ["paper", "obsidian"])
  for (const width of [1440, 1024, 480]) {
    test(`${theme} ${width}: discovered to applied list, board and shared inspector`, async ({
      page,
    }) => {
      await page.clock.setFixedTime(new Date("2026-10-14T03:00:00Z"));
      await page.emulateMedia({ reducedMotion: "reduce" });
      await page.setViewportSize({ width, height: 900 });
      await discoverFixtures(page);
      if (theme === "obsidian") {
        await page.getByRole("button", { name: "SETTINGS", exact: true }).click();
        await page.getByRole("button", { name: /^OBSIDIAN/ }).click();
        await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
        await page.getByRole("button", { name: /^DISCOVERED/ }).click();
      }
      const title = "Software Engineer Intern (2027) - Austin, TX";
      await page
        .getByRole("button", { name: `View ${title} at Demo Company` })
        .click();
      const inspector =
        width < 812
          ? page.getByRole("dialog", { name: "Job details" })
          : page.getByRole("complementary", { name: "Job details" });
      await expect(
        inspector.getByRole("heading", { name: title }),
      ).toBeVisible();
      const remove = inspector.getByRole("button", {
        name: `Remove ${title} at Demo Company`,
      });
      expect(await remove.textContent()).toBe("");
      expect(await contrastFailures(page)).toEqual([]);
      await expect.soft(page).toHaveScreenshot(`${theme}-${width}-inspector.png`, {
        animations: "disabled",
      });
      await inspector.getByLabel("Notes").fill("Follow up with recruiter");
      await expect(inspector.getByRole("status")).toHaveText("Saved");
      await inspector
        .getByRole("button", { name: "Mark as applied", exact: true })
        .click();
      await expect(
        page.getByRole("button", { name: "View in Tracked", exact: true }),
      ).toBeVisible();
      await page
        .getByRole("button", { name: "View in Tracked", exact: true })
        .click();
      await page.getByRole("button", { name: /LIST/ }).click();
      expect(await page.locator("thead th").count()).toBe(6);
      expect(await contrastFailures(page)).toEqual([]);
      await expect.soft(page).toHaveScreenshot(
        `${theme}-${width}-applied-list.png`,
        { animations: "disabled" },
      );
      await page.getByRole("button", { name: /KANBAN/ }).click();
      expect(await contrastFailures(page)).toEqual([]);
      await expect.soft(page).toHaveScreenshot(`${theme}-${width}-board.png`, {
        animations: "disabled",
      });
      await page
        .getByRole("button", { name: `View ${title} at Demo Company` })
        .click();
      await expect(inspector.getByLabel("Notes")).toHaveValue(
        "Follow up with recruiter",
      );
      await inspector
        .getByLabel("Application stage")
        .selectOption("interviewing");
      await expect(inspector.getByLabel("Application stage")).toHaveValue(
        "interviewing",
      );
      await page.keyboard.press("Escape");
      if (width < 768)
        await page.getByLabel("Board stage").selectOption("interviewing");
      await expect(
        page
          .getByRole("region", { name: "INTERVIEWING stage" })
          .getByRole("button", { name: `View ${title} at Demo Company` }),
      ).toBeVisible();
      if (width === 1440 && theme === "paper") {
        await moveByPointer(page, title, "offer");
        const moved = page.getByRole("region", {name:"OFFER stage"}).locator("article");
        await expect(moved).toBeVisible(); await expect(moved).not.toHaveClass(/border-dashed/);
      }
    });
  }
test("normal browser examples contain recorded provider jobs and cannot simulate a scrape", async ({
  page,
}) => {
  await page.goto("/?demo=1");
  await expect(page.getByText(/RECORDED EXAMPLES/)).toBeVisible();
  await expect(
    page.getByRole("button", { name: /^View .* at SEP/ }).first(),
  ).toBeVisible();
  await expect(page.getByText(/FICTIONAL DEMONSTRATION POSTING/)).toHaveCount(
    0,
  );
  await page
    .getByRole("button", { name: /^View .* at SEP/ })
    .first()
    .click();
  await page.getByText("Source evidence", { exact: true }).click();
  await expect(page.getByText(/https:\/\/jobs.lever.co\/sep\//)).toBeVisible();
});
