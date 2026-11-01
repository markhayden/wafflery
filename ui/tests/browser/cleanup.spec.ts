import { test, expect } from "./tool-host";
test("deleting a source run clears its baseline and preserves a saved example", async ({
  page,
  host,
}) => {
  await host.open(page);
  await page
    .locator("input[type=file]")
    .setInputFiles("tests/fixtures/sample.png");
  await page.getByLabel("Label *", { exact: true }).fill("Keep these inputs");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await page.getByRole("button", { name: "Pin as baseline" }).click();
  await expect(page.getByText("This run is the baseline")).toBeVisible();
  await page.getByRole("button", { name: "Examples", exact: true }).click();
  const examples = page.getByRole("dialog", { name: "Examples", exact: true });
  await examples
    .getByLabel("Example name", { exact: true })
    .fill("Survives cleanup");
  await examples
    .getByRole("button", { name: "Save example", exact: true })
    .click();
  await expect(
    examples.getByRole("button", { name: "Open Survives cleanup" }),
  ).toBeVisible();
  await examples.getByRole("button", { name: "Close", exact: true }).click();
  await page.route("**/api/attempts?limit=20*", async (route) => {
    const url = new URL(route.request().url());
    if (url.searchParams.has("cursor")) {
      await route.continue();
      return;
    }
    const response = await route.fetch();
    const value = await response.json();
    if (!value.items.length) {
      await route.fulfill({ response });
      return;
    }
    await route.fulfill({
      json: {
        items: [
          {
            ...value.items[0],
            id: "newer-placeholder",
            sequence: value.items[0].sequence + 1,
          },
        ],
        cursor: value.items[0].sequence + 1,
      },
    });
  });
  await page
    .getByRole("button", { name: "Clean up library", exact: true })
    .click();
  const cleanup = page.getByRole("dialog", {
    name: "Clean up library",
    exact: true,
  });
  await cleanup.getByRole("button", { name: "Load older runs" }).click();
  await expect(cleanup.getByLabel(/Delete run/)).toHaveCount(2);
  await cleanup
    .getByLabel(/Delete run/)
    .last()
    .check();
  await cleanup.getByRole("button", { name: "Preview cleanup" }).click();
  await expect(cleanup.getByLabel("Cleanup preview")).toContainText(
    "1 baseline pins will clear",
  );
  await cleanup.getByRole("button", { name: "Confirm cleanup" }).click();
  await expect(cleanup.getByRole("status")).toContainText("Cleanup complete");
  await cleanup.getByRole("button", { name: "Close", exact: true }).click();
  await page.getByRole("button", { name: "Examples", exact: true }).click();
  await examples.getByRole("button", { name: "Open Survives cleanup" }).click();
  await expect(page.getByLabel("Label *", { exact: true })).toHaveValue(
    "Keep these inputs",
  );
  await expect(
    page.getByRole("img", { name: "Selected image 1" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Pin as baseline" }),
  ).toBeVisible();
});
