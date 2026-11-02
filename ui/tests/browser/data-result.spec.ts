import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";

test.use({ hostScenario: "data-only" });
test("a data-only result shows saved data without an empty image comparison", async ({
  page,
  host,
}) => {
  await host.open(page);
  await prepareSampleInput(page, "Data result");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await expect(page.getByLabel("Result data", { exact: true })).toContainText(
    "Illustrative data-only result",
  );
  await expect(
    page.getByRole("region", { name: "Image comparison", exact: true }),
  ).toHaveCount(0);
  await page.getByRole("button", { name: "Inspect run" }).click();
  await page
    .getByRole("dialog")
    .getByRole("button", { name: "Raw result" })
    .click();
  await expect(page.getByLabel("Saved evidence")).toContainText(
    "Illustrative data-only result",
  );
});
