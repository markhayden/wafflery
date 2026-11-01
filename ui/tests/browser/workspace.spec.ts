import { test, expect } from "./tool-host";
test("run a real offline action and inspect immutable evidence after editing the draft", async ({
  page,
  host,
}, testInfo) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await host.open(page);
  await expect(page.locator(".workspace-brand img")).toBeVisible();
  await expect.poll(() => page.locator(".workspace-brand img").evaluate(
    (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
  )).toBe(true);
  await expect(
    page.getByRole("button", { name: /Action Add a test band/ }),
  ).toBeVisible();
  await page
    .locator("input[type=file]")
    .setInputFiles("tests/fixtures/sample.png");
  await page
    .getByLabel("Label *", { exact: true })
    .fill("Saved first experiment");
  await page.getByRole("button", { name: "Run", exact: true }).click();
  await expect(
    page.getByText("Contract evidence: complete.", { exact: false }).first(),
  ).toBeVisible();
  await expect(
    page.getByRole("slider", { name: "Before and after divider" }),
  ).toBeVisible();
  await page.getByLabel("Label *", { exact: true }).fill("Unsubmitted draft");
  await page.getByRole("button", { name: "Inspect run" }).click();
  const drawer = page.getByRole("dialog", { name: "Run details" });
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "Saved first experiment",
  );
  await expect(drawer.getByLabel("Saved evidence")).not.toContainText(
    "Unsubmitted draft",
  );
  await drawer.getByRole("button", { name: "Recorded call" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "progressToken",
  );
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "Saved first experiment",
  );
  await drawer.getByRole("button", { name: "Raw result" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "structuredContent",
  );
  await drawer.getByRole("button", { name: "Close" }).click();
  await page.screenshot({
    path: testInfo.outputPath("offline-result.png"),
    fullPage: true,
  });
  await host.stop();
  await host.start();
  await host.open(page);
  await expect(
    page.getByRole("slider", { name: "Before and after divider" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Inspect run" }).click();
  await expect(drawer.getByLabel("Saved evidence")).toContainText(
    "Saved first experiment",
  );
  await drawer.getByRole("button", { name: "Close" }).click();
  await page.getByRole("button", { name: "History", exact: true }).click();
  const history = page.getByRole("dialog", { name: "History", exact: true });
  await history.getByRole("button", { name: /sample \/ mark_band/ }).click();
  await expect(page.getByRole("slider")).toBeVisible();
  expect(errors).toEqual([]);
});

test("tool discovery does not expose a temporary image form that loses uploads", async ({
  page,
  host,
}) => {
  let requests = 0;
  let release: () => void = () => {};
  const ready = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/state", async (route) => {
    requests++;
    if (requests === 2) await ready;
    await route.continue();
  });
  try {
    await host.open(page);
    await expect(
      page.getByRole("status").filter({ hasText: "Connected locally" }),
    ).toBeVisible();
    await expect(page.getByText("Loading workspace…")).toBeVisible();
    await expect(page.locator("input[type=file]")).toHaveCount(0);
  } finally {
    release();
  }
  await expect(
    page.getByRole("button", { name: /Action Add a test band/ }),
  ).toBeVisible();
  await expect(page.locator("input[type=file]")).toHaveCount(1);
});
