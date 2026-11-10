import { test, expect } from "./tool-host";

test.describe("installable models", () => {
  test.use({ hostScenario: "installables" });

  test("Cancel stops an install, and the outcome is shown after the page reopens", async ({ page, host }) => {
    await host.open(page);
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const drawer = page.getByRole("dialog", { name: "Tool setup", exact: true });
    const large = drawer.getByRole("region", { name: "Installables" }).getByRole("listitem", { name: "Large model" });
    await large.getByRole("button", { name: "Get", exact: true }).click();
    await large.getByRole("button", { name: "Get Large model" }).click();
    await expect(large).toContainText("Installing");
    // The page goes away mid-install; the tool keeps the install going.
    await page.reload();
    await page.getByRole("button", { name: "Tool setup", exact: true }).click();
    const again = page.getByRole("dialog", { name: "Tool setup", exact: true })
      .getByRole("region", { name: "Installables" }).getByRole("listitem", { name: "Large model" });
    await expect(again).toContainText("Installing");
    await again.getByRole("button", { name: "Cancel" }).click();
    await expect(again).toContainText("Not installed");
    await expect(again).toContainText("The last install was cancelled.");
  });
});
