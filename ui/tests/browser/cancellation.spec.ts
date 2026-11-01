import { test, expect } from "./tool-host";
test.use({ hostScenario: "unassociated" });
test("cancelled calls without a structured run identity stay unconfirmed", async ({
  page,
  host,
}) => {
  await host.open(page);
  await page
    .locator("input[type=file]")
    .setInputFiles("tests/fixtures/sample.png");
  await page.getByLabel("Label *", { exact: true }).fill("Cancel this attempt");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "running" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Cancel run" }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Cancellation requested · outcome unconfirmed" }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Run", exact: true }),
  ).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "Cancel run" }),
  ).not.toBeVisible();
  await page.getByRole("button", { name: "Inspect run" }).click();
  await expect(page.getByRole("dialog")).toContainText(
    "final outcome unconfirmed",
  );
});
