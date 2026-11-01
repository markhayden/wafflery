import { test, expect } from "./tool-host";

test("setup shows reported allowances and never promotes a credential name to readiness", async ({ page }) => {
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--capped&viewMode=story");
  await expect(page.getByRole("region", { name: "Session spending" })).toContainText("$1.80");
  await expect(page.getByText(/final charge is unknown/)).toContainText("$0.04");
  await expect(page.getByText("EXAMPLE_API_KEY", { exact: true })).toBeVisible();
  await expect(page.getByText(/availability and validity have not been checked/)).toBeVisible();
  await expect(page.getByText(/Paid execution needs/)).toContainText("not enabled yet");
  for (const width of [320, 768, 1024, 1440]) {
    await page.setViewportSize({ width, height: 900 });
    await expect(page.getByRole("button", { name: "Refresh" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--exhausted&viewMode=story");
  await expect(page.getByRole("region", { name: "Session spending" })).toContainText("$0.00");
  await page.goto("http://127.0.0.1:6006/iframe.html?id=workspace-toolsetup--unavailable&viewMode=story");
  await expect(page.getByRole("alert")).toContainText("disconnected");
  await expect(page.getByRole("region", { name: "Session spending" })).not.toContainText("$0.00");
});
