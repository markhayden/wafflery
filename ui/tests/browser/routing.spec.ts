import { prepareSampleInput } from "./sample-input";
import { test, expect } from "./tool-host";

// The workspace's address is where it is (route.ts): `/<tool>/<action>/<run>`.
test("a tool alone is the empty state, and a reload never opens an old run", async ({ page, host }) => {
  await host.open(page);
  await prepareSampleInput(page, "Routed run");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
  await expect(page).toHaveURL(/\/sample\/mark_band\/[0-9a-f-]{36}$/);

  await host.open(page, "/sample");
  await expect(page.getByText("Choose what you want to do from the Action menu above.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Run", exact: true })).toHaveCount(0);
  await expect(page.getByText("Contract evidence", { exact: false })).toHaveCount(0);

  await host.open(page, "/sample/mark_band");
  await expect(page.getByRole("button", { name: "Action Add a test band", exact: true })).toBeVisible();
  await expect(page.getByText("Contract evidence", { exact: false })).toHaveCount(0);
});

test("a run's address reopens that run with its draft, and Back returns to the place before", async ({ page, host }) => {
  await host.open(page);
  await prepareSampleInput(page, "Linked run");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
  const address = new URL(page.url()).pathname;

  await host.open(page, "/sample");
  await expect(page.getByText("Choose what you want to do from the Action menu above.")).toBeVisible();
  await host.open(page, address);
  await expect(page.getByText("Contract evidence: complete.", { exact: false }).first()).toBeVisible();
  await expect(page.getByLabel("Label", { exact: true })).toHaveValue("Linked run");
  await expect(page).toHaveURL(new RegExp(`${address}$`));

  await page.goBack();
  await expect(page).toHaveURL(/\/sample$/);
  await expect(page.getByText("Choose what you want to do from the Action menu above.")).toBeVisible();
});
