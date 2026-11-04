import { expect, type Page } from "@playwright/test";

/** Expect the library to hold `count` saved runs, read from the History drawer (opened and closed
 * again, retried until the count settles). */
export async function expectSavedRuns(page: Page, count: number) {
  await expect(async () => {
    await page.getByRole("button", { name: "History", exact: true }).click();
    const history = page.getByRole("dialog", { name: "History", exact: true });
    try {
      await expect(history.getByRole("list", { name: "Saved runs" }).getByRole("listitem")).toHaveCount(count, { timeout: 1000 });
    } finally {
      await history.getByRole("button", { name: "Close", exact: true }).click();
    }
  }).toPass({ timeout: 15000 });
}
