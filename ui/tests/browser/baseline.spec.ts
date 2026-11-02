import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";
test("baseline comparison uses two saved runs and survives restart", async ({
  page,
  host,
}) => {
  await host.open(page);
  await prepareSampleInput(page, "Baseline experiment");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Pin as baseline" }).click();
  await expect(page.getByText("This run is the baseline")).toBeVisible();
  await page.getByLabel("Label *", { exact: true }).fill("Second experiment");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "Compare with baseline" }),
  ).toBeVisible();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await page
    .getByLabel("Label *", { exact: true })
    .fill("Unsubmitted settings");
  await page.getByRole("button", { name: "Compare with baseline" }).click();
  await page.getByText("Compare saved settings", { exact: true }).click();
  await expect(page.locator(".settings-comparison")).toContainText(
    "Baseline experiment",
  );
  await expect(page.locator(".settings-comparison")).toContainText(
    "Second experiment",
  );
  await expect(page.locator(".settings-comparison")).not.toContainText(
    "Unsubmitted settings",
  );
  await host.stop();
  await host.start();
  await host.open(page);
  await expect(
    page.getByRole("button", { name: "Compare with baseline" }),
  ).toBeVisible();
});
